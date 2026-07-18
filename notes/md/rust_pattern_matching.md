# Rust 模式匹配总结：语法、场景、最佳实践

本文根据思维导图内容整理，重点总结 Rust 模式匹配的常见语法、使用场景、工程最佳实践以及所有权相关注意事项。

---

## 1. 核心理解：模式匹配不只是结构化绑定

Rust 的模式匹配本质上是在做四件事：

```text
判断数据形态 + 解构数据 + 绑定变量 + 控制流程
```

它可以统一完成：

1. 判断 `enum` / `Option` / `Result` 当前是哪种形态；
2. 把内部数据解构出来；
3. 绑定成局部变量；
4. 根据不同分支进入不同控制流。

从 C++ 程序员视角看，它大致可以类比为：

```text
structured binding + switch + std::variant visit + optional 判断 + 错误处理
```

C++ 的结构化绑定主要负责“拆对象”：

```cpp
auto [x, y] = pair;
```

Rust 的模式匹配不只是拆对象，还可以分发控制流：

```rust
match value {
    Some(x) => handle(x),
    None => fallback(),
}
```

所以 Rust 模式匹配更接近一种**类型驱动的控制流机制**。

---

## 2. 常见语法与代码形式

### 2.1 `match`：完整分支处理

`match` 是最核心的模式匹配语法，适合处理 `Option`、`Result`、业务 `enum` 和状态机。

```rust
let x: Option<i32> = Some(10);

match x {
    Some(v) => println!("value = {v}"),
    None => println!("none"),
}
```

适合场景：

- `Option` / `Result` 多分支处理；
- `enum` 状态机；
- 协议消息分发；
- 业务事件处理；
- 错误分支需要本地恢复、降级或日志处理。

工程上推荐显式列出所有业务分支：

```rust
match state {
    State::Init => init(),
    State::Running => run(),
    State::Stopped => stop(),
}
```

少写这种形式：

```rust
match state {
    State::Init => init(),
    _ => {}
}
```

原因是 `_` 会吞掉未来新增分支，削弱编译器的穷尽性检查价值。

---

### 2.2 `if let`：只关心一种情况

`if let` 适合只处理一种模式，不想写完整 `match` 的场景。

```rust
let x = Some(3);

if let Some(v) = x {
    println!("v = {v}");
}
```

它等价于：

```rust
match x {
    Some(v) => println!("v = {v}"),
    _ => {}
}
```

适合场景：

- 只关心 `Some`，`None` 不重要；
- 只关心 `Ok`，`Err` 可以忽略或在别处处理；
- 局部小逻辑，不希望写完整 `match`。

---

### 2.3 `let else`：失败提前返回，主流程左对齐

`let else` 适合做前置校验：匹配成功继续向下，匹配失败直接退出。

```rust
let Some(user_id) = req.user_id else {
    return Err(Error::MissingUserId);
};

handle_user(user_id)?;
```

适合场景：

- 参数校验；
- 请求解析；
- CLI 参数解析；
- Web handler 中提取必要字段；
- 必须匹配成功，否则 `return` / `break` / `continue`。

核心价值是：**失败路径提前退出，主逻辑保持平铺，不形成多层嵌套**。

---

### 2.4 `while let`：循环消费某种模式

`while let` 适合不断从容器、队列、栈或 channel 中取值。

```rust
let mut stack = vec![1, 2, 3];

while let Some(x) = stack.pop() {
    println!("{x}");
}
```

channel 场景中也很常见：

```rust
while let Ok(msg) = receiver.recv() {
    handle(msg);
}
```

含义是：只要还能匹配到 `Ok(msg)` 就继续处理；一旦 `recv()` 返回 `Err`，通常表示 channel 关闭，循环结束。

---

### 2.5 `let` 解构：tuple / struct 普通绑定

普通 `let` 也可以使用模式完成解构。

#### tuple 解构

```rust
let pair = (1, "hello");
let (id, name) = pair;
```

#### struct 解构

```rust
struct Point {
    x: i32,
    y: i32,
}

let p = Point { x: 1, y: 2 };
let Point { x, y } = p;
```

