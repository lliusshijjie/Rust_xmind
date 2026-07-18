# Rust 闭包 Closure 总结

## 1. 闭包的基本概念

Rust 闭包可以理解为：

> **匿名函数 + 捕获外部环境的能力。**

基本语法：

```rust
let add = |a: i32, b: i32| -> i32 {
    a + b
};
```

多数情况下可以依赖类型推断：

```rust
let add = |a, b| a + b;
```

需要注意：闭包参数和返回值类型在第一次使用时确定，之后不能再用另一组不兼容的类型调用。

```rust
let identity = |x| x;

let value = identity(10);
// let text = identity("hello"); // 错误：闭包已经被推断为 i32 -> i32
```

从底层理解，闭包并不是普通函数指针，而是编译器生成的：

```text
匿名结构体 + 保存的捕获变量 + 可调用方法
```

因此闭包可以携带状态。

---

## 2. 基本语法

### 无参数闭包

```rust
let print_hello = || println!("hello");

print_hello();
```

### 带参数闭包

```rust
let add = |a, b| a + b;

assert_eq!(add(1, 2), 3);
```

### 显式类型与代码块

```rust
let add = |a: i32, b: i32| -> i32 {
    a + b
};
```

闭包既可以保存在变量中，也可以直接传给函数：

```rust
let nums = vec![1, 2, 3];

let result: Vec<_> = nums
    .iter()
    .map(|x| x * 2)
    .collect();
```

闭包的真实类型是编译器生成的匿名类型，通常不能直接写出。API 中一般使用：

- 泛型参数：`F: Fn(...)`
- 返回类型：`impl Fn(...)`
- Trait Object：`Box<dyn Fn(...)>`

---

## 3. 捕获外部环境

Rust 会根据闭包内部如何使用外部变量，自动选择捕获方式。

### 不可变借用捕获

```rust
let name = String::from("Rust");

let print_name = || {
    println!("{name}");
};

print_name();
println!("{name}");
```

闭包只是读取 `name`，因此编译器通常只进行不可变借用。

### 可变借用捕获

```rust
let mut count = 0;

let mut increase = || {
    count += 1;
};

increase();
increase();

assert_eq!(count, 2);
```

闭包修改了外部变量，因此需要可变借用。

### 所有权捕获

```rust
let data = String::from("hello");

let consume = move || {
    println!("{data}");
};

consume();
// println!("{data}"); // data 已移动进闭包
```

捕获变量之后，闭包仍然受到 Rust 所有权、借用和生命周期规则的约束。

普通 `fn` 函数不能捕获外部环境：

```rust
fn normal_function() {
    // 无法直接捕获调用位置的局部变量
}
```

---

## 4. `Fn`、`FnMut`、`FnOnce`

Rust 使用三个 Trait 描述闭包的调用语义。

可以简化理解为：

```rust
FnOnce::call_once(self)
FnMut::call_mut(&mut self)
Fn::call(&self)
```

| Trait | 接收方式 | 调用特点 |
|---|---|---|
| `Fn` | `&self` | 只读闭包状态，可以重复调用 |
| `FnMut` | `&mut self` | 可以修改闭包状态，可以重复调用 |
| `FnOnce` | `self` | 调用可能消费闭包，只保证调用一次 |

继承关系：

```text
Fn : FnMut : FnOnce
```

也就是说：

- 实现 `Fn` 的闭包也可以当作 `FnMut` 和 `FnOnce` 使用。
- 实现 `FnMut` 的闭包也可以当作 `FnOnce` 使用。
- 只实现 `FnOnce` 的闭包不能当作 `FnMut` 或 `Fn` 使用。

编译器会根据闭包体如何使用捕获变量，自动判断其实现哪些 Trait。

---

## 5. 三种闭包示例

### `Fn`：只读捕获变量

```rust
let value = 10;

let print_value = || {
    println!("{value}");
};

print_value();
print_value();
```

闭包只读取捕获变量，因此可以重复调用。

### `FnMut`：修改捕获变量

```rust
let mut count = 0;

let mut increase = || {
    count += 1;
};

increase();
increase();
```

调用 `FnMut` 闭包需要取得闭包对象的可变引用，因此闭包变量自身通常也要声明为 `mut`。

### `FnOnce`：消费捕获变量

