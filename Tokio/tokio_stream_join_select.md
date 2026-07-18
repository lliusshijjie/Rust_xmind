# Tokio 中的 `Stream`、`join!` 与 `select!`

> 本文承接上一篇 `async / await / Pin` 笔记，只保留最重要的概念、用法和工程注意事项。

---

## 1. 三个概念分别解决什么问题

| 工具 | 解决的问题 | 核心语义 |
|---|---|---|
| `Stream` | 异步数据连续到达 | 不断等待“下一个值” |
| `tokio::join!` | 多个异步操作都要完成 | 全部完成后返回 |
| `tokio::select!` | 同时等待多个异步事件 | 最先就绪的分支获胜 |

一句话记忆：

```text
Stream    = 异步 Iterator
join!     = 等所有人
select!   = 等第一个人
```

三者都建立在 `Future + poll + Waker` 机制之上，但职责不同：

```text
Future：未来产生一个结果
Stream：未来不断产生多个结果
join!：组合多个 Future，等待全部结果
select!：组合多个 Future，处理最先到达的结果
```

---

# 2. Stream：异步版本的 Iterator

## 2.1 为什么需要 Stream

普通迭代器处理的是已经存在，或者可以立即计算出的数据：

```rust
let values = vec![1, 2, 3];

for value in values {
    println!("{value}");
}
```

但服务端中很多数据是随时间陆续到达的：

- TCP / WebSocket 消息；
- 消息队列事件；
- Channel 中的任务；
- 数据库分页结果；
- 定时器 Tick；
- 日志流和 SSE 推送。

这些数据暂时可能没有下一项，因此不能用同步 `Iterator` 准确表达。

`Stream` 可以理解为：

```text
Iterator<Item = T>
        +
等待下一项时允许当前 Task 返回 Pending
```

## 2.2 Stream Trait 的核心接口

`Stream` 的核心定义可以简化为：

```rust
trait Stream {
    type Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>>;
}
```

它有三种结果：

```text
Poll::Ready(Some(item))
    → 成功产生一个元素，后面可能还有数据

Poll::Pending
    → 当前还没有下一项，登记 Waker 后暂停 Task

Poll::Ready(None)
    → Stream 已经结束，不会再产生元素
```

与 `Future` 对比：

```text
Future：Poll<Output>
Stream：Poll<Option<Item>>
```

`Future` 一般只产生一次最终结果；`Stream` 每次 `Ready(Some)` 只产生一项，之后还可以继续 `poll_next`。

## 2.3 最常用的消费方式：`next().await`

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
```

```rust
use tokio_stream::{self as stream, StreamExt};

#[tokio::main]
async fn main() {
    let mut values = stream::iter(vec![10, 20, 30]);

    while let Some(value) = values.next().await {
        println!("{value}");
    }
}
```

可以把：

```rust
values.next().await
```

理解为异步版本的：

```rust
iterator.next()
```

区别是：当下一项尚未到达时，`next().await` 会暂停 Task，而不是阻塞 Tokio 工作线程。

## 2.4 实际场景：将 Channel 当作 Stream

```rust
use tokio::sync::mpsc;
use tokio_stream::{
    wrappers::ReceiverStream,
    StreamExt,
};

#[tokio::main]
async fn main() {
    let (tx, rx) = mpsc::channel(16);

    tokio::spawn(async move {
        for value in 1..=3 {
            tx.send(value).await.unwrap();
        }
    });

    let mut stream = ReceiverStream::new(rx);

    while let Some(value) = stream.next().await {
        println!("received: {value}");
    }
}
```

运行流程：

```text
Producer Task
    ↓ tx.send(value).await
mpsc Channel
    ↓
ReceiverStream
    ↓ next().await
Consumer Task
```

当 Channel 为空时，消费者返回 `Pending`；生产者发送新数据后，消费者 Task 被唤醒。

## 2.5 StreamExt 重点方法

### `next`

获取下一项：

```rust
while let Some(item) = stream.next().await {
    process(item).await;
}
```

### `map`

同步转换每一个元素：

```rust
let stream = stream.map(|value| value * 2);
```

### `then`

对每一个元素执行异步转换：

```rust
let stream = stream.then(|value| async move {
    query_database(value).await
});
```

### `filter` / `take` / `collect`

```rust
let stream = stream.filter(|value| value % 2 == 0);
let stream = stream.take(10);
let values: Vec<_> = stream.collect().await;
```

注意：无限 Stream 不能直接 `collect().await`，否则永远不会完成。

## 2.6 Stream 与 Pin

一些组合器生成的 Stream 可能是 `!Unpin`：

```rust
use tokio_stream::{self as stream, StreamExt};

