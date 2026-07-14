# Rust 错误机制总结

## 1. 核心思想

Rust 将错误分为两类：

| 错误类型 | 处理机制 | 典型场景 |
|---|---|---|
| 可恢复错误 | `Result<T, E>` | 文件不存在、网络超时、解析失败、数据库错误、非法用户输入 |
| 不可恢复错误 | `panic!` | 内部不变量被破坏、程序逻辑错误、理论上不可能出现的状态 |

Rust 的设计重点不是“避免所有错误”，而是：

- 将可能失败的事实写进函数返回类型；
- 让错误路径保持显式；
- 避免异常带来的隐藏控制流；
- 使 API 契约和性能模型更加可预测。

可以将其概括为：

> `Result` 处理正常世界中的失败，`panic!` 处理程序自身已经进入错误状态的情况。

---

## 2. `Result<T, E>`

`Result` 是 Rust 表示可恢复错误的核心类型：

```rust
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

其中：

- `T`：成功时返回的数据类型；
- `E`：失败时返回的错误类型；
- `Ok(value)`：操作成功；
- `Err(error)`：操作失败，并携带错误原因。

示例：

```rust
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("除数不能为 0".to_string())
    } else {
        Ok(a / b)
    }
}
```

调用者可以使用 `match` 分别处理成功和失败：

```rust
match divide(10, 0) {
    Ok(value) => println!("结果：{value}"),
    Err(error) => eprintln!("错误：{error}"),
}
```

### 为什么要使用 `Result`

函数签名直接表达了操作可能失败：

```rust
fn read_config(path: &str) -> Result<String, std::io::Error>
```

调用者不需要阅读实现或依赖额外文档，就能知道：

1. 成功时得到 `String`；
2. 失败时得到 `std::io::Error`；
3. 这个失败需要被处理或继续向上传播。

---

## 3. `?` 运算符：传播错误

手动使用 `match` 传播错误会产生大量样板代码：

```rust
let content = match std::fs::read_to_string("config.txt") {
    Ok(value) => value,
    Err(error) => return Err(error),
};
```

可以使用 `?` 简化：

```rust
let content = std::fs::read_to_string("config.txt")?;
```

其核心语义是：

```text
Ok(value)  -> 取出 value，继续执行
Err(error) -> 提前返回 Err(error)
```

完整示例：

```rust
use std::fs;
use std::io;

fn read_config(path: &str) -> Result<String, io::Error> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}
```

`?` 虽然简洁，但本质上仍然是显式的返回值传播，而不是异常式的非局部跳转。

当底层错误类型和当前函数的错误类型不完全相同时，`?` 还可以通过 `From` / `Into` 完成错误类型转换。

---

## 4. `panic!`：不可恢复错误

`panic!` 表示当前程序已经进入不应该继续执行的状态。

```rust
fn get_element(values: &[i32], index: usize) -> i32 {
    if index >= values.len() {
        panic!("index out of bounds");
    }

    values[index]
}
```

适合使用 `panic!` 的情况包括：

- 内部不变量被破坏；
- 调用者违反明确的 API 契约；
- 出现理论上不可达的状态；
- 继续运行可能产生更严重或不安全的结果。

不应该使用 `panic!` 处理普通业务失败，例如：

- 文件不存在；
- 用户输入格式错误；
- 网络连接失败；
- 数据库查询失败。

这些都属于可预期失败，应返回 `Result`。

---

## 5. `unwrap` 与 `expect`

### `unwrap`

`unwrap` 在成功时取出值，在失败时触发 `panic!`：

```rust
let value = "123".parse::<i32>().unwrap();
```

其行为可以理解为：

```rust
match result {
    Ok(value) => value,
    Err(error) => panic!("{error:?}"),
}
```

### `expect`

`expect` 与 `unwrap` 类似，但允许补充错误信息：

```rust
let address = "127.0.0.1"
    .parse::<std::net::IpAddr>()
    .expect("硬编码的 IP 地址必须合法");
```

`expect` 的信息应说明：

> 为什么开发者认为这里理论上不会失败。

### 使用原则

适合使用：

- 单元测试；
- 示例代码；
- 快速原型；
- 能够通过程序结构严格证明不会失败的地方。

业务代码中不应滥用，否则会把本来可恢复的错误升级为程序崩溃。

---

## 6. `Box<dyn std::error::Error>`

`Box<dyn Error>` 用于统一承载不同的错误类型：

```rust
use std::error::Error;
use std::fs;

fn read_number(path: &str) -> Result<i32, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let number = content.trim().parse::<i32>()?;
    Ok(number)
}
```

这里可能产生两种不同错误：

- `std::io::Error`；
- `std::num::ParseIntError`。

它们都实现了 `std::error::Error`，因此可以统一转换为 `Box<dyn Error>`。

### 从 C++ 视角理解

```rust
Box<dyn Error>
```

可以近似类比为：

```cpp
std::unique_ptr<IError>
```

对应关系为：

- `Box`：独占拥有堆上的具体错误对象；
- `dyn Error`：通过 trait object 实现运行时多态；
- 底层可理解为数据指针与虚表指针的组合。

但 Rust 中不是“子类继承错误基类”，而是具体类型实现了 `Error` trait。

### 适用范围

`Box<dyn Error>` 适合：

- 应用程序入口；
- 命令行工具；
- 原型程序；
- 只需要统一向上传播错误的边界层。

它不适合被库的公共 API 大量使用，因为类型擦除后，调用者不容易精确匹配具体错误。

---

## 7. 自定义错误类型

库代码通常应该定义明确的错误类型：

```rust
#[derive(Debug)]
enum ConfigError {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
    MissingField(String),
}
```

自定义错误通常需要实现：

- `Debug`：供调试输出；
- `Display`：提供面向用户的错误描述；
- `std::error::Error`：接入 Rust 标准错误体系。

完整手写会产生一些样板代码，工程中常使用 `thiserror` 简化：

```rust
use thiserror::Error;

