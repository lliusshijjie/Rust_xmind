# Rust 异步编程：Future、Waker、Executor 与 Tokio 架构总结

## 1. 整体认识

在 Rust 异步编程中，可以先建立如下核心认识：

> **Future 描述异步任务当前执行到哪里，Executor 负责推动 Future 执行，Waker 负责在条件满足后通知 Executor 再次调度该任务。**

三者形成如下闭环：

```text
Executor poll Future
        ↓
Future 暂时无法继续，返回 Pending
        ↓
Future 注册并保存 Waker
        ↓
等待 socket、定时器或 Channel 等事件
        ↓
事件满足后调用 Waker::wake()
        ↓
任务重新进入 Executor 的就绪队列
        ↓
Executor 再次 poll Future
```

从 C++ 服务端角度看，可以粗略对应为：

| Rust / Tokio | C++ 服务端中的近似概念 |
|---|---|
| Future | C++20 协程状态机 / 手写连接状态机 |
| `.await` | `co_await` / 协程挂起点 |
| Waker | 将协程或连接重新放回 ready queue 的通知机制 |
| Executor | 协程调度器 / 任务调度线程池 |
| Tokio Reactor | `epoll`、`kqueue`、IOCP 等 I/O 事件驱动层 |
| Tokio Task | 被调度的异步任务 |
| `tokio::spawn` | 向调度器提交协程任务 |

---

## 2. Future Trait

### 2.1 Future 是什么

`Future` 是 Rust 异步编程的核心 Trait：

```rust
pub trait Future {
    type Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output>;
}
```

其中：

```rust
pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

含义分别是：

- `Poll::Ready(value)`：异步任务已经完成，返回结果；
- `Poll::Pending`：当前暂时无法继续，需要等待外部事件。

因此，可以把 Future 理解成：

> **一个可被反复推进、可暂停、可恢复的异步状态机。**

---

### 2.2 `async fn` 会生成 Future

例如：

```rust
async fn fetch_data() -> String {
    "data".to_string()
}
```

从类型角度看，大致等价于：

```rust
fn fetch_data() -> impl Future<Output = String>
```

调用异步函数：

```rust
let future = fetch_data();
```

此时只是创建了一个 Future，通常还没有真正执行函数体。

只有 Future 被 Executor 调用 `poll()`，它才会向前推进。

---

### 2.3 `.await` 是状态机暂停点

例如：

```rust
async fn handle_request() -> Result<String, Error> {
    let request = read_request().await?;
    let user = query_database(request).await?;
    Ok(format!("hello {}", user.name))
}
```

编译器会把它转换为类似下面的状态机：

```text
State 0：开始执行
State 1：等待 read_request
State 2：等待 query_database
State 3：构造结果
State 4：完成
```

概念上类似：

```rust
enum HandleRequestFuture {
    Start,
    ReadingRequest {
        read_future: ReadFuture,
    },
    QueryingDatabase {
        request: Request,
        query_future: QueryFuture,
    },
    Completed,
}
```

因此：

> `async/await` 的重要价值之一，是由编译器自动生成过去在 Reactor 模型中经常需要手写的连接状态机。

---

### 2.4 Future 不会主动运行

Future 本身只保存状态和定义下一步如何执行，它不会：

- 创建线程；
- 主动轮询自己；
- 自己监听 socket；
- 自己决定何时再次运行。

所以需要 Executor 来驱动 Future。

---

## 3. Executor

### 3.1 Executor 的职责

Executor 的核心工作流程是：

```text
从就绪队列中取出 Task
        ↓
调用 Task 内部 Future 的 poll()
        ↓
根据返回结果决定下一步
```

如果返回：

```rust
Poll::Ready(value)
```

说明任务完成，可以回收 Future。

如果返回：

```rust
Poll::Pending
```

说明任务暂时无法继续，Executor 不会一直占着线程等待，而是转去执行其他 Task。

因此：

> **Executor 每次只执行 Future 当前能够执行的一段，而不是保证一个任务从头运行到尾。**

---

### 3.2 Tokio Runtime 中的 Executor

Tokio Runtime 不只是 Executor，它通常包含：

```text
Tokio Runtime
├── Executor / Scheduler
├── Worker Threads
├── I/O Driver / Reactor
├── Timer Driver
└── Blocking Thread Pool
```

例如：

```rust
#[tokio::main]
async fn main() {
    println!("hello");
}
```

可以粗略理解成：

```rust
fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        println!("hello");
    });
}
```

其中 `runtime.block_on(...)` 会驱动最外层 Future 执行。

---

### 3.3 `tokio::spawn` 做了什么

```rust
tokio::spawn(async move {
    handle_connection(stream).await;
});
```

其内部过程可以大致理解为：

```text
async block 被编译成 Future
        ↓