#[tokio::main]
async fn main() {
    let values = stream::iter(1..=3).then(|value| async move {
        value * 2
    });

    tokio::pin!(values);

    while let Some(value) = values.next().await {
        println!("{value}");
    }
}
```

日常判断规则：

```text
普通 Stream 能直接 next()
    → 不需要显式 Pin

编译器提示不能 Unpin
    → 使用 tokio::pin!(stream)

需要堆存储或动态分发
    → 使用 Box::pin(stream)
```

## 2.7 C++ 视角理解 Stream

`Stream` 可以类比为：

```text
C++ Iterator
    +
异步 Generator
    +
持续产生事件的事件源
```

传统回调：

```cpp
connection.on_message([](Message message) {
    handle(message);
});
```

Stream 写法：

```rust
while let Some(message) = messages.next().await {
    handle(message).await;
}
```

它把“回调推送”转换成了更容易组合的“异步拉取”。

---

# 3. `join!`：并发执行，等待全部完成

## 3.1 基本用法

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn load_user() -> &'static str {
    sleep(Duration::from_secs(1)).await;
    "user"
}

async fn load_config() -> &'static str {
    sleep(Duration::from_secs(1)).await;
    "config"
}

#[tokio::main]
async fn main() {
    let (user, config) = tokio::join!(
        load_user(),
        load_config(),
    );

    println!("{user}, {config}");
}
```

两个 Future 会在当前 Task 内并发推进，总耗时约一秒，而不是两秒。

## 3.2 `join!` 不会创建新 Task

这是最重要的特性：

```rust
tokio::join!(a(), b());
```

并不等同于：

```rust
tokio::spawn(a());
tokio::spawn(b());
```

`join!` 的多个 Future：

- 保存在同一个组合 Future 中；
- 由同一个 Task 负责 poll；
- 可以并发，但不会因为 `join!` 本身获得多线程并行；
- 任何分支阻塞线程，其他分支也无法推进。

```text
join!：一个 Task 管理多个 Future
spawn：多个独立 Task 交给 Runtime 调度
```

## 3.3 `join!` 与串行 `.await`

串行：

```rust
let user = load_user().await;
let config = load_config().await;
```

```text
load_user 完成
    ↓
load_config 开始
```

并发：

```rust
let (user, config) = tokio::join!(
    load_user(),
    load_config(),
);
```

```text
load_user   ──────► 完成
load_config ──────► 完成
                   ↓
               join! 返回
```

选择原则：

```text
B 依赖 A 的结果
    → 串行 await

A 与 B 彼此独立，且都必须完成
    → join!
```

## 3.4 Result 场景优先考虑 `try_join!`

`join!` 即使遇到 `Err`，仍会等待其他分支完成：

```rust
let (user_result, config_result) = tokio::join!(
    load_user(),
    load_config(),
);
```

如果希望遇到第一个错误就提前返回：

```rust
let (user, config) = tokio::try_join!(
    load_user(),
    load_config(),
)?;
```

服务端同时调用多个下游时很常见：

```rust
async fn build_home_page(
    user_id: u64,
) -> Result<Page, AppError> {
    let (profile, recommendations, notices) =
        tokio::try_join!(
            load_profile(user_id),
            load_recommendations(user_id),
            load_notices(user_id),
        )?;

    Ok(Page {
        profile,
        recommendations,
        notices,
    })
}
```

## 3.5 什么时候使用 `join!`

适合：

- 多个操作相互独立；
- 所有结果都需要；
- 分支数量固定且较少；
- 不需要独立 Task 生命周期。

不适合：

- 任务需要独立运行；
- 任务数量动态且很多；
- 需要真正的 CPU 并行；
- 某个分支可能执行阻塞代码；
- 只关心最先完成的结果。

---

# 4. `select!`：处理最先就绪的异步事件

## 4.1 基本用法

```rust
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    tokio::select! {
        _ = sleep(Duration::from_secs(1)) => {
            println!("first branch completed");
        }

        _ = sleep(Duration::from_secs(3)) => {
            println!("second branch completed");
        }
    }
}
```

大约一秒后，第一个分支完成，整个 `select!` 返回：

```text
Future A ──1 秒──► Ready  ← 获胜
Future B ──3 秒──► 尚未完成，分支被丢弃
```

区别：

```text
join!：A 和 B 都完成后返回
select!：A 或 B 中第一个完成后返回
```

## 4.2 常见场景：业务操作与超时

```rust
use std::time::Duration;
use tokio::time::sleep;

async fn request() -> &'static str {
    sleep(Duration::from_secs(3)).await;
    "response"
}

#[tokio::main]
async fn main() {
    tokio::select! {
        response = request() => {
            println!("received: {response}");
        }

        _ = sleep(Duration::from_secs(1)) => {
            println!("request timeout");
        }
    }
}
```

