# Rust 多线程与 Tokio 并发总结

## 1. 整体理解

Rust 并发的核心并不只是提供线程 API，而是把跨线程安全纳入：

- 所有权与借用规则
- 生命周期检查
- `Send` / `Sync` Trait
- 类型系统的编译期约束

Rust 中常见的两类并发执行单位：

| 方式 | 执行单位 | 调度者 | 适用场景 |
|---|---|---|---|
| `std::thread` | OS 线程 | 操作系统 | 少量后台线程、阻塞任务、CPU 任务 |
| Tokio | 异步 Task / `Future` | Tokio Runtime | 高并发网络和 I/O 服务 |

工程上的快速判断：

```text
I/O 高并发          -> Tokio
CPU 密集计算        -> Rayon / 专用线程池
少量长期后台任务    -> std::thread
阻塞旧库 / C FFI    -> spawn_blocking 或专用线程
```

---

## 2. `std::thread` 基础用法

### 2.1 创建线程

```rust
use std::thread;

fn main() {
    let handle = thread::spawn(|| {
        println!("child thread");
        42
    });

    let result = handle.join().unwrap();
    println!("result = {result}");
}
```

- `thread::spawn` 创建一个 OS 线程。
- 返回 `JoinHandle<T>`。
- `join()` 阻塞等待线程结束，并取得线程返回值。
- 示例中常写 `unwrap()`；工程代码应明确处理线程 panic。

### 2.2 `thread::scope`

`thread::spawn` 通常要求线程闭包满足 `'static`，因此不能直接借用当前栈变量。

如果线程确定会在当前作用域结束前退出，可以使用 `thread::scope`：

```rust
use std::thread;

fn main() {
    let values = vec![1, 2, 3];

    thread::scope(|scope| {
        scope.spawn(|| {
            println!("{values:?}");
        });
    });
}
```

`thread::scope` 的价值：

- 允许子线程安全借用当前作用域的数据。
- 所有 scoped thread 会在作用域结束前完成。
- 可以避免为了短期借用而强行使用 `Arc`。

---

## 3. 多线程与所有权

### 3.1 为什么常用 `move`

```rust
use std::thread;

fn main() {
    let values = vec![1, 2, 3];

    let handle = thread::spawn(move || {
        println!("{values:?}");
    });

    handle.join().unwrap();
}
```

新线程可能比创建它的栈帧存活得更久，因此 Rust 不允许线程持有可能悬垂的引用。

`move` 会把捕获变量的所有权移动进线程闭包。

### 3.2 跨线程传递数据的优先顺序

```text
1. 能移动所有权，就直接 move。
2. 需要共享所有权，再使用 Arc<T>。
3. 需要共享可变状态，使用 Arc<Mutex<T>> 等同步原语。
```

从 C++ 视角理解：

| Rust | C++ 近似概念 |
|---|---|
| `move` 闭包 | 将对象移动进线程，避免引用悬垂 |
| `Arc<T>` | 线程安全引用计数的 `shared_ptr` |
| `MutexGuard` | `std::lock_guard`，基于 RAII 自动解锁 |

---

## 4. Channel：消息传递并发

标准库提供 `std::sync::mpsc`：

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        tx.send(String::from("hello")).unwrap();
    });

    let message = rx.recv().unwrap();
    println!("{message}");
}
```

### 4.1 核心特征

- `mpsc`：Multiple Producer, Single Consumer。
- `send(value)` 会把 `value` 的所有权移动进 Channel。
- `recv()` 阻塞等待消息。
- 消息传递本质上是所有权转移。

### 4.2 多生产者

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel();

    for i in 0..3 {
        let tx = tx.clone();
        thread::spawn(move || {
            tx.send(i).unwrap();
        });
    }

    drop(tx);

    for value in rx {
        println!("{value}");
    }
}
```

注意：不再使用的原始 `Sender` 应及时 `drop`，否则接收端可能一直认为未来还会有新消息。

### 4.3 常见用途

- 任务分发
- 事件通知
- 日志异步写入
- 工作线程向主线程返回结果
- 减少共享可变状态

---

## 5. 共享状态并发

### 5.1 `Arc<T>`

`Arc` 是线程安全的原子引用计数智能指针。

```text
单线程共享所有权：Rc<T>
多线程共享所有权：Arc<T>
```

`Rc<T>` 的引用计数不是原子操作，因此不能安全跨线程使用。

### 5.2 `Arc<Mutex<T>>`

多个线程共享并修改同一份数据时，最常见的组合是：

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let counter = Arc::new(Mutex::new(0));
    let mut handles = Vec::new();

    for _ in 0..10 {
        let counter = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            let mut guard = counter.lock().unwrap();
            *guard += 1;
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("{}", *counter.lock().unwrap());
}
```

- `Arc` 解决共享所有权。
- `Mutex` 解决互斥访问。
- `lock()` 返回 `MutexGuard`。
- `MutexGuard` 离开作用域时自动解锁。

### 5.3 `RwLock<T>`

适用于读多写少：

```rust
use std::sync::RwLock;