Tokio 将 Future 包装成 Task
        ↓
Task 放入 Scheduler 的就绪队列
        ↓
某个 Worker Thread 取出 Task
        ↓
调用 Future::poll()
```

一个 Tokio Task 可以粗略理解为：

```text
Task
├── Future 状态机
├── 当前调度状态
├── Waker / 唤醒信息
└── 最终结果
```

在 Tokio 多线程 Runtime 中，同一个任务的不同执行阶段可能由不同 Worker 执行：

```text
第一次 poll：Worker 1
第二次 poll：Worker 3
第三次 poll：Worker 2
```

所以 `tokio::spawn` 中的 Future 通常需要满足：

```rust
Send + 'static
```

---

## 4. Waker

### 4.1 为什么需要 Waker

当 Future 返回：

```rust
Poll::Pending
```

Executor 会停止运行当前任务，转而执行其他任务。

问题是：Executor 如何知道这个 Future 什么时候可以再次执行？

不能不断地进行忙轮询：

```text
poll Future A → Pending
poll Future A → Pending
poll Future A → Pending
...
```

这种方式会浪费大量 CPU。

因此，Rust 引入了 Waker：

> **Future 在暂时无法继续时注册 Waker；当外部条件满足后，通过 Waker 通知 Executor 将任务重新加入就绪队列。**

---

### 4.2 Waker 从哪里获得

Future 的 `poll()` 接收：

```rust
cx: &mut Context<'_>
```

通过 Context 可以获得当前任务的 Waker：

```rust
let waker = cx.waker();
```

典型逻辑如下：

```rust
fn poll(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
) -> Poll<Self::Output> {
    if self.result_is_ready() {
        Poll::Ready(self.take_result())
    } else {
        self.register_waker(cx.waker().clone());
        Poll::Pending
    }
}
```

一个重要规则是：

> Future 返回 `Pending` 时，必须确保未来存在某个事件能够调用对应 Waker，否则这个 Future 可能永远不会再次执行。

---

### 4.3 `wake()` 不代表立即执行

调用：

```rust
waker.wake();
```

并不意味着当前线程立即进入 Future 继续执行。

更准确的流程是：

```text
Waker::wake()
        ↓
将 Task 标记为 Runnable
        ↓
Task 放回 Executor 的就绪队列
        ↓
某个 Worker Thread 稍后取出 Task
        ↓
再次调用 Future::poll()
```

因此：

> Waker 是一种调度通知机制，不是直接执行业务回调。

---

## 5. Future、Waker、Executor 的职责边界

| 组件 | 主要职责 | 不负责什么 |
|---|---|---|
| Future | 保存异步任务状态，描述下一步如何推进 | 不主动调度自己 |
| Executor | 调用 `poll()`，执行和调度就绪任务 | 不直接监听 socket 是否就绪 |
| Waker | 把等待中的 Task 重新标记为可运行 | 不直接执行整个 Future |
| Reactor / I/O Driver | 监听 socket、定时器等事件 | 不直接运行业务 Future |

可以用下面的对话帮助理解：

```text
Future：我现在还不能继续，需要等待 socket 可读。
Reactor：我来监听这个 socket。
Waker：socket 可读后，通过我通知调度器。
Executor：收到通知后，我重新 poll 这个 Future。
```

---

## 6. Future 生命周期状态图

一个 Future 从创建到完成，通常经历以下状态：

```text
┌─────────────┐
│   Created   │ Future 已创建，尚未执行
└──────┬──────┘
       │ spawn / block_on / 父 Future 驱动
       ▼
┌─────────────┐
│  Runnable   │ 已进入 Executor 就绪队列
└──────┬──────┘
       │ Worker 调用 poll
       ▼
