# Rust 高级 Trait 与高级闭包

## 1. 高级 Trait

Rust 的高级 Trait 主要是在普通 Trait 基础上，解决以下问题：

- Trait 中需要关联某种类型
- 泛型参数需要默认类型
- 多个同名方法产生歧义
- 一个 Trait 依赖另一个 Trait
- 为外部类型实现外部 Trait

------

## 2. 关联类型

关联类型允许在 Trait 中声明一个类型占位符，由具体实现决定真实类型。

```rust
trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

实现 Trait 时指定关联类型：

```rust
struct Counter {
    value: u32,
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.value += 1;
        Some(self.value)
    }
}
```

这里：

```rust
type Item = u32;
```

表示这个 `Iterator` 实现产生的元素类型是 `u32`。

### 关联类型与泛型的区别

使用泛型：

```rust
trait Container<T> {
    fn get(&self) -> T;
}
```

同一个类型理论上可以多次实现：

```rust
impl Container<i32> for MyType {}
impl Container<String> for MyType {}
```

使用关联类型：

```rust
trait Container {
    type Item;

    fn get(&self) -> Self::Item;
}
```

一个类型实现该 Trait 时，只能确定一个 `Item` 类型。

### 使用原则

当一个 Trait 对于某个实现只有一种固定的相关类型时，优先使用关联类型。

典型例子：

- `Iterator::Item`
- `Deref::Target`
- `Add::Output`

------

## 3. 默认泛型类型参数

Trait 的泛型参数可以指定默认类型：

```rust
trait Add<Rhs = Self> {
    type Output;

    fn add(self, rhs: Rhs) -> Self::Output;
}
```

其中：

```rust
Rhs = Self
```

表示右操作数默认和左操作数类型相同。

例如：

```rust
use std::ops::Add;