如果只关心部分字段，可以用 `..` 忽略剩余字段：

```rust
let Point { x, .. } = p;
```

`..` 常用于大型结构体，只取少数字段。

---

### 2.6 `for` 解构：遍历集合时直接拆元素

遍历元素本身是复合结构时，可以直接在 `for` 中解构。

```rust
let users = vec![(1, "Alice"), (2, "Bob")];

for (id, name) in users {
    println!("{id}: {name}");
}
```

遍历 `HashMap` 时也常用：

```rust
for (key, value) in &map {
    println!("{key} = {value}");
}
```

这是工程中最轻量、最常见的模式匹配用法之一，可以减少 `item.0`、`item.1` 这类样板代码。

---

### 2.7 `matches!`：只判断是否匹配，不取值

`matches!` 用于把模式匹配变成一个布尔判断。

```rust
let ok = matches!(result, Ok(_));
let is_ready = matches!(state, State::Ready);
```

配合迭代器：

```rust
let n = tasks
    .iter()
    .filter(|t| matches!(t, Task::Download { .. }))
    .count();
```

测试中也常见：

```rust
assert!(matches!(result, Err(MyError::InvalidInput)));
```

适合场景：

- `filter`；
- `assert`；
- 状态判断；
- 不需要拿出内部值，只需要 `true` / `false`。

---

## 3. 组合模式

### 3.1 `|`：多个模式走同一逻辑

```rust
match code {
    200 | 201 | 204 => println!("success"),
    400..=499 => println!("client error"),
    500..=599 => println!("server error"),
    _ => println!("other"),
}
```

适合多个值需要相同处理逻辑的场景。

---

### 3.2 `..=`：闭区间范围匹配

```rust
match age {
    0..=17 => println!("minor"),
    18..=64 => println!("adult"),
    _ => println!("senior"),
}
```

Rust 中范围匹配常用闭区间 `..=`。

---

### 3.3 `if guard`：模式之外追加条件

```rust
match user {
    Some(u) if u.age >= 18 => println!("adult"),
    Some(_) => println!("minor"),
    None => println!("no user"),
}
```

适合“模式匹配不够，还需要额外条件判断”的场景。

注意：带 `if guard` 的分支，编译器不会认为它一定覆盖所有情况。如果所有分支都依赖 guard，通常还需要兜底分支。

---

### 3.4 `@`：匹配同时保留原值

```rust
match n {
    v @ 1..=10 => println!("in range: {v}"),
    _ => println!("out"),
}
```

适合既要判断范围，又要拿到原始值的场景。

---

### 3.5 `_`：忽略不关心的值

```rust
match tuple {
    (1, _, z) => println!("z = {z}"),
    _ => println!("other"),
}
```

注意：

```text
_  ：完全忽略，不绑定变量
_x ：绑定到变量 _x，只是不产生 unused variable 警告
```

对非 `Copy` 类型，`_x` 仍然可能触发所有权移动。

---

## 4. 工程应用场景

### 4.1 enum 状态机 / 业务事件 / 协议消息

Rust 工程中非常推荐把业务状态设计成 `enum`，再用 `match` 做分发。

```rust
enum Event {
    Login { user_id: u64 },
    Logout { user_id: u64 },
    Message { from: u64, text: String },
}

fn handle_event(event: Event) {
    match event {
        Event::Login { user_id } => handle_login(user_id),
        Event::Logout { user_id } => handle_logout(user_id),
        Event::Message { from, text } => handle_message(from, text),
    }
}
```

这种写法的优势是：新增 `enum` 变体时，编译器可以提醒哪些地方还没处理。

---

### 4.2 `Option`：查找 / 缓存 / 可选配置

`Option<T>` 表示“没有值是正常业务情况”。

典型场景：

- `HashMap::get()` 查不到 key；
- 缓存未命中；
- 可选配置项不存在；
- 请求中某个字段可选。

```rust
let Some(user) = cache.get(&user_id) else {
    return Err(Error::UserNotFound);
};
```

典型 API：

```rust
HashMap::get() -> Option<&V>
```

---

### 4.3 `Result`：错误处理 / 降级 / 恢复

