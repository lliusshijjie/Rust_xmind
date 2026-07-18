# Rust 生命周期 Lifetime 总结

## 1. 核心理解

生命周期（Lifetime）本质上是**引用的有效范围**。

它主要解决的是 **non-owning access（非拥有访问）** 的有效性问题：当一个变量只是借用别人的数据时，Rust 需要保证这个引用不会在被引用对象释放后继续使用。

从 C++ 角度理解：

| Rust | C++ 类比 |
|---|---|
| `&T` | `const T&` / `const T*` |
| `&mut T` | 独占可变引用 / `T*` |
| `&str` | `std::string_view` |
| `&[T]` | `std::span<T>` |

生命周期标注的核心不是“延长对象生命”，而是**描述引用之间的存活关系**。它是编译期检查，运行时零成本。

---

## 2. 基础规则

### 2.1 每个引用都有生命周期

```rust
let r = &x;
```

编译器内部会认为它类似于：

```rust
let r: &'a T = &x;
```

只是大多数情况下生命周期可以自动推导，不需要手写。

### 2.2 引用不能比被引用对象活得更久

错误示例：

```rust
let r;

{
    let x = 10;
    r = &x;
}

println!("{}", r); // x 已经释放，r 悬垂
```

Rust 会在编译期拒绝这种代码。

### 2.3 生命周期约束的是借用关系，不是所有权本身

拥有型数据：

```rust
let s = String::from("hello");
```

`s` 自己拥有堆内存，释放时机由 ownership 和 `Drop` 决定。

借用型数据：

```rust
let r = &s;
```

`r` 不拥有数据，所以需要生命周期来保证它不会悬垂。

### 2.4 借用规则

Rust 的引用同时受生命周期和借用规则约束：

```text
多个不可变引用，或者一个可变引用。
```

也就是：

```rust
let r1 = &s;
let r2 = &s;      // ok，多个只读引用

let r3 = &mut s;  // 如果 r1/r2 后续还要用，则不允许
```

### 2.5 NLL：非词法生命周期

Rust 现在采用 NLL（Non-Lexical Lifetimes）。

生命周期通常结束于**最后一次使用**，而不是机械地等于 `{}` 代码块范围。

```rust
let mut s = String::from("hello");

let r = &s;
println!("{}", r); // r 最后一次使用

let m = &mut s;     // ok
m.push_str(" world");
```

---

## 3. 什么时候需要显式生命周期标注

### 3.1 函数返回引用且来自输入参数

```rust
fn first<'a>(s: &'a str) -> &'a str {
    &s[0..1]
}
```

含义：返回值不能比 `s` 活得更久。

不过这个例子可以省略成：

```rust
fn first(s: &str) -> &str {
    &s[0..1]
}
```

因为它满足生命周期省略规则。

### 3.2 多个输入引用，返回其中之一

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
```

含义：返回值的有效期不能超过 `x` 和 `y` 共同可用的范围，近似理解为取较短者。

如果不写生命周期：

```rust
fn longest(x: &str, y: &str) -> &str {
    if x.len() > y.len() { x } else { y }
}
```

编译器无法判断返回值来自 `x` 还是 `y`，因此会报错。

### 3.3 结构体字段保存引用

```rust
struct User<'a> {
    name: &'a str,
}
```

含义：`User` 不能比 `name` 指向的数据活得更久。

示例：

```rust
let name = String::from("Alice");
let user = User { name: &name };
println!("{}", user.name);
```

错误示例：

```rust
let user;

{
    let name = String::from("Alice");
    user = User { name: &name };
}

println!("{}", user.name); // name 已经释放
```

### 3.4 `impl` 带生命周期的结构体

```rust
struct User<'a> {
    name: &'a str,
}

impl<'a> User<'a> {
    fn name(&self) -> &str {
        self.name
    }
}
```

`impl<'a> User<'a>` 表示：为所有生命周期 `'a` 的 `User<'a>` 实现方法。

### 3.5 泛型约束中的生命周期

```rust
T: 'a
```

表示：`T` 内部包含的引用至少能活过 `'a`。

```rust
T: 'static
```

常见于线程、异步任务、全局保存等场景，表示类型内部不包含短生命周期引用，能够被长期持有。

---

## 4. 生命周期省略规则 Elision

Rust 不要求所有生命周期都显式写出来。常见省略规则有三条。

