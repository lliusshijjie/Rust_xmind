# Rust `Option` / `Result` 常用方法总结

`Option<T>` 和 `Result<T, E>` 是 Rust 中处理“值可能不存在”和“操作可能失败”的核心类型。

理解它们时，不建议逐个死记 API，而应该先掌握方法的**命名规律**和**数据流方向**。

---

## 1. 核心模型

### 1.1 `Option<T>`

```rust
enum Option<T> {
    Some(T),
    None,
}
```

它表示：

- `Some(T)`：存在一个值；
- `None`：没有值，但不一定意味着发生了错误。

常见场景：

- 查找不到元素；
- 容器为空；
- 迭代结束；
- 结构体中的可选字段。

```rust
fn find_user(id: u32) -> Option<String> {
    if id == 1 {
        Some("Alice".to_string())
    } else {
        None
    }
}
```

### 1.2 `Result<T, E>`

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

它表示：

- `Ok(T)`：操作成功，并携带结果；
- `Err(E)`：操作失败，并携带错误原因。

常见场景：

- 文件 I/O；
- 数据解析；
- 网络请求；
- 数据库访问；
- 业务校验错误。

### 1.3 选择原则

```text
没有值，但无需解释原因  → Option<T>
操作失败，需要错误原因  → Result<T, E>
```

当 `None` 在当前业务语境中需要被解释为错误时，可以转换为 `Result`：

```rust
let user = users.get(&id)
    .ok_or(UserError::NotFound)?;
```

---

## 2. 方法命名规律

### 2.1 `unwrap` 系列：拆开外壳，取得内部值

| 方法 | 含义 |
|---|---|
| `unwrap()` | 强行取值，`None` / `Err` 时 panic |
| `expect(msg)` | 强行取值，panic 时输出指定信息 |
| `unwrap_or(v)` | 失败时使用现成默认值 |
| `unwrap_or_else(f)` | 失败时调用闭包生成默认值 |
| `unwrap_or_default()` | 失败时使用 `Default::default()` |

```rust
let value = Some(10).unwrap_or(0);
```

需要注意：

```rust
let value = expensive_default();
option.unwrap_or(value);
```

即使 `option` 是 `Some`，`expensive_default()` 也已经执行。

有计算成本时应使用懒求值版本：

```rust
let value = option.unwrap_or_else(expensive_default);
```

### 2.2 `map` 系列：转换内部值

`map` 不负责处理外壳，只转换 `Some` 或 `Ok` 内部的值：

```rust
let len = Some("hello").map(|s| s.len());
// Some(5)
```

```rust
let result: Result<i32, &str> = Ok(10);
let doubled = result.map(|v| v * 2);
// Ok(20)
```

主要规律：

| 方法 | 作用 |
|---|---|
| `map(f)` | 将成功值 `T` 转换为 `U`，保留外壳 |
| `map_or(default, f)` | 成功时转换，失败时返回默认值 |
| `map_or_else(default_fn, f)` | 失败分支通过闭包懒生成默认值 |
| `map_err(f)` | 只转换 `Result` 的错误类型 |

```rust
let result = "abc"
    .parse::<i32>()
    .map_err(|e| format!("解析失败: {e}"));
```

### 2.3 `and_then`：成功后继续执行可能失败的操作

当闭包本身也返回 `Option` 或 `Result` 时，使用 `and_then`。

```rust
fn parse_positive(s: &str) -> Option<i32> {
    s.parse::<i32>()
        .ok()
        .and_then(|v| (v > 0).then_some(v))
}
```

它可以避免出现嵌套结构：

```rust
Option<Option<T>>
Result<Result<T, E>, E>
```

从 C++23 的角度看，它类似 `std::optional` / `std::expected` 的 monadic `and_then`。

### 2.4 `or_else`：失败后执行补救逻辑

```rust
let value = primary_source()
    .or_else(backup_source);
```

规律是：

- `and_then`：处理成功路径；
- `or_else`：处理失败路径。

### 2.5 常见后缀

| 后缀 | 含义 |
|---|---|
| `_or` | 直接传入现成备用值，会立即求值 |
| `_or_else` | 通过闭包懒生成备用值 |
| `_default` | 使用 `Default` trait |
| `_err` | 针对 `Result` 的 `Err` 分支 |

---

## 3. `Option<T>` 常用方法

### 3.1 判断状态

```rust
option.is_some();
option.is_none();
option.is_some_and(|v| v > 0);
option.is_none_or(|v| v > 0);
```

`is_some_and` 表示：

```text
是 Some，并且内部值满足条件
```

`is_none_or` 表示：

```text
是 None，或者内部值满足条件
```

### 3.2 转换和过滤