`Result<T, E>` 表示“失败是需要解释的错误情况”。

普通错误传播时优先用 `?`：

```rust
let content = std::fs::read_to_string(path)?;
```

需要针对错误分支做本地处理时，再用 `match`：

```rust
match std::fs::read_to_string(path) {
    Ok(content) => parse(content),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => use_default(),
    Err(e) => return Err(e.into()),
}
```

工程选择：

```text
只是向上传播错误：用 ?
需要降级 / 重试 / 日志 / 恢复：用 match
```

---

### 4.4 Web / CLI 参数校验：`let else`

```rust
fn handle(req: Request) -> Result<Response, Error> {
    let Some(token) = req.token else {
        return Err(Error::Unauthorized);
    };

    let user = auth(token)?;
    Ok(build_response(user))
}
```

`let else` 可以让失败路径提前返回，使主流程保持清晰。

---

### 4.5 并发消费：`while let` + channel

```rust
while let Ok(task) = rx.recv() {
    worker.handle(task);
}
```

适合：

- `std::sync::mpsc`；
- `tokio::sync::mpsc`；
- 任务队列；
- 事件循环。

---

### 4.6 测试与过滤：`matches!`

```rust
assert!(matches!(result, Err(MyError::InvalidInput)));
```

```rust
let download_count = tasks
    .iter()
    .filter(|t| matches!(t, Task::Download { .. }))
    .count();
```

`matches!` 非常适合只判断类型或状态，而不关心内部数据的场景。

---

## 5. 最佳实践

### 5.1 用 `enum + match` 表达业务状态，少用多个 bool

不推荐：

```rust
struct Conn {
    connected: bool,
    closed: bool,
    reconnecting: bool,
}
```

多个 bool 容易组合出非法状态。

推荐：

```rust
enum ConnState {
    Connected,
    Closed,
    Reconnecting,
}
```

然后用 `match` 处理状态：

```rust
match state {
    ConnState::Connected => handle_connected(),
    ConnState::Closed => handle_closed(),
    ConnState::Reconnecting => handle_reconnecting(),
}
```

核心思想是：**让非法状态无法表达**。

---

### 5.2 `Result` 传播优先用 `?`，本地分支处理再用 `match`

推荐优先级：

1. 只是向上传播错误：用 `?`；
2. 需要补充上下文：用 `map_err` 或 `anyhow::Context`；
3. 需要降级、重试、打日志、恢复：用 `match`。

不要为了使用 `match`，把简单错误传播写得很重。

---

### 5.3 分清 `Option` 和 `Result` 的语义

```text
Option<T>   ：没有值是正常业务情况
Result<T,E> ：失败是需要解释的错误情况
```

例子：

```text
HashMap::get 找不到 key：Option
File::open 打不开文件：Result
```

---

### 5.4 `match` 负责分发，分支不要太长

不推荐在每个 `match` 分支中写几十行复杂逻辑。

推荐：

```rust
match event {
    Event::Login(user) => handle_login(user),
    Event::Logout(user) => handle_logout(user),
}
```

`match` 保持为 dispatch 层，复杂业务逻辑下沉到独立函数中。

---

### 5.5 少用 `_` 吞掉业务 enum 分支

对业务 `enum`，尽量显式列出所有分支。

只有以下情况适合用 `_`：

- 真正不关心剩余情况；
- 外部 crate 的 `#[non_exhaustive]` enum；
- 日志、指标、兜底逻辑。

否则 `_` 会削弱编译器对新增分支的提醒。

---

### 5.6 业务代码避免滥用 `unwrap` / `expect`

demo 或测试中可以使用 `unwrap`。

业务代码更推荐：

```rust
let user = get_user(id).ok_or(Error::UserNotFound)?;
```

或者：

```rust
let Some(user) = get_user(id) else {
    return Err(Error::UserNotFound);
};
```

---

### 5.7 优先让 enum 变体携带需要的数据

推荐：

```rust
enum Command {
    Move { x: i32, y: i32 },
    Write(String),
    Quit,
}
```

这样 `match` 分支中可以直接解构出业务数据，而不是再通过外部状态查询。

---