fn main() {
    let value = RwLock::new(10);

    {
        let r1 = value.read().unwrap();
        let r2 = value.read().unwrap();
        println!("{} {}", *r1, *r2);
    }

    {
        let mut writer = value.write().unwrap();
        *writer += 1;
    }
}
```

- 多个读锁可以同时存在。
- 写锁必须独占。

### 5.4 原子类型

简单计数器或状态位可以使用 `Atomic*`：

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

let counter = AtomicUsize::new(0);
counter.fetch_add(1, Ordering::Relaxed);
```

适合：

- 计数器
- 状态标志
- 简单无锁同步

不适合：

- 复杂数据结构
- 需要同时维护多个不变量的状态

复杂场景优先使用 `Mutex`，不要为了“无锁”而过度增加正确性成本。

---

## 6. `Send` 与 `Sync`

### 6.1 `Send`

`Send` 表示：

> 一个值的所有权可以安全地移动到另一个线程。

例如：

- `String` 通常是 `Send`
- `Vec<T>` 在 `T: Send` 时是 `Send`
- `Rc<T>` 不是 `Send`

### 6.2 `Sync`

`Sync` 表示：

> 一个类型的共享引用 `&T` 可以安全地被多个线程使用。

等价关系：

```text
T: Sync  <=>  &T: Send
```

`Rc<T>`、`RefCell<T>` 通常不是 `Sync`，因为它们的内部状态修改不是线程安全的。

### 6.3 记忆方式

```text
Send 管“搬家”：值能不能跨线程移动。
Sync 管“共享”：引用能不能跨线程共享。
```

`thread::spawn`、`tokio::spawn` 等 API 会通过 Trait Bound 触发这些检查。类型不满足要求时，代码在编译期就会被拒绝。

---

## 7. Tokio 基础

Tokio 是 Rust 生态中最重要的异步 Runtime 之一。

Rust 标准库提供：

- `async` / `.await` 语法
- `Future` 抽象

Tokio 进一步提供：

- 异步任务调度
- 异步网络 I/O
- 定时器与超时
- 异步 Channel
- 异步锁
- 多线程 Runtime

### 7.1 基础示例

```rust
#[tokio::main]
async fn main() {
    let handle = tokio::spawn(async {
        println!("async task");
        42
    });

    let result = handle.await.unwrap();
    println!("result = {result}");
}
```

- `#[tokio::main]` 创建并启动 Runtime。
- `tokio::spawn` 创建异步 Task，而不是 OS 线程。
- `.await` 等待任务完成，但不会像普通阻塞调用一样长期占住线程。

### 7.2 Thread 与 Task 的区别

```text
std::thread::spawn：创建 OS 线程。
tokio::spawn：创建由 Tokio Runtime 调度的异步 Task。
```

Task 更轻量，一个服务中可以存在大量 Task；OS 线程的创建和上下文切换成本更高。

---

## 8. Tokio 与所有权、`Send`

### 8.1 `async move`

```rust
#[tokio::main]
async fn main() {
    let values = vec![1, 2, 3];

    let handle = tokio::spawn(async move {
        println!("{values:?}");
    });

    handle.await.unwrap();
}
```

`async move` 会把变量移动进 `Future` 状态机，避免 Task 借用外部短生命周期数据。

### 8.2 `'static` 约束

通过 `tokio::spawn` 启动的 Task 可能比当前函数活得更久，因此通常要求：

```text
Future: 'static
```

这里的 `'static` 并不表示 Task 永远存活，而是表示 Task 内部不能持有可能提前失效的借用。

### 8.3 `Send` 约束

Tokio 多线程 Runtime 可能把一个 Task 从某个 Worker Thread 移动到另一个 Worker Thread。

因此，跨 `.await` 被保存进 Future 状态机的数据通常必须是 `Send`。

常见写法：

```rust
use std::sync::Arc;

async fn process(data: Arc<String>) {
    println!("{data}");
}

#[tokio::main]
async fn main() {
    let data = Arc::new(String::from("shared"));

    let handle = tokio::spawn({
        let data = Arc::clone(&data);
        async move {
            process(data).await;
        }
    });

    handle.await.unwrap();
}
```

在多线程 Tokio 代码中，通常使用 `Arc<T>`，而不是 `Rc<T>`。

---

## 9. Tokio Channel 与共享状态

### 9.1 `tokio::sync::mpsc`

```rust
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move {
        tx.send("hello").await.unwrap();
    });

    while let Some(message) = rx.recv().await {
        println!("{message}");
    }
}
```

与标准库 Channel 的主要区别：

- `send().await` 和 `recv().await` 是异步等待。
- 等待期间 Task 可以让出执行权，不长期阻塞 Runtime Worker Thread。

### 9.2 Tokio 常见 Channel