### 4.1 规则一：每个输入引用参数获得独立生命周期

```rust
fn foo(x: &i32, y: &i32)
```

等价于：

```rust
fn foo<'a, 'b>(x: &'a i32, y: &'b i32)
```

### 4.2 规则二：只有一个输入引用时，输出引用默认与它相同

```rust
fn first(s: &str) -> &str
```

等价于：

```rust
fn first<'a>(s: &'a str) -> &'a str
```

### 4.3 规则三：方法有 `&self` / `&mut self` 时，输出引用默认来自 `self`

```rust
impl User {
    fn name(&self) -> &str {
        &self.name
    }
}
```

等价于：

```rust
impl User {
    fn name<'a>(&'a self) -> &'a str {
        &self.name
    }
}
```

### 4.4 无法省略的典型场景

1. 多个输入引用，同时返回一个引用。
2. 结构体字段保存引用。
3. 编译器无法根据规则判断返回引用来自哪里。

工程记忆：

```text
能省略就省略；
报错时再显式表达引用之间的关系。
```

---

## 5. `'static` 的理解

### 5.1 `&'static T`

表示这个引用指向的数据可以活到程序结束。

最典型例子：

```rust
let s: &'static str = "hello";
```

字符串字面量通常存放在程序的只读数据区，因此可以认为整个程序运行期间都有效。

### 5.2 `T: 'static`

这不一定表示对象本身永远不释放，而是表示：

```text
这个类型内部不包含短生命周期引用。
```

常见场景：

```rust
std::thread::spawn(|| {
    println!("hello");
});
```

线程可能比当前函数活得更久，因此 `spawn` 通常要求闭包满足 `'static`。

`tokio::spawn` 等异步任务也经常有类似要求。

### 5.3 常见误区

```text
'static 不等于变量一定永远不释放。
```

例如一个 `String` 可以满足 `T: 'static`，但它仍然会在所有权结束时被释放。

---

## 6. 生命周期与所有权系统的关系

Rust 的内存安全不是只靠生命周期，而是由多套机制共同完成。

| 机制 | 作用 |
|---|---|
| Ownership | 决定资源归谁释放 |
| Move | 防止 double free / use after move |
| Borrow | 允许临时访问资源 |
| Lifetime | 保证借用不会悬垂 |
| Borrow Rules | 限制别名与可变访问 |
| Drop | 确定析构时机 |
| Unsafe Boundary | 将裸指针风险隔离到 unsafe 中 |

### 6.1 Owned 类型靠所有权管理

例如：

```rust
String
Vec<T>
Box<T>
```

这些类型拥有数据，生命周期主要由所有权和 `Drop` 管理。

### 6.2 Borrowed 类型靠生命周期管理

例如：

```rust
&str
&[T]
&T
&mut T
```

这些类型不拥有数据，所以需要 lifetime 保证有效性。

### 6.3 裸指针仍然可能悬垂

Rust 中也有类似 C++ 裸指针的类型：

```rust
*const T
*mut T
```

它们不受 borrow checker 的完整保护。创建裸指针可以是 safe 的，但解引用必须在 `unsafe` 中进行。

```rust
let p: *const i32;

{
    let x = 10;
    p = &x as *const i32;
}

unsafe {
    println!("{}", *p); // UB，p 已经悬垂
}
```

结论：

```text
safe Rust 防止悬垂引用；
unsafe Rust 中仍然可能产生悬垂指针。
```

---

## 7. 最佳实践

### 7.1 函数参数优先用引用

临时读取数据时，优先使用：

```rust
&str
&[T]
&T
```

例如：

```rust
fn print_name(name: &str) {
    println!("{}", name);
}
```

这样既可以接收 `String`，也可以接收字符串字面量：

```rust
let s = String::from("Alice");
print_name(&s);
print_name("Bob");
```

### 7.2 新构造的数据，返回拥有型类型

错误设计：

```rust
fn make_name() -> &str {
    let s = String::from("Alice");
    &s
}
```

正确设计：

```rust
fn make_name() -> String {
    String::from("Alice")
}
```

### 7.3 结构体长期持有数据，优先持有 owned data

优先：

```rust
struct User {
    name: String,
}
```

谨慎：

```rust
struct User<'a> {
    name: &'a str,
}
```

带生命周期的结构体会让调用方也被生命周期约束，工程组合成本更高。

### 7.4 返回引用时明确来源

