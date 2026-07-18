# 使用 Rust 实现一个简单的 Web 服务器

> 本项目对应《Rust 权威指南》最后一章的实践内容。
> 目标不是实现生产级 HTTP 服务器，而是综合练习：
>
> - TCP 网络编程
> - HTTP 请求与响应
> - 所有权与生命周期
> - 闭包与 Trait 对象
> - 多线程与消息传递
> - `Arc<Mutex<T>>`
> - RAII 与 `Drop`

------

## 1. 项目整体结构

最终工程结构如下：

```text
web-server/
├── Cargo.toml
├── hello.html
├── 404.html
└── src
    ├── lib.rs
    └── main.rs
```

创建项目：

```bash
cargo new web-server
cd web-server
```

------

# 2. 第一阶段：监听 TCP 连接

Web 服务器底层首先是一个 TCP 服务器。

```rust
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878")
        .expect("failed to bind address");

    for stream in listener.incoming() {
        let stream = stream.expect("failed to accept connection");
        println!("Connection established: {:?}", stream.peer_addr());
    }
}
```

## 核心逻辑

```rust
TcpListener::bind("127.0.0.1:7878")
```

表示监听本机的 `7878` 端口。

```rust
listener.incoming()
```

返回一个迭代器，每当客户端连接服务器时，就产生一个：

```rust
Result<TcpStream, std::io::Error>
```

其中 `TcpStream` 表示服务器与客户端之间的一条 TCP 连接。

运行：

```bash
cargo run
```

浏览器访问：

```text
http://127.0.0.1:7878
```

此时服务器可以接收到连接，但不会返回任何 HTTP 响应。

------

# 3. 第二阶段：读取 HTTP 请求

HTTP 请求本质上是通过 TCP 发送的一段文本。

修改代码：

```rust
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};

fn handle_connection(stream: TcpStream) {
    let reader = BufReader::new(stream);

    let request: Vec<String> = reader
        .lines()
        .map(|line| line.expect("failed to read request"))
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {request:#?}");
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878")
        .expect("failed to bind address");

    for stream in listener.incoming() {
        let stream = stream.expect("failed to accept connection");
        handle_connection(stream);
    }
}
```

浏览器发送的请求大致如下：

```http
GET / HTTP/1.1
Host: 127.0.0.1:7878
Connection: keep-alive
User-Agent: Mozilla/5.0
```

第一行最重要：

```http
GET / HTTP/1.1
```

它可以拆分为：

```text
GET       请求方法
/         请求路径
HTTP/1.1  HTTP 协议版本
```

这里使用 `BufReader<TcpStream>`，是因为直接从 TCP 流中逐字节读取比较低效。

------

# 4. 第三阶段：返回 HTTP 响应

一个最简单的 HTTP 响应如下：

```http
HTTP/1.1 200 OK
Content-Length: 12

Hello world!
```

HTTP 响应由三部分组成：

```text
状态行
响应头
空行
响应体
```

实现：

```rust
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn handle_connection(mut stream: TcpStream) {
    let reader = BufReader::new(&stream);

    let request_line = reader
        .lines()
        .next()
        .expect("request is empty")
        .expect("failed to read request");

    println!("Request: {request_line}");

    let body = "Hello world!";

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    stream
        .write_all(response.as_bytes())
        .expect("failed to write response");
}
```

注意 HTTP 头部使用：

```rust
"\r\n"
```

而不是单纯的：

```rust
"\n"
```

------

# 5. 第四阶段：返回 HTML 文件

创建 `hello.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>Rust Web Server</title>
</head>
<body>
    <h1>Hello!</h1>
    <p>This page is served by Rust.</p>
</body>
</html>
```

服务器读取 HTML 文件：

```rust
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

fn handle_connection(mut stream: TcpStream) {
    let reader = BufReader::new(&stream);

    let request_line = reader
        .lines()
        .next()
        .expect("request is empty")
        .expect("failed to read request");

    let contents = fs::read_to_string("hello.html")
        .expect("failed to read hello.html");

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{}",
        contents.len(),
        contents
    );

    stream
        .write_all(response.as_bytes())
        .expect("failed to write response");
}
```