不过，对于单个 Future 的超时，工程中通常优先使用：

```rust
tokio::time::timeout(duration, request()).await
```

`select!` 更适合同时监听多种事件。

## 4.3 同时监听消息和关闭信号

```rust
use tokio::sync::mpsc;

async fn run_worker(
    mut task_rx: mpsc::Receiver<String>,
    mut shutdown_rx: mpsc::Receiver<()>,
) {
    loop {
        tokio::select! {
            Some(task) = task_rx.recv() => {
                println!("process: {task}");
            }

            _ = shutdown_rx.recv() => {
                println!("worker shutting down");
                break;
            }
        }
    }
}
```

这类似于 C++ Reactor 同时等待：

```text
业务 fd 可读事件
eventfd / pipe 关闭事件
```

## 4.4 `select!` 通常放在循环中

一次 `select!` 只处理一次获胜事件：

```rust
loop {
    tokio::select! {
        Some(message) = message_rx.recv() => {
            handle_message(message).await;
        }

        _ = shutdown.recv() => {
            break;
        }
    }
}
```

要区分两种 Future。

### 每轮应该重新创建

例如每次接收下一条消息：

```rust
message_rx.recv()
```

### 整个循环只能创建一次

例如“整体五秒超时”：

```rust
let timeout = tokio::time::sleep(Duration::from_secs(5));
tokio::pin!(timeout);

loop {
    tokio::select! {
        _ = &mut timeout => {
            println!("overall timeout");
            break;
        }

        Some(message) = message_rx.recv() => {
            handle_message(message).await;
        }
    }
}
```

这里使用 `&mut timeout`，因为多轮 `select!` 要继续 poll 同一个 Future。

---

# 5. `select!` 最重要的注意事项：取消安全

## 5.1 未获胜分支会发生什么

当一个分支获胜时，其他分支对应的 Future 会被丢弃：

```rust
tokio::select! {
    result = operation_a() => { /* A 获胜 */ }
    result = operation_b() => { /* B 获胜 */ }
}
```

假设 A 先完成：

```text
A → Ready
B → 尚未完成
    ↓
B Future 被 Drop
```

这就是异步取消的一种形式。

## 5.2 什么是取消安全

如果一个异步操作被中途取消，之后重新开始不会丢失数据或破坏状态，就可以称为取消安全。

常见适合放入 `select!` 循环的操作包括：

```rust
receiver.recv()
stream.next()
```

例如关闭信号获胜时，本轮 `recv()` Future 虽然被丢弃，但消息不会因为一次未完成的接收而消失。

## 5.3 非取消安全操作的风险

假设某个 Future 内部执行：

```text
1. 从 socket 读取一部分数据
2. 数据仅暂存在 Future 内部
3. 再读取剩余数据
4. 返回完整消息
```

如果它在第 2 步被取消，已经读取的数据可能随 Future 一起被丢弃。

工程建议：

- 查阅 API 的 `Cancel safety` 文档；
- 长期状态尽量保存在 Future 外部；
- 协议解析使用持久化连接缓冲区；
- 单纯超时优先使用成熟的 `timeout` API；
- 多个同类数据源可以考虑合并为 Stream。

---

# 6. 公平性与 `biased;`

默认情况下，Tokio 会尽量轮换分支的 poll 顺序。

可以显式固定顺序：

```rust
tokio::select! {
    biased;

    _ = shutdown.recv() => {
        break;
    }

    Some(message) = message_rx.recv() => {
        handle(message).await;
    }
}
```

`biased;` 表示按照代码顺序 poll。

这里把关闭信号放在前面，是为了防止高流量消息分支持续就绪时，关闭信号长期得不到处理。

一般不需要主动使用 `biased;`；只有明确需要固定优先级时才使用，并注意低优先级分支饥饿问题。

---

# 7. 三者如何组合

## 7.1 Stream + select!

持续消费消息，同时响应关闭信号：

```rust
use tokio_stream::{Stream, StreamExt};

async fn consume<S>(
    stream: S,
    mut shutdown: tokio::sync::mpsc::Receiver<()>,
)
where
    S: Stream<Item = String>,
{
    tokio::pin!(stream);

    loop {
        tokio::select! {
            message = stream.next() => {
                match message {
                    Some(message) => {
                        println!("received: {message}");
                    }
                    None => {
                        println!("stream completed");
                        break;
                    }
                }
            }

            _ = shutdown.recv() => {
                println!("shutdown");
                break;
            }
        }
    }
}
```

## 7.2 join! + Stream

并发运行两个长期消费者：

```rust
async fn consume_orders() {
    // 持续消费订单 Stream
}

async fn consume_logs() {
    // 持续消费日志 Stream
}

#[tokio::main]
async fn main() {
    tokio::join!(
        consume_orders(),
        consume_logs(),
    );
}
```