| 类型 | 作用 |
|---|---|
| `mpsc` | 多生产者、单消费者 |
| `oneshot` | 一次性发送一个结果 |
| `broadcast` | 一个消息发送给所有订阅者 |
| `watch` | 只保留并观察最新值 |

### 9.3 Mutex 的选择

普通内存数据通常可以使用：

```rust
std::sync::Mutex
```

异步资源或确实需要跨 `.await` 持锁时，可以考虑：

```rust
tokio::sync::Mutex
```

但必须注意：

```rust
let guard = mutex.lock().unwrap();

// 不推荐：持有同步锁时执行 await
some_async_operation().await;

drop(guard);
```

推荐缩短锁作用域：

```rust
{
    let mut guard = mutex.lock().unwrap();
    // 只进行快速的内存操作
}

some_async_operation().await;
```

复杂异步资源更推荐：

```text
由一个专门 Task 独占资源，
其他 Task 通过 Channel 向它发送命令。
```

这类似 Actor 模型，可以显著降低锁和状态一致性的复杂度。

---

## 10. 工程中的选择

### 10.1 选择 `std::thread`

适合：

- 少量长期后台线程
- 简单 CLI 工具
- 直接控制 OS 线程
- 阻塞型任务
- 线程与外部同步库长期绑定

例如：

- 日志落盘线程
- 监控线程
- 文件扫描线程
- 与某个阻塞 SDK 绑定的专用线程

### 10.2 选择 Tokio

适合：

- Web / RPC 服务
- TCP / HTTP 服务
- 大量并发连接
- 异步数据库、Redis、消息队列客户端
- 定时器、超时和异步事件处理

Tokio 的优势来自：

```text
少量 OS 线程 + 大量异步 Task + 非阻塞 I/O
```

### 10.3 CPU 密集任务

不要直接在异步 Task 中执行长时间 CPU 计算：

```rust
// 不推荐
 tokio::spawn(async move {
    heavy_cpu_work();
});
```

因为这段代码在完成前没有 `.await`，会长期占用 Runtime Worker Thread。

可选择：

```rust
tokio::task::spawn_blocking(|| {
    heavy_cpu_work();
});
```

对于大量、持续的 CPU 密集任务，优先使用：

- Rayon
- 专用固定大小线程池
- 独立计算服务

### 10.4 阻塞 API 与 C FFI

同步文件 I/O、旧 SDK、阻塞数据库驱动、C/C++ FFI 等可能阻塞当前线程。

处理方式：

- 少量阻塞调用：`spawn_blocking`
- 长期独占或不可控阻塞：专用线程
- 大量 CPU 工作：Rayon / 专用线程池

---

## 11. 并发设计最佳实践

### 11.1 优先所有权转移

```text
能 move，就不要共享。
```

所有权转移通常最容易推导，也最不容易产生数据竞争和锁顺序问题。

### 11.2 优先消息传递

```text
能用 Channel 清晰表达，就少用共享可变状态。
```

Channel 特别适合任务队列、事件流和资源独占模型。

### 11.3 必须共享时再加锁

常见组合：

```text
Arc<Mutex<T>>       共享可变状态
Arc<RwLock<T>>      读多写少
Arc<AtomicUsize>    简单计数器
```

### 11.4 缩短持锁时间

- 锁内只做必要的数据读写。
- 不执行慢 I/O。
- 不做长时间 CPU 计算。
- 尽量不要跨 `.await` 持锁。

### 11.5 不要滥用 `Arc<Mutex<T>>`

看到所有对象都被包进 `Arc<Mutex<_>>`，通常说明并发边界没有设计清楚。

应先考虑：

- 是否可以直接移动所有权？
- 是否可以拆成独立 Task？
- 是否可以通过 Channel 交互？
- 是否真的需要多个线程同时访问？

### 11.6 正确处理异常退出

- `thread::join()` 可能返回 panic 信息。
- `Mutex` 可能发生 poisoning。
- Tokio `JoinHandle.await` 可能返回 `JoinError`。
- Channel 发送失败通常意味着接收端已经退出。

工程代码不应无条件依赖 `unwrap()`。

---

## 12. 面试与复习重点

需要能够清楚回答：

1. `std::thread` 和 Tokio Task 有什么区别？
2. 为什么线程闭包常使用 `move`？
3. `async move` 把数据移动到了哪里？
4. `Send` 和 `Sync` 分别约束什么？
5. 为什么 `Rc<T>` 不能跨线程，而 `Arc<T>` 可以？
6. Channel 和共享状态是两种怎样的并发模型？
7. 为什么不能在 async Task 中执行长时间阻塞操作？
8. `std::sync::Mutex` 与 `tokio::sync::Mutex` 应如何选择？

最终可压缩为：

```text
std::thread：直接使用 OS 线程。
Tokio：面向异步 I/O 的 Runtime 和 Task 调度系统。
Arc：跨线程共享所有权。
Mutex / RwLock / Atomic：保护共享可变状态。
Send / Sync：定义类型的编译期并发安全边界。
```