```rust
let text = String::from("hello");

let consume = || {
    drop(text);
};

consume();
// consume(); // 错误：捕获值已经被消费
```

调用后，闭包内部保存的 `text` 已被移出并销毁，因此闭包只能调用一次。

---

## 6. `move` 关键字

`move` 的作用是：

> 在创建闭包时，将捕获变量的所有权移动到闭包内部。

```rust
let value = String::from("hello");

let closure = move || {
    println!("{value}");
};
```

### `move` 不等于 `FnOnce`

下面的闭包虽然使用了 `move`，但仍然可以重复调用：

```rust
let value = String::from("hello");

let closure = move || {
    println!("{value}");
};

closure();
closure();
```

原因是调用时只是读取闭包内部的 `value`，并没有把它移出去。

是否是 `FnOnce`，取决于：

```text
调用闭包时，是否消费了闭包内部保存的捕获值
```

### 常见使用场景

#### 创建线程

```rust
use std::thread;

let data = vec![1, 2, 3];

let handle = thread::spawn(move || {
    println!("{data:?}");
});

handle.join().unwrap();
```

#### 异步任务

```rust
tokio::spawn(async move {
    // 使用移动进任务的数据
});
```

#### 返回闭包

```rust
fn make_adder(base: i32) -> impl Fn(i32) -> i32 {
    move |value| base + value
}
```

#### 闭包可能比当前作用域存活得更久

此时通常需要让闭包拥有所需数据，而不是借用即将销毁的局部变量。

---

## 7. 工程中的主要用途

### 迭代器链式处理

```rust
let result: Vec<_> = nums
    .iter()
    .filter(|x| **x > 0)
    .map(|x| x * 2)
    .collect();
```

常见方法包括：

- `map`
- `filter`
- `find`
- `for_each`
- `collect`

闭包用于描述“每个元素应该执行什么逻辑”。

### 回调和策略参数

```rust
fn process<F>(value: i32, strategy: F) -> i32
where
    F: FnOnce(i32) -> i32,
{
    strategy(value)
}
```

这种设计可以理解为：

```text
框架函数负责固定流程
闭包负责可变化的行为
```

类似 C++ 中把 Lambda 传给 STL 算法或回调接口。

### 惰性计算和延迟初始化

```rust
let value = option.unwrap_or_else(|| expensive_compute());
```

闭包只有在 `Option` 为 `None` 时才执行。

其他常见 API：

```rust
get_or_insert_with
or_else
```

### `Option` / `Result` 链式转换

```rust
let port = std::env::var("PORT")
    .map_err(|err| format!("读取配置失败：{err}"))
    .and_then(|value| {
        value
            .parse::<u16>()
            .map_err(|err| format!("端口格式错误：{err}"))
    });
```

常见方法：

- `map`
- `map_err`
- `and_then`
- `ok_or_else`

适用于配置解析、错误转换和数据清洗。

### 线程与异步任务

闭包用于捕获任务上下文，并将任务逻辑提交给线程池或异步运行时。

### 构造带状态的行为对象

```rust
fn make_counter() -> impl FnMut() -> usize {
    let mut count = 0;

    move || {
        count += 1;
        count
    }
}
```

闭包可以同时保存状态和行为，类似 C++ 中带捕获的 Lambda 或函数对象。

---

## 8. API 设计最佳实践

### 优先使用泛型和 `Fn` Trait

```rust
fn run<F>(task: F)
where
    F: FnOnce(),
{
    task();
}
```

与函数指针相比：

```rust
fn run(task: fn()) {
    task();
}
```

泛型 + `Fn` Trait 可以接收：

- 普通函数
- 不捕获环境的闭包
- 捕获环境的闭包

因此通常更加通用。

### Trait 约束尽量放宽

根据函数如何调用闭包选择最宽松的约束：

```text
只调用一次                 -> FnOnce
多次调用，允许修改状态      -> FnMut
多次调用，要求只读调用      -> Fn
```

原则是：

> 能用 `FnOnce` 就不要强制要求 `FnMut`；能用 `FnMut` 就不要强制要求 `Fn`。

因为约束越宽，调用者能够传入的闭包种类越多。

### 普通场景优先静态分发

```rust
fn execute<F>(task: F)
where
    F: Fn(),
{
    task();
}
```