------

# 6. 第五阶段：处理不同请求路径

服务器需要根据请求路径返回不同内容。

例如：

```text
GET / HTTP/1.1
```

返回主页。

其他路径：

```text
GET /abc HTTP/1.1
```

返回 `404 Not Found`。

创建 `404.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>404 Not Found</title>
</head>
<body>
    <h1>404</h1>
    <p>The requested page was not found.</p>
</body>
</html>
```

请求处理：

```rust
let (status_line, filename) = match request_line.as_str() {
    "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
    _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
};
```

完整逻辑：

```rust
fn handle_connection(mut stream: TcpStream) {
    let reader = BufReader::new(&stream);

    let request_line = reader
        .lines()
        .next()
        .expect("request is empty")
        .expect("failed to read request");

    let (status_line, filename) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename)
        .expect("failed to read HTML file");

    let response = format!(
        "{status_line}\r\nContent-Length: {}\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n{contents}",
        contents.len()
    );

    stream
        .write_all(response.as_bytes())
        .expect("failed to write response");
}
```

------

# 7. 第六阶段：模拟慢请求

为了观察单线程服务器的问题，增加 `/sleep` 路径：

```rust
use std::thread;
use std::time::Duration;

match request_line.as_str() {
    "GET / HTTP/1.1" => {
        // 返回主页
    }

    "GET /sleep HTTP/1.1" => {
        thread::sleep(Duration::from_secs(5));
        // 返回主页
    }

    _ => {
        // 返回 404
    }
}
```

当一个浏览器访问：

```text
http://127.0.0.1:7878/sleep
```

服务器会阻塞 5 秒。

此时另一个浏览器访问主页，也必须等待前一个请求完成。

原因是当前处理方式为：

```rust
for stream in listener.incoming() {
    handle_connection(stream);
}
```

所有连接都由主线程串行处理：

```text
请求 A → 处理完成 → 请求 B → 处理完成 → 请求 C
```

------

# 8. 第七阶段：为每个请求创建线程

最直接的多线程方案是：

```rust
for stream in listener.incoming() {
    let stream = stream.expect("failed to accept connection");

    thread::spawn(|| {
        handle_connection(stream);
    });
}
```

由于闭包需要取得 `stream` 的所有权，实际应使用：

```rust
thread::spawn(move || {
    handle_connection(stream);
});
```

这样多个请求可以并发处理。

但是这种方式存在问题：

```text
每个连接都创建一个新线程
```

如果短时间内收到大量请求，就可能创建大量线程，耗尽系统资源。

因此需要使用线程池。

------

# 9. 第八阶段：设计线程池

线程池的基本思想是：

```text
启动服务器
    ↓
提前创建固定数量的工作线程
    ↓
请求到达
    ↓
将任务放入任务队列
    ↓
空闲线程从队列中取任务执行
```

服务器代码希望写成：

```rust
let pool = ThreadPool::new(4);

for stream in listener.incoming() {
    let stream = stream.expect("failed to accept connection");

    pool.execute(|| {
        handle_connection(stream);
    });
}
```

线程池需要提供两个接口：

```rust
ThreadPool::new(size)
```

创建固定数量的线程。

```rust
pool.execute(job)
```

向线程池提交任务。

------

# 10. 定义 Job 类型

线程池中的任务本质上是一个闭包：

```rust
type Job = Box<dyn FnOnce() + Send + 'static>;
```

解释：

```text
Box<...>
```

使用堆内存保存大小不确定的闭包。

```text
dyn FnOnce()
```

任务只需要执行一次。

```text
Send
```

任务会从主线程发送到工作线程。

```text
'static
```

闭包不能借用可能提前失效的局部变量。

这与 C++ 中的：

```cpp
std::function<void()>
```