如果两个消费者都是无限循环，`join!` 正常情况下永远不会返回，因此还需要配合关闭信号和错误传播机制。

## 7.3 spawn + select!

```rust
let task_a = tokio::spawn(worker_a());
let task_b = tokio::spawn(worker_b());

tokio::select! {
    result = task_a => {
        println!("worker A exited: {result:?}");
    }

    result = task_b => {
        println!("worker B exited: {result:?}");
    }
}
```

注意：

> `JoinHandle` 在 `select!` 中未获胜而被丢弃，不代表对应 Task 自动停止。

`tokio::spawn` 创建的 Task 有独立生命周期。需要结束它时，应调用 `abort()`，或设计关闭 Channel、取消令牌等协作式退出机制。

---

# 8. 选择速查表

| 需求 | 推荐工具 |
|---|---|
| 连续处理异步数据 | `Stream` |
| 等待下一个事件 | `stream.next().await` |
| 多个独立请求都要成功 | `try_join!` |
| 多个异步操作都必须完成 | `join!` |
| 谁先完成就处理谁 | `select!` |
| 单个请求超时 | `tokio::time::timeout` |
| 消息与关闭信号同时监听 | `select!` |
| 独立任务生命周期 | `tokio::spawn` |
| 动态数量的并发 Future | `FuturesUnordered` 等集合工具 |
| 多个同类事件源 | 优先考虑合并 Stream |

---

# 9. C++ 网络编程对比

## Stream

```text
Rust Stream
    ≈ C++20 async generator
    ≈ 持续产生事件的 Connection
    ≈ 对回调事件源的拉取式封装
```

## join!

类似同时发起多个异步 RPC，等待全部回调完成：

```text
RPC A ─────► result A
RPC B ─────► result B
RPC C ─────► result C
             ↓
          组合结果
```

Tokio 用 Future 状态机组织流程，不需要手动维护回调计数器。

## select!

类似 Reactor 同时等待多个事件源：

```text
socket readable
timer expired
shutdown eventfd
channel message
```

```rust
tokio::select! {
    data = socket.read(&mut buf) => { /* 网络数据 */ }
    _ = timer.tick() => { /* 定时任务 */ }
    _ = shutdown.recv() => { /* 关闭 */ }
}
```

---

# 10. 最佳实践总结

## Stream

1. 把 Stream 理解为异步数据序列，而不是单次异步结果。
2. 使用 `while let Some(item) = stream.next().await` 消费。
3. `None` 表示 Stream 已经结束。
4. 对无限 Stream 不要直接 `collect().await`。
5. 出现 `!Unpin` 编译错误时，再使用 `tokio::pin!`。
6. 异步转换用 `then`，同步转换用 `map`。
7. 高流量 Stream 要设计背压和有界 Channel。

## join!

1. 多个 Future 相互独立且全部需要完成时使用。
2. `join!` 是并发，不保证多线程并行。
3. `Result` 场景通常优先考虑 `try_join!`。
4. 任一分支阻塞线程，其他分支也无法继续。
5. 独立生命周期任务使用 `spawn`。

## select!

1. 用于等待最先完成的异步事件。
2. 一次 `select!` 只处理一个获胜分支。
3. 循环中复用同一个 Future 时，在循环外创建并 Pin。
4. 必须关注未获胜分支被取消后的安全性。
5. 单个操作超时优先使用 `tokio::time::timeout`。
6. 默认公平轮询即可，谨慎使用 `biased;`。
7. 多个同类事件源可以优先合并成 Stream。

---

# 11. 最终认知图

```text
                         异步任务组织
                              │
              ┌───────────────┼───────────────┐
              │               │               │
           Stream           join!          select!
              │               │               │
     连续产生多个值      等待所有分支      等待首个分支
              │               │               │
       next().await      返回结果元组      其他 Future 被取消
              │               │               │
     消息/连接/事件流   并发查询多个服务   超时/关闭/事件竞争
```

最简记忆：

```text
Future 是一次异步计算；
Stream 是多次异步产出；

join! 关心“全部完成”；
select! 关心“谁先完成”。
```

---

# 参考资料

- [futures-core：Stream Trait](https://docs.rs/futures-core/latest/futures_core/stream/trait.Stream.html)
- [tokio-stream：StreamExt](https://docs.rs/tokio-stream/latest/tokio_stream/trait.StreamExt.html)
- [Tokio：join!](https://docs.rs/tokio/latest/tokio/macro.join.html)
- [Tokio：try_join!](https://docs.rs/tokio/latest/tokio/macro.try_join.html)
- [Tokio：select!](https://docs.rs/tokio/latest/tokio/macro.select.html)