优势：

- 泛型单态化
- 容易内联
- 通常没有动态分发开销

### 统一存储不同闭包类型时使用 Trait Object

```rust
struct Handler {
    callback: Box<dyn Fn()>,
}
```

适用场景：

- 不同闭包需要存储在同一字段或容器中。
- 闭包的具体类型在运行时决定。

代价：

- 堆分配。
- 间接调用。
- 编译器更难内联。

### 复杂闭包应抽成具名函数

不建议在 `map`、`filter` 中塞入大量分支、日志和错误处理。

```rust
fn is_valid_user(user: &User) -> bool {
    user.active && user.age >= 18
}

let result: Vec<_> = users
    .iter()
    .filter(|user| is_valid_user(user))
    .collect();
```

这样可以提高可读性，也便于独立测试。

---

## 9. 常见注意事项

### `move` 不是 `FnOnce`

`move` 决定如何捕获变量；`FnOnce` 取决于调用时是否消费捕获值。

### `FnMut` 闭包变量通常要声明为 `mut`

```rust
let mut count = 0;
let mut increase = || count += 1;

increase();
```

### 闭包参数类型推断后固定

一个闭包不能像泛型函数那样，用不同参数类型反复调用。

### 捕获可变借用会限制外部访问

```rust
let mut count = 0;

let mut increase = || count += 1;

// println!("{count}"); // 闭包仍持有可变借用时可能产生冲突
increase();
```

具体借用结束时间会受到非词法生命周期分析影响，但应明确：闭包捕获变量后，外部访问仍需遵循借用规则。

### 返回借用局部变量的闭包会产生生命周期问题

因此返回闭包时常结合 `move`，让闭包拥有所需数据。

```rust
fn make_printer(text: String) -> impl Fn() {
    move || println!("{text}")
}
```

### 存储闭包需要泛型或 Trait Object

泛型结构体：

```rust
struct Handler<F>
where
    F: Fn(),
{
    callback: F,
}
```

动态分发结构体：

```rust
struct Handler {
    callback: Box<dyn Fn()>,
}
```

### 不要为了链式写法牺牲可读性

闭包和迭代器适合表达清晰的数据处理流程；复杂控制流通常直接使用具名函数或普通循环更好。

---

## 10. 与 C++ Lambda 对比

| Rust 闭包 | C++ Lambda |
|---|---|
| `|x| x + 1` | `[](auto x) { return x + 1; }` |
| 编译器根据使用方式推断捕获 | 程序员显式编写捕获列表 |
| 捕获受所有权和借用约束 | 捕获受值语义、引用语义和对象生命周期约束 |
| 使用 `Fn` / `FnMut` / `FnOnce` 表示调用语义 | 主要通过 `operator()`、`mutable` 和对象成员语义体现 |
| `move` 将捕获值移入闭包 | 通常使用初始化捕获和 `std::move` |
| 常通过泛型和 Trait 表达 | 常通过模板、`auto` 或 `std::function` 表达 |

C++ 常见捕获形式：

```cpp
[=]   // 默认按值捕获
[&]   // 默认按引用捕获
[x]   // 按值捕获 x
[&x]  // 按引用捕获 x
```

Rust 通常不需要手动声明捕获列表，而是根据闭包内部的使用方式推断：

```text
只读访问 -> 不可变借用
修改变量 -> 可变借用
move     -> 所有权捕获
```

从 C++ 程序员角度看：

> Rust 闭包和 C++ Lambda 都是编译器生成的匿名函数对象，但 Rust 进一步用所有权系统和 `Fn` Trait 家族约束捕获与调用行为。

---

## 11. 记忆总结

```text
闭包 = 携带上下文的匿名行为对象

FnOnce：调用时取得 self，只保证调用一次
FnMut ：调用时取得 &mut self，可以修改闭包状态
Fn    ：调用时取得 &self，只读闭包状态

move 决定捕获时是否转移所有权
FnOnce 决定调用时是否消费闭包状态

API 设计：
只调用一次                 -> FnOnce
多次调用且允许修改状态      -> FnMut
多次调用且要求只读          -> Fn
```

最关键的工程理解是：

> **Rust 闭包并不只是更简洁的函数写法，而是一种同时封装行为、状态和所有权关系的零成本抽象。**