比较类似，但 Rust 对线程安全和生命周期进行了静态约束。

------

# 11. 使用 Channel 传递任务

主线程作为任务生产者：

```rust
sender.send(job)
```

工作线程作为任务消费者：

```rust
receiver.recv()
```

Rust 标准库的 `mpsc` 是：

```text
multiple producer, single consumer
```

它的 `Receiver` 不能直接被多个线程同时持有，因此需要包装为：

```rust
Arc<Mutex<Receiver<Message>>>
```

含义：

```text
Arc
```

允许多个工作线程共享 Receiver 的所有权。

```text
Mutex
```

保证同一时刻只有一个线程执行 `recv()`。

------

# 12. 定义线程池消息

除了执行任务，还需要支持通知线程退出：

```rust
enum Message {
    NewJob(Job),
    Terminate,
}
```

工作线程根据消息类型执行不同操作：

```rust
match message {
    Message::NewJob(job) => {
        job();
    }

    Message::Terminate => {
        break;
    }
}
```

------

# 13. Worker 工作线程

每个 `Worker` 保存一个线程句柄：

```rust
struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}
```

之所以使用：

```rust
Option<JoinHandle<()>>
```

是因为调用 `join()` 时需要取得 `JoinHandle` 的所有权。

但是在 `Drop::drop(&mut self)` 中只有可变引用，不能直接移动字段，因此需要：

```rust
worker.thread.take()
```

将：

```rust
Some(handle)
```

替换成：

```rust
None
```

并取出其中的线程句柄。

------

# 14. 使用 Drop 关闭线程池

线程池销毁时应：

1. 向每个工作线程发送退出消息。
2. 等待所有线程执行结束。

```rust
impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender
                .send(Message::Terminate)
                .expect("failed to send terminate message");
        }

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().expect("worker thread panicked");
            }
        }
    }
}
```

这体现了 Rust 的 RAII：

```text
ThreadPool 离开作用域
    ↓
自动调用 Drop
    ↓
通知线程退出
    ↓
回收线程资源
```

它和 C++ 析构函数管理资源的思想基本相同。

------

# 15. 完整可运行代码

## 15.1 Cargo.toml

```toml
[package]
name = "web-server"
version = "0.1.0"
edition = "2024"

[dependencies]
```

如果本地 Rust 版本不支持 Edition 2024，也可以改成：

```toml
edition = "2021"
```

------

## 15.2 src/lib.rs

```rust
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "thread pool size must be greater than zero");

        let (sender, receiver) = mpsc::channel::<Message>();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self { workers, sender }
    }

    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(job);

        self.sender
            .send(Message::NewJob(job))
            .expect("failed to send job to worker");
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("Shutting down thread pool.");

        for _ in &self.workers {
            self.sender
                .send(Message::Terminate)
                .expect("failed to send terminate message");
        }

        for worker in &mut self.workers {
            println!("Shutting down worker {}.", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().expect("worker thread panicked");
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<Message>>>,
    ) -> Self {
        let thread = thread::spawn(move || loop {
            let message = {
                let receiver = receiver
                    .lock()
                    .expect("worker failed to lock receiver");

                receiver.recv()
            };

            match message {
                Ok(Message::NewJob(job)) => {
                    println!("Worker {id} received a job.");
                    job();
                }

                Ok(Message::Terminate) => {
                    println!("Worker {id} received terminate signal.");
                    break;
                }

                Err(_) => {
                    println!("Worker {id}: channel disconnected.");
                    break;
                }
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}
```

------

## 15.3 src/main.rs