┌─────────────┐
│   Polling   │ Future 正在执行
└──────┬──────┘
       │
       ├── Poll::Ready(value)
       │           ↓
       │      ┌─────────────┐
       │      │  Completed  │
       │      └─────────────┘
       │
       └── Poll::Pending
                   │ 注册 Waker
                   ▼
            ┌─────────────┐
            │   Waiting   │ 等待 I/O、Timer、Channel
            └──────┬──────┘
                   │ 外部事件满足
                   │ Waker::wake()
                   ▼
               Runnable
```

最核心的循环是：

```text
Runnable
    ↓ poll
Polling
    ↓ Pending
Waiting
    ↓ wake
Runnable
```

---

## 7. Tokio 整体架构图

```text
                         Tokio Runtime
┌─────────────────────────────────────────────────────────┐
│                                                         │
│  ┌──────────────────────┐                               │
│  │ Scheduler / Executor │                               │
│  │                      │                               │
│  │ Runnable Queue       │                               │
│  │ [Task A][Task B]...  │                               │
│  └──────────┬───────────┘                               │
│             │                                           │
│             │ 调度 Task                                 │
│             ▼                                           │
│  ┌──────────────────────┐                               │
│  │ Worker Threads       │                               │
│  │                      │                               │
│  │ poll(Task Future)    │                               │
│  └──────────┬───────────┘                               │
│             │                                           │
│      Pending│                         Ready              │
│             ▼                           │                │
│  ┌──────────────────────┐               │                │
│  │ I/O Driver / Reactor │               ▼                │
│  │                      │         Future 执行完成          │
│  │ Linux：epoll         │                                │
│  │ BSD：kqueue          │                                │
│  │ Windows：IOCP        │                                │
│  └──────────┬───────────┘                                │
│             │                                           │
│             │ socket / timer / channel 就绪              │
│             ▼                                           │
│       Waker::wake()                                     │
│             │                                           │
│             └──────────▶ Task 重新进入 Runnable Queue    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

可以概括为：

```text
Future
    = 业务异步状态机

Executor
    = Future 调度器

Waker
    = Reactor 与 Executor 之间的通知桥梁

Reactor
    = 底层 I/O 事件监听者
```

---

## 8. `TcpStream::read().await` 的完整过程

示例：

```rust
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

async fn handle_connection(
    mut stream: TcpStream,
) -> std::io::Result<()> {
    let mut buffer = [0u8; 1024];

    let n = stream.read(&mut buffer).await?;

    println!("received {n} bytes");
    Ok(())
}
```

### 8.1 第一次 poll

Executor 首次调用：

```text
poll(handle_connection Future)
```

Future 执行到：

```rust
stream.read(&mut buffer).await
```

Tokio 底层尝试进行非阻塞读取。

如果 socket 暂时没有数据，Linux 通常返回：

```text
EAGAIN / EWOULDBLOCK
```

此时读取 Future 会：

```text
1. 将 socket 的可读事件注册到 Reactor
2. 保存当前 Task 的 Waker
3. 返回 Poll::Pending
```

状态变化：

```text
Polling → Waiting
```

Executor 随后去执行其他 Task。

---

### 8.2 socket 数据到达

```text
客户端发送 TCP 数据
        ↓
内核 TCP 协议栈处理
        ↓
socket 接收缓冲区出现数据
        ↓
epoll 返回 EPOLLIN
        ↓
Tokio Reactor 找到对应等待任务
        ↓
调用 Waker::wake()
```

Task 被重新放入就绪队列：

```text
Waiting → Runnable
```

---

### 8.3 第二次 poll

某个 Worker Thread 再次调用：

```text
poll(handle_connection Future)
```

这次非阻塞读取成功，得到 `n` 个字节：

```text
read(socket) → n bytes
```

读取 Future 返回：

```rust
Poll::Ready(Ok(n))
```

外层异步函数从 `.await` 后继续执行：

```rust
println!("received {n} bytes");
```

最终整个任务返回：

```rust
Poll::Ready(Ok(()))
```

任务完成。

---

## 9. 完整时序图