## 6. 注意事项与常见坑

### 6.1 所有权：`match value` 可能发生 move

```rust
let msg = Some(String::from("hello"));

match msg {
    Some(s) => println!("{s}"),
    None => {}
}

// msg 已经被移动，后面不能再用
```

如果后面还要使用原值，应匹配引用：

```rust
match &msg {
    Some(s) => println!("{s}"),
    None => {}
}
```

或者：

```rust
match msg.as_ref() {
    Some(s) => println!("{s}"),
    None => {}
}
```

记忆方式：

```text
match value      ：可能拿走所有权
match &value     ：只读借用
match &mut value ：可变借用
as_ref()         ：Option<T> / Result<T,E> 转成内部引用
as_mut()         ：转成内部可变引用
```

---

### 6.2 可变修改：用 `match &mut value` 或 `as_mut()`

```rust
let mut opt = Some(String::from("hello"));

match opt.as_mut() {
    Some(s) => s.push_str(" world"),
    None => {}
}

println!("{opt:?}");
```

`as_mut()` 可以在不移动 `Option` 本身的情况下，拿到内部值的可变引用。

---

### 6.3 `_` 和 `_name` 不一样

```text
_  ：完全忽略，不绑定
_x ：绑定变量，只是不产生 unused variable 警告
```

示例：

```rust
let s = Some(String::from("hi"));

if let Some(_x) = s {
    // String 被 move 到 _x
}
```

对非 `Copy` 类型，`_x` 仍然可能造成所有权移动。

---

### 6.4 refutable / irrefutable pattern

普通 `let` 只能使用一定能匹配成功的模式，也就是 irrefutable pattern。

可以这样写：

```rust
let (a, b) = (1, 2);
```

不能直接这样写：

```rust
let Some(x) = opt;
```

因为 `Some(x)` 可能匹配失败，是 refutable pattern。

正确写法：

```rust
let Some(x) = opt else {
    return;
};
```

或者：

```rust
if let Some(x) = opt {
    use_value(x);
}
```

---

### 6.5 `match guard` 不参与穷尽性保证

```rust
match x {
    Some(v) if v > 0 => println!("positive"),
    Some(_) => println!("non-positive"),
    None => println!("none"),
}
```

带 `if guard` 的分支，编译器不会认为它一定覆盖所有情况。所以如果所有分支都依赖 guard，通常还需要兜底分支。

---

### 6.6 分支顺序：具体在前，宽泛在后

推荐：

```rust
match code {
    200 => ok(),
    400..=499 => client_error(),
    500..=599 => server_error(),
    _ => other(),
}
```

如果 `_` 或宽泛范围写得太靠前，后面的分支会不可达。

---

## 7. 选型口诀

```text
完整状态分发：match
只关心一种情况：if let
失败提前返回：let else
循环消费数据：while let
遍历时拆数据：for 解构
只判断不取值：matches!
错误简单传播：?
不想移动所有权：match &x / as_ref() / as_mut()
```

最终工程原则：

```text
把业务状态设计成 enum，
把状态分发交给 match，
把异常路径交给 Result，
把可空值交给 Option，
把复杂分支中的长逻辑下沉到函数。
```

---

## 8. 快速复习表

| 用法 | 适合场景 | 关键点 |
|---|---|---|
| `match` | enum / Option / Result 多分支 | 完整、穷尽、适合状态分发 |
| `if let` | 只关心一种模式 | 避免写空的 `_ => {}` |
| `let else` | 前置校验、失败提前返回 | 保持主流程左对齐 |
| `while let` | 循环消费队列、栈、channel | 匹配成功继续循环 |
| `for` 解构 | 遍历 tuple、map、复合 item | 减少样板代码 |
| `matches!` | bool 判断、filter、assert | 只判断，不取内部值 |
| `|` | 多个模式同一处理 | 合并重复分支 |
| `..=` | 范围匹配 | 常用于数字区间 |
| `if guard` | 模式外再加条件 | 不参与穷尽性保证 |
| `@` | 匹配并保留原值 | 范围判断时常用 |
| `_` | 忽略值或兜底 | 业务 enum 中慎用 |

