# Rust 宏基础总结

## 1. 如何理解 Rust 宏

Rust 宏不是 C/C++ 中简单的 `#define` 文本替换，而是：

> 在编译阶段匹配输入的 Token，并生成 Rust 代码。

因此，Rust 宏更接近：

```text
C++ 可变参数模板
+ 折叠表达式
+ 编译期代码生成
```

宏通常通过名字后的 `!` 识别：

```rust
println!("hello");
vec![1, 2, 3];
format!("value = {}", value);
```

------

## 2. 为什么需要宏

普通函数主要处理运行时的值：

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

宏则可以生成代码结构，适合：

- 接收不定数量的参数
- 自动生成重复代码
- 自动实现 Trait
- 实现特殊语法
- 在编译期检查输入

例如：

```rust
let values = vec![1, 2, 3, 4];
```

`vec!` 可以接收任意数量的参数，而普通 Rust 函数不支持这种可变参数形式。

------

## 3. Rust 宏的主要分类

Rust 宏主要分为两类：

```text
声明宏 macro_rules!
过程宏 procedural macro
```

基础阶段重点掌握宏的使用，不需要深入学习复杂实现。

------

# 4. 声明宏 `macro_rules!`

声明宏通过模式匹配输入，然后展开成对应代码。

## 4.1 最简单的宏

```rust
macro_rules! say_hello {
    () => {
        println!("hello");
    };
}

say_hello!();
```

基本结构：

```rust
macro_rules! 宏名称 {
    (匹配模式) => {
        生成的代码
    };
}
```

------

## 4.2 接收表达式

```rust
macro_rules! double {
    ($value:expr) => {
        $value * 2
    };
}

let result = double!(10);
```

其中：

```rust
$value:expr
```

表示匹配一个 Rust 表达式，并绑定到 `$value`。

------

## 4.3 常见匹配类型

基础阶段了解以下几种即可：

| 类型      | 含义                       |
| --------- | -------------------------- |
| `expr`    | 表达式                     |
| `ident`   | 标识符，例如变量名、函数名 |
| `ty`      | 类型                       |
| `tt`      | 一个 Token Tree            |
| `literal` | 字面量                     |

例如匹配变量名：

```rust
macro_rules! create_variable {
    ($name:ident, $value:expr) => {
        let $name = $value;
    };
}

create_variable!(number, 42);
```

------

## 4.4 重复匹配

Rust 宏可以处理不定数量的参数：

```rust
macro_rules! print_all {
    ($($value:expr),*) => {
        $(
            println!("{}", $value);
        )*
    };
}

print_all!(1, 2, 3);
```

其中：

```rust
$($value:expr),*
```

表示匹配零个或多个由逗号分隔的表达式。

可以类比 C++：

```cpp
template<typename... Args>
void print_all(Args&&... args) {
    (print(args), ...);
}
```

对应关系：

```text
Rust $($x:expr),*  ≈ C++ 可变参数包
Rust $()*          ≈ C++ 参数包展开
Rust 重复生成代码   ≈ C++ 折叠表达式
```

不过 Rust 宏操作的是语法 Token，而 C++ 可变参数模板主要操作类型和参数包。

------

# 5. 过程宏

过程宏比声明宏更强大，它接收 Rust Token 并输出新的 Token。

基础阶段重点会使用，不需要自己实现。

过程宏主要分为三类。

------

## 5.1 派生宏

派生宏会自动为类型实现 Trait：

```rust
#[derive(Debug, Clone, PartialEq)]
struct User {
    name: String,
    age: u32,
}
```

第三方库也经常提供派生宏：

```rust
#[derive(Serialize, Deserialize)]
struct User {
    name: String,
}
```

基础阶段需要熟练使用 `#[derive(...)]`。

------

## 5.2 属性宏

属性宏写在函数、结构体或模块上方：

```rust
#[test]
fn test_add() {
    assert_eq!(1 + 1, 2);
}
#[tokio::main]
async fn main() {
    println!("hello");
}
```

可以简单理解为：

> 编译器或框架会对下面的代码进行转换或增强。

------

## 5.3 函数式过程宏

调用方式和普通宏类似：

```rust
sql!("SELECT * FROM users");
```

常见于：

- SQL 检查
- HTML 模板
- 正则表达式
- 编译期 DSL

基础阶段知道它存在即可。

------

# 6. 常见标准库宏

建议熟悉以下宏：

```rust
println!()
format!()
vec![]
panic!()
assert!()
assert_eq!()
matches!()
dbg!()
todo!()
unimplemented!()
```

例如：

```rust
let value = 42;

println!("value = {}", value);
dbg!(value);

assert_eq!(value, 42);
```

------

# 7. 宏和函数如何选择

默认优先使用普通函数。

```text
处理运行时数据       → 普通函数
处理不同类型         → 泛型和 Trait
生成 Rust 代码结构   → 宏
```

适合使用宏的场景：

- 参数数量不固定
- 需要接收变量名或类型
- 需要生成函数、结构体或 `impl`
- 需要减少大量重复代码
- 需要自定义编译期语法

不要仅仅为了少写几行代码就使用宏，因为宏通常更难阅读和调试。

------

# 8. 使用宏的注意事项

## 8.1 注意重复求值

下面的宏会对表达式求值两次：

```rust
macro_rules! double_call {
    ($value:expr) => {
        $value + $value
    };
}
```

调用：

```rust
double_call!(get_value());
```

展开后类似：

```rust
get_value() + get_value()
```

更安全的写法：

```rust
macro_rules! double_value {
    ($value:expr) => {{
        let value = $value;
        value + value
    }};
}
```

------

## 8.2 控制变量作用域

宏中经常使用双层花括号：

```rust
macro_rules! calculate {
    ($value:expr) => {{
        let temp = $value;
        temp * 2
    }};
}
```

内部代码块可以限制临时变量的作用域，避免影响外部代码。

------

## 8.3 宏会增加阅读成本

宏调用可能只写了一行：

```rust
some_macro!(value);
```

但它可能生成大量代码。

因此：

- 不要在宏中隐藏复杂业务逻辑
- 宏应尽量短小
- 能用函数解决时优先使用函数
- 出现问题时可以查看宏展开结果

常用工具：

```bash
cargo expand
```

------

# 9. 基础阶段应该掌握到什么程度

## 必须掌握

- 能识别 `xxx!()` 是宏
- 会使用常见标准库宏
- 会使用 `#[derive(...)]`
- 会使用常见属性宏
- 能读懂简单的 `macro_rules!`
- 理解 `$value:expr`
- 理解 `$($value:expr),*`

## 建议了解

- `expr`、`ident`、`ty`、`tt`
- 声明宏类似模式匹配
- 过程宏分为派生宏、属性宏和函数式过程宏
- 宏在编译期生成代码

## 暂时不需要深入

- 自己编写过程宏
- `TokenStream`
- `syn`
- `quote`
- 复杂递归宏
- TT muncher
- 宏卫生的底层原理
- 复杂 DSL 设计

------

# 10. 核心记忆

```text
Rust 宏不是简单文本替换，
而是编译期 Token 匹配和代码生成。
macro_rules!        → 声明宏，类似模式匹配
#[derive(...)]      → 自动实现 Trait
#[attribute]        → 转换或增强代码
function_like!(...) → 函数式过程宏
```

对于普通 Rust 工程开发，只需要做到：

```text
熟练使用常见宏
会使用 derive 和属性宏
能读懂简单 macro_rules!
知道复杂过程宏暂时不必深入
```

达到这个程度，就已经足以应对大多数基础 Rust 项目。