```text
Executor          Future          Tokio Reactor        epoll/socket
    │                │                  │                   │
    │── poll() ─────▶│                  │                   │
    │                │── read() ───────────────────────────▶│
    │                │◀────────────── EAGAIN ───────────────│
    │                │                  │                   │
    │                │── 注册 Waker ───▶│                   │
    │◀── Pending ────│                  │                   │
    │                │                  │                   │
    │ 执行其他任务   │                  │                   │
    │                │                  │                   │
    │                │                  │◀── socket 可读 ───│
    │                │                  │                   │
    │◀──────────── wake Task ───────────│                   │
    │                │                  │                   │
    │ 将 Task 放入   │                  │                   │
    │ Runnable Queue │                  │                   │
    │                │                  │                   │
    │── poll() ─────▶│                  │                   │
    │                │── read() ───────────────────────────▶│
    │                │◀────────────── n bytes ──────────────│
    │                │                  │                   │
    │◀── Ready(n) ───│                  │                   │
```

---

## 10. 在 Tokio 业务代码中的主要使用方式

### 10.1 Future：经常直接使用

通过 `async fn`、`async {}` 和 `.await` 使用：

```rust
async fn query_user() -> User {
    // 异步查询
}

let user = query_user().await;
```

普通业务开发中每天都会使用 Future，但通常不需要手写 `poll()`。

---

### 10.2 Executor：通过 Runtime 和 spawn 使用

常见入口包括：

```rust
#[tokio::main]
```

```rust
runtime.block_on(...)
```

```rust
tokio::spawn(...)
```

```rust
tokio::task::spawn_local(...)
```

例如：

```rust
#[tokio::main]
async fn main() {
    tokio::spawn(async {
        process_request().await;
    });
}
```

普通业务开发中通常不需要自己实现 Executor。

---

### 10.3 Waker：业务层通常不直接使用

以下 Tokio API 内部已经处理了 Waker：

```rust
TcpStream::read().await
TcpListener::accept().await
tokio::time::sleep(...).await
mpsc::Receiver::recv().await
oneshot::Receiver.await
Mutex::lock().await
```

通常只有以下场景才需要直接接触 Waker：

- 手写 Future；
- 实现底层异步库；
- 对接自定义事件源；
- 开发 Runtime 或 Executor；
- 对接 C/C++ 异步回调；
- 编写底层驱动。

---

## 11. 与 C++ Reactor 服务端的对应理解

传统 C++ Reactor 中，可能需要手写连接状态机：

```cpp
void on_readable(Connection& conn) {
    switch (conn.state) {
        case ReadingHeader:
            read_header(conn);
            break;

        case ReadingBody:
            read_body(conn);
            break;

        case WritingResponse:
            write_response(conn);
            break;
    }
}
```

Rust 中可以写成顺序代码：

```rust
async fn handle_connection(
    stream: &mut TcpStream,
) -> Result<(), Error> {
    let header = read_header(stream).await?;
    let body = read_body(stream, header.length).await?;
    let response = process(header, body).await?;
    write_response(stream, response).await?;
    Ok(())
}
```

编译器自动生成状态机，Tokio 负责调度和事件通知。

因此可以理解为：

```text
传统 C++：
事件 → 回调 → 手写状态机 → 手动投递线程池

Rust + Tokio：
事件 → Waker → Executor → Future 状态机恢复
```

---

## 12. 三个最重要的边界

### 12.1 Future 只是状态机，不是线程

```text
Future ≠ 线程
Future ≠ Executor
Future ≠ epoll
```

Future 只描述任务当前状态和下一步如何执行。

---

### 12.2 Waker 只负责通知

```text
wake()
≠ 立即执行 Future

wake()
= 将 Task 重新变成可调度状态
```

---

### 12.3 Executor 不应忙轮询 Pending Future

错误方式：

```text
不停 poll 所有 Pending Future
```

正确方式：

```text
Future 返回 Pending
        ↓
等待外部事件
        ↓
Waker 通知
        ↓
Executor 再次 poll
```

---

## 13. 最终心智模型

```text
                         事件源
                socket / timer / channel
                          │
                          │ 条件满足
                          ▼
Future 返回 Pending ◀── Reactor
       │                  │
       │ 注册 Waker       │ 调用 wake()
       ▼                  ▼
    Waiting          Runnable Queue
                          │
                          │ Executor 调度
                          ▼
                     Worker Thread
                          │
                          │ poll Future
                          ▼
                ┌──────────────────┐
                │ Poll::Pending    │──▶ 再次等待
                │ Poll::Ready      │──▶ 任务完成
                └──────────────────┘
```

可以用一句话总结：

> **Future 是状态，Executor 是动力，Waker 是通知；Reactor 监听外部事件，并通过 Waker 将等待中的 Future 重新交给 Executor。**