#[derive(Debug)]
struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Point {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
```

调用：

```rust
let p1 = Point { x: 1, y: 2 };
let p2 = Point { x: 3, y: 4 };

let p3 = p1 + p2;
```

这里没有指定 `Rhs`，所以默认是 `Point`。

也可以指定不同类型：

```rust
use std::ops::Add;

struct Millimeters(u32);
struct Meters(u32);

impl Add<Meters> for Millimeters {
    type Output = Millimeters;

    fn add(self, rhs: Meters) -> Millimeters {
        Millimeters(self.0 + rhs.0 * 1000)
    }
}
```

### 主要用途

默认泛型参数常用于：

- 运算符重载
- 为已有接口增加扩展能力
- 保持常见用法简单
- 特殊场景下允许不同类型组合

------

## 4. 运算符重载

Rust 不允许任意重载运算符，只能通过实现标准库指定的 Trait 完成。

常见运算符对应关系：

| 运算符   | Trait        |
| -------- | ------------ |
| `+`      | `Add`        |
| `-`      | `Sub`        |
| `*`      | `Mul`        |
| `/`      | `Div`        |
| `==`     | `PartialEq`  |
| `<`、`>` | `PartialOrd` |
| `[]`     | `Index`      |
| `!`      | `Not`        |

例如实现 `+`：

```rust
use std::ops::Add;

#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

impl Add for Point {
    type Output = Point;

    fn add(self, rhs: Point) -> Self::Output {
        Point {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
```

使用：

```rust
let result = Point { x: 1, y: 2 } + Point { x: 3, y: 4 };

assert_eq!(result, Point { x: 4, y: 6 });
```

运算符重载应该符合运算符的直觉语义，不要让 `+` 执行删除、网络请求等无关操作。

------

## 5. 同名方法与完全限定语法

不同 Trait 可以定义同名方法，类型本身也可以定义同名方法。

```rust
trait Pilot {
    fn fly(&self);
}

trait Wizard {
    fn fly(&self);
}

struct Human;

impl Pilot for Human {
    fn fly(&self) {
        println!("飞行员驾驶飞机");
    }
}

impl Wizard for Human {
    fn fly(&self) {
        println!("巫师使用魔法飞行");
    }
}

impl Human {
    fn fly(&self) {
        println!("人类挥动双臂");
    }
}
```

调用类型自身的方法：

```rust
let person = Human;

person.fly();
```

调用指定 Trait 的方法：

```rust
Pilot::fly(&person);
Wizard::fly(&person);
```

### 关联函数的歧义

如果方法没有 `self` 参数，Rust 有时无法判断应该调用哪个实现。

```rust
trait Animal {
    fn baby_name() -> String;
}

struct Dog;

impl Dog {
    fn baby_name() -> String {
        String::from("Spot")
    }
}

impl Animal for Dog {
    fn baby_name() -> String {
        String::from("puppy")
    }
}
```

调用类型自身的关联函数：

```rust
println!("{}", Dog::baby_name());
```

调用 Trait 实现中的关联函数：

```rust
println!("{}", <Dog as Animal>::baby_name());
```

这称为完全限定语法：

```rust
<Type as Trait>::method(arguments)
```

一般只有出现名称歧义时才需要使用。

------

## 6. Supertrait

一个 Trait 可以要求实现者同时实现另一个 Trait，这种依赖关系称为 Supertrait。

```rust
use std::fmt;

trait OutlinePrint: fmt::Display {
    fn outline_print(&self) {
        let output = self.to_string();
        let len = output.len();

        println!("{}", "*".repeat(len + 4));
        println!("*{}*", " ".repeat(len + 2));
        println!("* {output} *");
        println!("*{}*", " ".repeat(len + 2));
        println!("{}", "*".repeat(len + 4));
    }
}
```

这里：

```rust
trait OutlinePrint: fmt::Display
```

表示要实现 `OutlinePrint`，必须先实现 `Display`。

```rust
struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl OutlinePrint for Point {}
```

使用：

```rust
let point = Point { x: 1, y: 2 };
point.outline_print();
```

### 常见用途

当一个 Trait 的默认方法依赖另一个 Trait 的能力时，可以使用 Supertrait。

例如：

- 输出能力依赖 `Display`
- 序列化能力依赖某个基础数据 Trait
- 业务 Trait 依赖身份、日志或验证能力

它类似于 C++ Concepts 中的约束组合，但不是传统面向对象继承。

------

## 7. Newtype 模式

Rust 存在孤儿规则：

> 只有当 Trait 或目标类型至少有一个定义在当前 crate 中时，才能实现该 Trait。

因此不能直接为外部类型实现外部 Trait：

```rust
// 不允许：Display 和 Vec 都定义在标准库中
// impl std::fmt::Display for Vec<String> {}
```

可以使用 Newtype 模式包装外部类型：

```rust
use std::fmt;

struct Wrapper(Vec<String>);

impl fmt::Display for Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}
```

使用：

```rust
let values = Wrapper(vec![
    String::from("hello"),
    String::from("world"),
]);

println!("{values}");
```

因为 `Wrapper` 是当前 crate 中定义的新类型，所以可以为它实现 `Display`。

### Newtype 的特点

Newtype 通常是只有一个字段的元组结构体：

```rust
struct UserId(u64);
struct Meters(f64);
struct Wrapper(Vec<String>);
```

它的常见用途包括：

- 绕过孤儿规则
- 增强类型安全
- 区分底层类型相同但语义不同的数据
- 隐藏内部实现
- 限制外部可以执行的操作

例如：

```rust
struct UserId(u64);
struct OrderId(u64);
```

虽然底层都是 `u64`，但它们是不同类型，无法被错误混用。

------

# 高级函数与闭包

## 8. 函数指针

普通函数可以作为参数传递。

函数指针类型使用 `fn` 表示：

```rust
fn add_one(x: i32) -> i32 {
    x + 1
}

fn calculate(f: fn(i32) -> i32, value: i32) -> i32 {
    f(value)
}
```

使用：

```rust
let result = calculate(add_one, 10);

assert_eq!(result, 11);
```

这里的：

```rust
fn(i32) -> i32
```

表示接收一个 `i32`，返回一个 `i32` 的函数指针。

------

## 9. 函数指针和闭包的关系

函数指针 `fn` 与闭包 Trait 不完全相同。

闭包通常通过以下 Trait 表示：

```rust
Fn
FnMut
FnOnce
```

普通函数也实现了这三个 Trait，因此接收闭包的函数通常也能接收普通函数。

```rust
fn apply<F>(f: F, value: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(value)
}
```

可以传入闭包：

```rust
let result = apply(|x| x * 2, 10);
```

也可以传入普通函数：

```rust
fn square(x: i32) -> i32 {
    x * x
}

let result = apply(square, 10);
```

### 一般如何选择

优先使用泛型闭包参数：

```rust
F: Fn(...)
```

因为它既可以接收闭包，也可以接收普通函数，并且通常可以静态分发和内联。

只有明确要求普通函数指针时，才使用：

```rust
fn(...)
```

函数指针常见于：

- 与 C 接口交互
- 回调函数表
- 不需要捕获环境的简单函数
- 需要固定函数指针类型的场景

------

## 10. 将函数用于迭代器

函数可以直接作为迭代器适配器的参数。

```rust
let numbers = vec![1, 2, 3];

let strings: Vec<String> = numbers
    .iter()
    .map(|value| value.to_string())
    .collect();
```

也可以传入方法：

```rust
let strings: Vec<String> = numbers
    .iter()
    .map(ToString::to_string)
    .collect();
```

对于枚举构造函数，也可以直接传递：

```rust
enum Status {
    Value(u32),
    Stop,
}

let statuses: Vec<Status> = (0..5)
    .map(Status::Value)
    .collect();
```

这里：

```rust
Status::Value
```

可以被看作一个接收 `u32` 并返回 `Status` 的函数。

------

## 11. 返回闭包

闭包的具体类型由编译器生成，程序员无法直接写出闭包类型的名称。

因此不能这样写：

```rust
// 无法直接写出闭包的具体返回类型
// fn create_closure() -> ??? {}
```

可以使用 `impl Trait` 返回闭包：

```rust
fn create_adder(value: i32) -> impl Fn(i32) -> i32 {
    move |x| x + value
}
```

使用：

```rust
let add_five = create_adder(5);

assert_eq!(add_five(10), 15);
```

这里使用 `move`，将 `value` 移入闭包，使闭包可以安全地从函数中返回。

### 使用 `impl Fn` 的特点

```rust
fn create_adder(value: i32) -> impl Fn(i32) -> i32
```

适合：

- 函数只返回一种具体闭包类型
- 不需要调用者知道闭包的真实类型
- 希望使用静态分发

------

## 12. 返回不同类型的闭包

即使两个闭包签名完全一样，它们也可能拥有不同的具体类型。

因此下面的写法通常无法通过编译：

```rust
// 两个分支中的闭包是不同类型
// fn choose(flag: bool) -> impl Fn(i32) -> i32 {
//     if flag {
//         |x| x + 1
//     } else {
//         |x| x * 2
//     }
// }
```

可以使用 Trait 对象统一类型：

```rust
fn choose(flag: bool) -> Box<dyn Fn(i32) -> i32> {
    if flag {
        Box::new(|x| x + 1)
    } else {
        Box::new(|x| x * 2)
    }
}
```

使用：

```rust
let operation = choose(true);

assert_eq!(operation(10), 11);
```

### 如何选择返回方式

只返回一种闭包：

```rust
impl Fn(...)
```

可能返回多种闭包：

```rust
Box<dyn Fn(...)>
```

两者的主要区别：

| 方式          | 特点                                     |
| ------------- | ---------------------------------------- |
| `impl Fn`     | 静态分发，性能较好，只能表示一种具体类型 |
| `Box<dyn Fn>` | 动态分发，可以统一多个不同闭包类型       |
| `fn(...)`     | 只能表示普通函数或不捕获环境的闭包       |

------

## 13. `Fn`、`FnMut`、`FnOnce` 的简单选择

闭包实现哪个 Trait，取决于它如何使用捕获的变量。

### `Fn`

只读取捕获变量，可以重复调用：

```rust
let value = 10;

let print = || {
    println!("{value}");
};

print();
print();
```

常见参数写法：

```rust
fn execute<F>(f: F)
where
    F: Fn(),
{
    f();
}
```

------

### `FnMut`

会修改捕获变量，需要通过可变闭包调用：

```rust
let mut count = 0;

let mut increment = || {
    count += 1;
};

increment();
increment();
```

函数参数：

```rust
fn execute<F>(mut f: F)
where
    F: FnMut(),
{
    f();
}
```

------

### `FnOnce`

会取走捕获变量的所有权，因此通常只能调用一次：

```rust
let text = String::from("hello");

let consume = move || {
    drop(text);
};

consume();
```

函数参数：

```rust
fn execute<F>(f: F)
where
    F: FnOnce(),
{
    f();
}
```

### 包含关系

可以简单理解为：

```text
Fn       可以当作 FnMut 和 FnOnce 使用
FnMut    可以当作 FnOnce 使用
FnOnce   至少可以调用一次
```

编写接收闭包的函数时，应该使用限制最弱、适用范围最大的 Trait：

- 只调用一次：优先 `FnOnce`
- 需要多次调用并允许修改状态：使用 `FnMut`
- 需要多次调用且不允许修改捕获状态：使用 `Fn`

------

## 14. 常见使用方式总结

### 接收一个普通回调

```rust
fn process<F>(value: i32, operation: F) -> i32
where
    F: Fn(i32) -> i32,
{
    operation(value)
}
let result = process(10, |x| x * 2);
```

------

### 接收需要修改状态的闭包

```rust
fn repeat<F>(times: usize, mut operation: F)
where
    F: FnMut(),
{
    for _ in 0..times {
        operation();
    }
}
let mut count = 0;

repeat(3, || {
    count += 1;
});

assert_eq!(count, 3);
```

------

### 返回一个闭包

```rust
fn multiplier(factor: i32) -> impl Fn(i32) -> i32 {
    move |value| value * factor
}
let double = multiplier(2);

assert_eq!(double(10), 20);
```

------

### 保存多个不同闭包

```rust
let operations: Vec<Box<dyn Fn(i32) -> i32>> = vec![
    Box::new(|x| x + 1),
    Box::new(|x| x * 2),
    Box::new(|x| x * x),
];
for operation in operations {
    println!("{}", operation(10));
}
```

因为每个闭包的具体类型不同，所以需要使用：

```rust
Box<dyn Fn(...)>
```

------

## 15. 实际使用建议

### 高级 Trait

- 类型和 Trait 存在固定关联关系时，使用关联类型。
- 运算符重载应保持符合直觉的语义。
- 出现同名方法歧义时，使用完全限定语法。
- Trait 依赖另一个 Trait 时，使用 Supertrait。
- 需要为外部类型实现外部 Trait 时，使用 Newtype。
- 不要为了抽象而过度设计复杂 Trait 层次。

### 高级闭包

- 接收回调时，通常优先使用 `Fn`、`FnMut` 或 `FnOnce` 泛型。
- 只需要普通函数指针时，使用 `fn`。
- 返回单一闭包类型时，使用 `impl Fn`。
- 返回多种不同闭包时，使用 `Box<dyn Fn>`。
- 闭包需要离开当前作用域时，通常需要使用 `move`。
- 优先选择限制最弱的闭包 Trait，避免给调用者增加不必要约束。

------

## 16. 核心记忆

```text
高级 Trait：
关联类型       → Trait 实现绑定一个具体类型
默认泛型参数   → 常见情况使用默认类型
完全限定语法   → 解决同名方法歧义
Supertrait     → 一个 Trait 依赖另一个 Trait
Newtype        → 包装外部类型，实现 Trait 或增强类型安全
高级闭包：
fn             → 普通函数指针
F: Fn          → 接收可重复调用的闭包
F: FnMut       → 接收会修改状态的闭包
F: FnOnce      → 接收可能消费捕获变量的闭包
impl Fn        → 返回一种闭包类型
Box<dyn Fn>    → 统一多种不同闭包类型
```

这部分知识不需要一开始全部掌握。

实际工程中最常见的是：

```rust
fn process<F>(f: F)
where
    F: Fn(),
{
    f();
}
```

以及：

```rust
fn create() -> impl Fn(i32) -> i32 {
    |value| value + 1
}
```

其他高级语法在遇到 Trait 冲突、接口封装或动态回调集合时再使用即可。