```rust
let result = Some(10)
    .map(|v| v * 2)
    .filter(|v| *v > 15);
```

`filter` 的规律是：

```text
Some(v) 且满足条件    → Some(v)
Some(v) 但不满足条件  → None
None                  → None
```

### 3.3 为 `None` 提供备用值

```rust
option.or(Some(default_value));
option.or_else(|| load_from_backup());
```

`or` 传入现成的 `Option`，`or_else` 在需要时才执行闭包。

### 3.4 原地插入值

```rust
let mut value: Option<String> = None;

let s: &mut String = value.get_or_insert_with(|| "default".to_string());
s.push_str(" value");
```

常用方法：

| 方法 | 作用 |
|---|---|
| `get_or_insert(v)` | `None` 时插入 `v`，返回 `&mut T` |
| `get_or_insert_with(f)` | `None` 时懒生成并插入 |
| `get_or_insert_default()` | `None` 时插入默认值 |

### 3.5 转换为 `Result`

```rust
let value = option.ok_or(MyError::MissingValue)?;
```

错误构造有成本时使用：

```rust
let value = option.ok_or_else(|| {
    MyError::MissingField("username".to_string())
})?;
```

### 3.6 所有权和借用

```rust
let name: Option<String> = Some("Alice".to_string());

let length = name.as_ref().map(|s| s.len());
println!("{name:?}"); // name 没有被 move
```

常用方法：

| 方法 | 转换结果 |
|---|---|
| `as_ref()` | `Option<T>` → `Option<&T>` |
| `as_mut()` | `Option<T>` → `Option<&mut T>` |
| `as_deref()` | 对内部值执行 `Deref` 后借用 |
| `take()` | 取走内部值，原位置留下 `None` |
| `replace(v)` | 用新值替换，返回旧的 `Option<T>` |

`take()` 在状态机和结构体字段处理中非常实用：

```rust
struct Task {
    callback: Option<Box<dyn FnOnce()>>,
}

impl Task {
    fn run(&mut self) {
        if let Some(callback) = self.callback.take() {
            callback();
        }
    }
}
```

### 3.7 结构组合

```rust
Some(1).zip(Some("a"));
// Some((1, "a"))
```

```rust
let nested = Some(Some(10));
let flat = nested.flatten();
// Some(10)
```

常用方法：

| 方法 | 作用 |
|---|---|
| `zip` | 两个值都是 `Some` 时组成元组 |
| `unzip` | `Option<(A, B)>` 拆为两个 `Option` |
| `flatten` | `Option<Option<T>>` 降为 `Option<T>` |

---

## 4. `Result<T, E>` 常用方法

### 4.1 判断状态

```rust
result.is_ok();
result.is_err();
result.is_ok_and(|v| v > 0);
result.is_err_and(|e| e.is_retryable());
```

### 4.2 分别转换成功值和错误值

```rust
let result = read_number()
    .map(|v| v * 2)
    .map_err(AppError::ReadNumber);
```

规律：

- `map` 只处理 `Ok(T)`；
- `map_err` 只处理 `Err(E)`。

### 4.3 链式执行可能失败的操作

```rust
fn load_user(id: u64) -> Result<User, AppError> {
    find_user(id)
        .and_then(validate_user)
        .and_then(load_permissions)
}
```

不过在较长的业务流程中，通常使用 `?` 更直观：

```rust
fn load_user(id: u64) -> Result<User, AppError> {
    let user = find_user(id)?;
    let user = validate_user(user)?;
    let user = load_permissions(user)?;
    Ok(user)
}
```

### 4.4 错误补救

```rust
let result = request_primary_server()
    .or_else(|_| request_backup_server());
```

`or_else` 可以把旧错误类型转换成新的错误类型：

```rust
let result: Result<Data, AppError> = read_cache()
    .or_else(|_| read_database().map_err(AppError::Database));
```

### 4.5 转换为 `Option`

```rust
result.ok();  // Ok(v)  → Some(v)，Err → None
result.err(); // Err(e) → Some(e)，Ok  → None
```

这种转换会丢失另一分支的信息，因此应确认调用者确实不关心失败原因。

### 4.6 观察值但不改变结果

```rust
let result = load_config()
    .inspect(|config| println!("loaded: {config:?}"))
    .inspect_err(|e| eprintln!("load failed: {e}"));
```

适合：

- 日志；
- 调试；
- 埋点；
- 监控统计。

---

## 5. `?` 运算符

### 5.1 在返回 `Option` 的函数中

```rust
fn first_char_length(input: Option<String>) -> Option<usize> {
    let text = input?;
    let first = text.chars().next()?;
    Some(first.len_utf8())
}
```

规则：