常见合理设计：

```rust
struct User {
    name: String,
}

impl User {
    fn name(&self) -> &str {
        &self.name
    }
}
```

返回引用来自 `self`，关系非常清晰。

### 7.5 生命周期标注过复杂时，优先重新设计所有权模型

如果一个函数或结构体出现大量生命周期参数，例如：

```rust
struct Context<'a, 'b, 'c> { ... }
```

需要警惕：可能是所有权设计不合理。

常见改法：

```text
返回 owned data；
让结构体拥有数据；
使用 Arc<T> 共享所有权；
缩短引用保存时间；
拆分结构体职责。
```

### 7.6 异步/线程场景避免短引用跨任务保存

异步任务和线程通常要求数据满足 `'static`，因此不要轻易把短生命周期引用放进任务中。

常见做法：

```rust
use std::sync::Arc;

let data = Arc::new(String::from("hello"));
let cloned = Arc::clone(&data);

std::thread::spawn(move || {
    println!("{}", cloned);
});
```

### 7.7 unsafe 边界内可以用裸指针，但要封装成安全 API

工程上可以在底层使用裸指针优化或对接 FFI，但应该把不安全逻辑封装在小范围内，对外暴露 safe API。

---

## 8. C++ 程序员的理解方式

C++ 中，很多引用/指针有效性问题主要靠程序员经验维护：

```text
这个指针是否已经 free？
这个引用是否还有效？
string_view 指向的数据是否还活着？
vector 扩容后引用/迭代器是否失效？
多线程读写是否有 data race？
```

Rust 把其中很大一部分规则提升成语言机制：

```text
生命周期：引用不能比对象活得更久；
借用规则：多个读可以，一个写独占；
所有权：资源有明确 owner；
move：移动后原变量不能再用；
Send/Sync：线程间共享必须满足类型约束。
```

所以生命周期不是孤立概念，而是 Rust 所有权系统对“非拥有访问”的补充。

---

## 9. 常见误区

### 误区 1：`'a` 会延长对象生命

错误。

生命周期标注只描述关系，不改变对象释放时机。

### 误区 2：所有引用都要手写生命周期

错误。

大多数生命周期都可以由编译器自动推导。

### 误区 3：生命周期只防悬垂引用

不完整。

生命周期还表达 API 契约：返回引用到底借用了哪个输入，结构体内部引用依赖哪个外部对象。

### 误区 4：用了 `Rc` / `Arc` 就不用考虑生命周期

不完全正确。

`Rc` / `Arc` 管理共享所有权，但引用、借用、并发边界仍然要考虑。

### 误区 5：遇到 lifetime 报错就加 `'static`

非常危险。

很多 lifetime 报错本质是所有权设计问题。盲目加 `'static` 往往只是掩盖问题，甚至导致不必要的 clone、泄漏或架构复杂化。

---

## 10. 生命周期问题排查思路

遇到生命周期报错时，可以按下面顺序分析。

### 第一步：谁拥有数据？谁只是借用？

```text
String / Vec / Box / Arc：通常是拥有或共享拥有；
&str / &[T] / &T：只是借用。
```

### 第二步：返回引用来自哪里？

返回引用通常只能来自：

```text
输入参数；
self；
全局/static 数据。
```

不能返回局部临时变量的引用。

### 第三步：引用是否跨越了 owner 的 Drop 点？

如果 owner 已经释放，引用必然无效。

### 第四步：不可变引用存在时，是否又可变修改原对象？

典型例子：

```rust
let mut v = vec![1, 2, 3];
let x = &v[0];
v.push(4);       // 如果 x 后面还要用，这里不允许
println!("{}", x);
```

Rust 会阻止类似 C++ 中 `vector` 扩容后引用失效的问题。

### 第五步：复杂时改为 owned data

如果生命周期关系很难表达，通常可以考虑：

```text
返回 String / Vec；
结构体持有 String / Vec；
使用 Arc<T>；
缩短借用范围；
减少跨层保存引用。
```

---

## 11. 总结口诀

```text
生命周期 = 引用的有效范围；
标注描述关系，不延长生命；
拥有型资源靠 ownership；
借用型访问靠 lifetime；
函数临时读数据，用引用；
函数新造数据，返回 owned；
结构体长期保存，优先 owned；
多个输入返回引用，常要显式标注；
遇到复杂 lifetime，先反思所有权设计。
```