#[derive(Debug, Error)]
enum ConfigError {
    #[error("读取配置文件失败: {0}")]
    Io(#[from] std::io::Error),

    #[error("解析数字失败: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error("缺少字段: {0}")]
    MissingField(String),
}
```

这样调用者可以精确处理不同错误：

```rust
match load_config() {
    Ok(config) => use_config(config),
    Err(ConfigError::MissingField(name)) => {
        eprintln!("配置缺少字段：{name}");
    }
    Err(error) => eprintln!("加载失败：{error}"),
}
```

---

## 8. `thiserror` 与 `anyhow` 的分工

### `thiserror`

适合库代码或领域层：

- 定义结构化错误枚举；
- 保留具体错误类别；
- 允许调用者通过 `match` 做精确处理；
- 简化 `Display`、`Error` 和 `From` 实现。

### `anyhow`

适合应用层：

- 聚合不同来源的错误；
- 快速添加上下文信息；
- 减少顶层业务流程中的错误类型样板代码。

典型原则是：

```text
库代码：thiserror + 明确错误类型
应用层：anyhow + 统一传播和记录
```

---

## 9. 与 C++ 异常机制对比

| 对比点 | C++ 异常 | Rust `Result` |
|---|---|---|
| 错误是否体现在返回类型 | 通常不体现 | 明确体现在 `Result<T, E>` 中 |
| 控制流 | 非局部跳转，较隐式 | 普通返回值与提前返回 |
| 调用者是否容易发现失败路径 | 依赖文档和实现 | 从函数签名直接看出 |
| 错误传播 | `throw` / `catch` | `?` |
| 性能模型 | 正常路径与异常路径差异较大 | 普通枚举与分支，较可预测 |
| 典型定位 | 语言提供的错误机制之一 | Rust 生态的默认可恢复错误模型 |

Rust 没有否定“程序可能失败”，而是选择将可恢复错误纳入类型系统。

这特别适合：

- 系统编程；
- 高可靠服务；
- 嵌入式和 `no_std`；
- FFI 边界；
- 对控制流和资源释放要求严格的代码。

---

## 10. 工程最佳实践

### 10.1 可预期失败返回 `Result`

```rust
fn parse_port(text: &str) -> Result<u16, std::num::ParseIntError> {
    text.parse()
}
```

不要把正常失败写成 `panic!`。

### 10.2 错误传播优先使用 `?`

```rust
fn load() -> Result<Data, AppError> {
    let text = read_file()?;
    let data = parse_data(&text)?;
    Ok(data)
}
```

只有需要恢复、转换、记录或降级处理时，才显式使用 `match`。

### 10.3 业务代码避免滥用 `unwrap`

不推荐：

```rust
let config = load_config().unwrap();
```

更合理的做法包括：

- 使用 `?` 传播；
- 使用 `match` 恢复；
- 使用 `unwrap_or` 提供合理默认值；
- 在真正能证明不会失败时使用 `expect`。

### 10.4 库和应用采用不同错误策略

```text
库公共 API：明确的错误 enum
应用程序内部：anyhow::Result 或 Box<dyn Error>
```

库需要让调用者理解和处理错误；应用层更关注错误上下文、日志和最终展示。

### 10.5 为错误添加上下文

底层错误本身可能只有：

```text
No such file or directory
```

应用层应补充业务上下文：

```text
加载用户配置 /etc/myapp/config.toml 失败：No such file or directory
```

这样日志才能直接用于定位问题。

### 10.6 公共 API 说明 `Errors` 和 `Panics`

```rust
/// 加载指定路径下的配置文件。
///
/// # Errors
///
/// 当文件无法读取或内容无法解析时返回错误。
///
/// # Panics
///
/// 此函数不会主动 panic。
fn load_config(path: &str) -> Result<Config, ConfigError> {
    // ...
}
```

错误行为本身也是 API 契约的一部分。

---

## 11. 快速决策表

| 场景 | 推荐方式 |
|---|---|
| 文件、网络、数据库失败 | `Result<T, E>` |
| 用户输入不合法 | `Result<T, E>` |
| 只关心“有或没有” | `Option<T>` |
| 连续传播底层错误 | `?` |
| 需要针对错误进行恢复 | `match` / `if let` |
| 测试和示例代码 | 可使用 `unwrap` / `expect` |
| 违反内部不变量 | `panic!` / `assert!` |
| 应用层统一聚合错误 | `anyhow` / `Box<dyn Error>` |
| 库公共 API | 自定义错误枚举 / `thiserror` |

---

## 12. 最终总结

Rust 错误机制可以记住以下四条：

1. **正常失败使用 `Result<T, E>`，程序错误才使用 `panic!`。**
2. **`?` 让错误传播更加简洁，但错误路径仍然是显式的。**
3. **应用层可以统一聚合错误，库层应保留清晰、结构化的错误类型。**
4. **错误类型、错误上下文和是否可能 panic，都是 API 设计的一部分。**

从 C++ 程序员的视角看，Rust 相当于将“错误码 / `expected` 风格返回值”提升成了整个语言生态的默认错误模型，并通过类型系统、模式匹配和 `?` 运算符形成了一套完整且统一的工程实践。