```text
Some(v) → 取出 v，继续执行
None    → 当前函数立即返回 None
```

### 5.2 在返回 `Result` 的函数中

```rust
fn load_number(path: &str) -> Result<i32, AppError> {
    let text = std::fs::read_to_string(path)?;
    let number = text.trim().parse::<i32>()?;
    Ok(number)
}
```

规则：

```text
Ok(v)  → 取出 v，继续执行
Err(e) → 当前函数立即返回 Err(e)
```

如果底层错误与当前函数错误类型不同，`?` 会借助 `From` / `Into` 完成转换。

### 5.3 `Option` 和 `Result` 的桥接

不能直接在返回 `Result` 的函数里对普通 `Option` 使用 `?`。应先将它转换成 `Result`：

```rust
fn get_user(id: u64) -> Result<&'static User, AppError> {
    USERS.get(&id)
        .ok_or_else(|| AppError::UserNotFound(id))
}
```

或者继续使用 `?`：

```rust
let user = USERS
    .get(&id)
    .ok_or_else(|| AppError::UserNotFound(id))?;
```

这是工程代码中非常常见的模式。

---

## 6. `transpose`：交换嵌套顺序

### `Option<Result<T, E>>`

```rust
let value: Option<Result<i32, E>> = ...;
let value: Result<Option<i32>, E> = value.transpose();
```

### `Result<Option<T>, E>`

```rust
let value: Result<Option<i32>, E> = ...;
let value: Option<Result<i32, E>> = value.transpose();
```

它常用于：

```text
某个字段可以不存在；
但字段一旦存在，解析过程可能失败。
```

示例：

```rust
fn parse_port(value: Option<&str>) -> Result<Option<u16>, std::num::ParseIntError> {
    value.map(str::parse).transpose()
}
```

---

## 7. 工程最佳实践

### 7.1 业务代码少用 `unwrap`

`unwrap` 适合：

- 测试代码；
- 快速原型；
- 能严格证明不可能失败的内部不变量。

必须 panic 时，优先使用 `expect`，并说明为什么一定成功：

```rust
let addr = "127.0.0.1"
    .parse::<std::net::IpAddr>()
    .expect("hardcoded IP address must be valid");
```

### 7.2 有成本的默认值使用 `_else`

```rust
// 默认值总会构造
option.unwrap_or(build_default());

// 只有 None 时才构造
option.unwrap_or_else(build_default);
```

同理：

```rust
ok_or_else(...)
or_else(...)
map_or_else(...)
get_or_insert_with(...)
```

### 7.3 连续失败流程优先使用 `?`

简单转换适合组合器：

```rust
value.map(...).filter(...)
```

较长流程适合 `?`：

```rust
let config = read_config()?;
let config = parse_config(config)?;
validate_config(&config)?;
```

### 7.4 避免不必要的所有权移动

当只需要读取内部值时，优先考虑：

```rust
as_ref()
as_mut()
as_deref()
```

不要为了调用方法而把整个 `Option<String>` 或 `Result<String, E>` move 掉。

### 7.5 复杂分支使用 `match`

组合器并不是越多越好。

简单数据流：

```rust
option.map(...).unwrap_or_default()
```

复杂业务分支：

```rust
match result {
    Ok(value) if value.is_cached() => { /* ... */ }
    Ok(value) => { /* ... */ }
    Err(Error::Timeout) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

当链式调用开始影响可读性时，应回到 `match` 或普通控制流。

---

## 8. C++ 视角对比

| Rust | 现代 C++ |
|---|---|
| `Option<T>` | `std::optional<T>` |
| `Result<T, E>` | `std::expected<T, E>`（C++23） |
| `map` | `transform` |
| `and_then` | `and_then` |
| `or_else` | `or_else` |
| `unwrap_or` | `value_or` |
| `?` | 暂无完全对应的标准语法 |

Rust 的主要优势在于：

1. `Option` / `Result` 被标准库和生态广泛使用；
2. `?` 与语言深度集成，错误传播更简洁；
3. 类型系统会促使调用者处理失败路径；
4. 所有权与借用方法使错误处理可以避免不必要的复制和移动。

---

## 9. 快速记忆

```text
map       ：成功值做普通转换
map_err   ：错误值做普通转换
and_then  ：成功后继续执行一个可能失败的操作
or_else   ：失败后执行补救操作
unwrap_or ：失败时使用现成默认值
*_or_else ：失败时懒生成默认值
as_ref    ：借用内部值，避免 move
ok_or     ：Option 升级为 Result
ok / err  ：Result 降级为 Option，并丢弃另一分支
transpose ：交换 Option 与 Result 的嵌套顺序
?         ：失败时提前返回，成功时取出内部值
```