```rust
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use web_server::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878")
        .expect("failed to bind 127.0.0.1:7878");

    let pool = ThreadPool::new(4);

    println!("Server running at http://127.0.0.1:7878");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(move || {
                    handle_connection(stream);
                });
            }

            Err(error) => {
                eprintln!("Failed to accept connection: {error}");
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let request_line = {
        let reader = BufReader::new(&stream);

        match reader.lines().next() {
            Some(Ok(line)) => line,

            Some(Err(error)) => {
                eprintln!("Failed to read request: {error}");
                return;
            }

            None => {
                eprintln!("Received empty request");
                return;
            }
        }
    };

    println!("Request: {request_line}");

    let (status_line, filename) = match request_line.as_str() {
        "GET / HTTP/1.1" => {
            ("HTTP/1.1 200 OK", "hello.html")
        }

        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }

        _ => {
            ("HTTP/1.1 404 NOT FOUND", "404.html")
        }
    };

    let contents = match fs::read_to_string(filename) {
        Ok(contents) => contents,

        Err(error) => {
            eprintln!("Failed to read {filename}: {error}");

            let body = "500 Internal Server Error";

            let response = format!(
                "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\
                 Content-Length: {}\r\n\
                 Content-Type: text/plain; charset=utf-8\r\n\
                 Connection: close\r\n\
                 \r\n\
                 {}",
                body.len(),
                body
            );

            let _ = stream.write_all(response.as_bytes());
            return;
        }
    };

    let response = format!(
        "{status_line}\r\n\
         Content-Length: {}\r\n\
         Content-Type: text/html; charset=utf-8\r\n\
         Connection: close\r\n\
         \r\n\
         {contents}",
        contents.len()
    );

    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("Failed to send response: {error}");
    }
}
```

------

## 15.4 hello.html

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>Rust Web Server</title>
</head>
<body>
    <h1>Hello!</h1>
    <p>This page is served by a Rust thread pool.</p>

    <p>
        <a href="/">Home</a>
    </p>

    <p>
        <a href="/sleep">Sleep for 5 seconds</a>
    </p>

    <p>
        <a href="/not-found">Test 404</a>
    </p>
</body>
</html>
```

------

## 15.5 404.html

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <title>404 Not Found</title>
</head>
<body>
    <h1>404 Not Found</h1>
    <p>The requested page does not exist.</p>

    <a href="/">Back to home</a>
</body>
</html>
```

------

# 16. 运行项目

在项目根目录执行：

```bash
cargo run
```

访问主页：

```text
http://127.0.0.1:7878/
```

测试慢请求：

```text
http://127.0.0.1:7878/sleep
```

测试 404：

```text
http://127.0.0.1:7878/abc
```

也可以使用 `curl`：

```bash
curl http://127.0.0.1:7878/
curl http://127.0.0.1:7878/sleep
curl http://127.0.0.1:7878/not-found
```

------

# 17. 如何测试优雅关闭

正常版本的服务器会一直监听连接，因此 `ThreadPool` 通常不会离开作用域。

为了观察 `Drop` 的关闭流程，可以临时修改：

```rust
for stream in listener.incoming() {
```

为：

```rust
for stream in listener.incoming().take(2) {
```

这样服务器只接受两个连接。

两个请求处理完后：

```text
main 函数结束
    ↓
ThreadPool 离开作用域
    ↓
调用 Drop
    ↓
发送 Terminate
    ↓
等待所有 Worker 退出
```

测试完成后，应恢复无限监听版本。

------

# 18. 代码中的关键 Rust 知识

## 18.1 `move` 闭包

```rust
pool.execute(move || {
    handle_connection(stream);
});
```

`stream` 必须移动到任务闭包中。

任务随后可能由另一个线程执行，因此闭包不能借用主线程栈上的局部变量。

------

## 18.2 `FnOnce`

```rust
type Job = Box<dyn FnOnce() + Send + 'static>;
```

任务只会被执行一次，并且可能消费捕获的变量，因此使用 `FnOnce`。

------

## 18.3 `Send`

```rust
F: FnOnce() + Send + 'static
```

任务从主线程发送给工作线程，因此任务必须可以安全地跨线程移动。

------

## 18.4 `Arc<Mutex<T>>`

```rust
Arc<Mutex<mpsc::Receiver<Message>>>
```

这里有两层含义：

```text
Arc：多个 Worker 共享 Receiver
Mutex：同一时刻只有一个 Worker 调用 recv
```

------

## 18.5 `Box<dyn Trait>`

不同闭包具有不同的匿名类型。

线程池需要将不同闭包统一存入 Channel，因此使用：

```rust
Box<dyn FnOnce() + Send + 'static>
```

进行类型擦除。

这类似于 C++ 中使用：

```cpp
std::function<void()>
```

统一保存不同类型的可调用对象。

------

## 18.6 `Drop`

```rust
impl Drop for ThreadPool
```

负责在线程池销毁时：

```text
发送退出信号
等待线程结束
释放线程资源
```

这与 C++ 的 RAII 和析构函数非常相似。

------

# 19. 线程池执行流程

```text
客户端发送 HTTP 请求
        │
        ▼
TcpListener 接收到 TcpStream
        │
        ▼
主线程调用 ThreadPool::execute
        │
        ▼
闭包被包装成 Box<dyn FnOnce()>
        │
        ▼
通过 mpsc Channel 发送 Message::NewJob
        │
        ▼
某个 Worker 调用 receiver.recv()
        │
        ▼
Worker 取得任务并执行 job()
        │
        ▼
handle_connection 解析请求
        │
        ▼
读取 HTML 文件
        │
        ▼
通过 TcpStream 返回 HTTP 响应
```

------

# 20. 这个项目真正需要掌握什么

不需要死记整个线程池实现，重点理解以下组合：

```text
TcpListener + TcpStream
```

实现基本 TCP 服务。

```text
BufReader + Write
```

读取请求并写回响应。

```text
ThreadPool + Channel
```

将任务分发给固定数量的工作线程。

```text
Arc<Mutex<Receiver>>
```

多个线程共享任务接收端。

```text
Box<dyn FnOnce() + Send + 'static>
```

表示可跨线程发送、只执行一次的动态任务。

```text
Drop + JoinHandle
```

管理工作线程生命周期。

------

# 21. 工程上的局限性

这个服务器只适合学习，不适合生产环境。

它没有完整实现：

- HTTP 请求解析
- Keep-Alive
- 请求体解析
- POST、PUT、DELETE 等方法
- URL 参数和查询参数
- HTTPS/TLS
- 请求大小限制
- 超时控制
- 异步 I/O
- 日志系统
- 路由系统
- 防御恶意请求
- 优雅停止监听器

实际 Rust Web 服务通常使用：

```text
Tokio
Axum
Actix Web
Hyper
Tower
```

《Rust 权威指南》的这个项目真正目的，是让我们理解 Web 框架底层的一些基础机制，而不是重新实现一个完整 Web 框架。

------

# 22. 最终总结

整个实现过程可以概括为：

```text
TcpListener 监听端口
        ↓
TcpStream 表示客户端连接
        ↓
读取 HTTP 请求行
        ↓
根据请求路径选择响应文件
        ↓
构造 HTTP 响应
        ↓
线程池并发处理连接
        ↓
Channel 分发任务
        ↓
Arc<Mutex<T>> 共享接收端
        ↓
Drop 回收工作线程
```

对于有 C++ 背景的开发者，可以建立下面的对应关系：

| Rust                | C++ 类比                        |
| ------------------- | ------------------------------- |
| `TcpListener`       | 监听 socket                     |
| `TcpStream`         | 已连接 socket                   |
| `Box<dyn FnOnce()>` | `std::function<void()>`         |
| `mpsc::channel`     | 线程安全任务队列                |
| `Arc<T>`            | `std::shared_ptr<T>`            |
| `Mutex<T>`          | `std::mutex` 与被保护对象的组合 |
| `JoinHandle`        | `std::thread`                   |
| `Drop`              | 析构函数                        |
| `ThreadPool`        | 固定工作线程与任务队列          |

这个项目将 Rust 的所有权、闭包、Trait 对象、并发和 RAII 串联到了一起，是《Rust 权威指南》中非常重要的综合练习。