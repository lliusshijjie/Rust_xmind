# Rust 智能指针、Deref 与 Drop 总结

## 1. 核心理解：智能指针服务于所有权模型

Rust 的智能指针并不只是“自动释放内存的指针”。它们本质上是对不同所有权关系、访问权限、可变性和线程安全需求的类型化表达。

Rust 的基础规则是：

- 一个值同一时间只有一个 owner。
- owner 离开作用域时，值会被自动释放。
- 所有权发生 move 后，原绑定失效。
- 可以有多个不可变借用 `&T`。
- 或者只能有一个可变借用 `&mut T`。
- 引用不能比被引用对象活得更久。

因此，Rust 的智能指针不是绕开所有权系统，而是在不同工程场景下更精确地表达：

- 谁拥有数据？
- 是否允许多个 owner？
- 是否允许修改？
- 是否可以跨线程共享？
- 是否需要引用计数？
- 是否可能产生循环引用？

相比 C++，Rust 的智能指针拆分得更细，是因为 Rust 不只关心资源释放，还关心访问权限和数据竞争。

---

## 2. 为什么 Rust 智能指针比 C++ 更细

C++ 常见智能指针主要有：

- `std::unique_ptr<T>`：独占所有权。
- `std::shared_ptr<T>`：共享所有权。
- `std::weak_ptr<T>`：弱引用，用于打破循环引用。

Rust 中对应关系大致如下：

| C++ | Rust | 核心作用 |
|---|---|---|
| `unique_ptr<T>` | `Box<T>` | 独占堆所有权 |
| `shared_ptr<T>` | `Rc<T>` | 单线程共享所有权 |
| `shared_ptr<T>` | `Arc<T>` | 多线程共享所有权 |
| `weak_ptr<T>` | `Weak<T>` | 非拥有引用，打破循环引用 |
| 无直接对应 | `RefCell<T>` | 单线程运行时借用检查 |
| 无直接对应 | `Cell<T>` | 简单 Copy 类型的内部可变性 |
| `shared_ptr<T> + mutex` | `Arc<Mutex<T>>` | 多线程共享可变状态 |
| 读写锁组合 | `Arc<RwLock<T>>` | 多读单写共享状态 |

Rust 拆分智能指针的主要维度有：

- 所有权：独占、共享、弱引用。
- 可变性：不可变访问、内部可变性。
- 线程模型：单线程、多线程。
- 检查时机：编译期检查、运行期检查。
- 释放策略：RAII、引用计数。

---

## 3. `Box<T>`：独占堆所有权

`Box<T>` 最接近 C++ 的 `std::unique_ptr<T>`。

它表示：

- 数据 `T` 存放在堆上。
- 栈上保存一个拥有该数据的指针。
- 同一时间只有一个 owner。
- owner 离开作用域时自动释放堆内存。

基本用法：

```rust
let x = Box::new(10);
println!("{}", *x);
```

所有权移动：

```rust
let x = Box::new(10);
let y = x;
// x 已经失效，不能再使用
```

常见场景：

1. 递归类型。

```rust
enum List {
    Cons(i32, Box<List>),
    Nil,
}
```

如果没有 `Box`，递归类型的大小无法在编译期确定。

2. Trait Object。

```rust
let obj: Box<dyn SomeTrait> = Box::new(SomeType {});
```

3. 大对象放到堆上，减少栈压力。

注意事项：

- `Box<T>` 不代表共享所有权。
- `Box<T>` 仍然遵守 Rust 的 move 规则。
- 没有 GC，释放仍然由 owner 的 `Drop` 触发。

---

## 4. `Rc<T>`：单线程共享所有权

`Rc<T>` 是 Reference Counted，表示引用计数智能指针。

它类似 C++ 的 `std::shared_ptr<T>`，但是只适合单线程。

基本用法：

```rust
use std::rc::Rc;

let a = Rc::new(String::from("hello"));
let b = Rc::clone(&a);

println!("strong count = {}", Rc::strong_count(&a));
```

`Rc<T>` 的特点：

- 允许多个 owner 共享同一份数据。
- 每次 `Rc::clone` 增加强引用计数。
- 最后一个强引用离开作用域时释放数据。
- 默认只提供共享不可变访问。

和所有权规则的关系：

`Rc<T>` 打破了“一个值只能有一个 owner”的简单模型，但是没有打破借用规则。它允许多个 owner，但不允许你直接获得 `&mut T` 修改内部值。

也就是说，`Rc<T>` 更接近：

```text
单线程 shared ownership + 默认只读访问
```

注意事项：

- `Rc<T>` 不是线程安全的。
- `Rc<T>` 不能跨线程传递。
- `Rc<T>` 循环引用会造成内存泄漏。
- 遇到父子、双向链表、图结构时，要考虑使用 `Weak<T>` 打破引用环。

---

## 5. `Arc<T>`：多线程共享所有权

`Arc<T>` 是 Atomic Reference Counted，可以理解为线程安全版 `Rc<T>`。

基本用法：

```rust
use std::sync::Arc;
use std::thread;

let data = Arc::new(String::from("hello"));
let data2 = Arc::clone(&data);

thread::spawn(move || {
    println!("{}", data2);
});
```

`Arc<T>` 的特点：

- 多线程可共享所有权。
- 引用计数操作是原子的。
- 最后一个强引用释放时销毁对象。

需要注意：

`Arc<T>` 只解决“多个线程拥有同一份数据”的问题，不解决“多个线程同时修改数据”的问题。

如果需要多线程共享可变状态，通常使用：

```rust
use std::sync::{Arc, Mutex};

let data = Arc::new(Mutex::new(0));
```

含义是：

- `Arc` 负责共享所有权。
- `Mutex` 负责互斥可变访问。

选择规则：

- 单线程共享：优先 `Rc<T>`。
- 多线程共享：使用 `Arc<T>`。
- 多线程共享可变：使用 `Arc<Mutex<T>>` 或 `Arc<RwLock<T>>`。

---

## 6. `RefCell<T>`：运行时借用检查

`RefCell<T>` 提供内部可变性，即 Interior Mutability。

它的核心作用是：

```text
把借用规则从编译期推迟到运行期检查。
```

基本用法：

```rust
use std::cell::RefCell;

let x = RefCell::new(10);

let r = x.borrow();        // 不可变借用
// let w = x.borrow_mut(); // 如果 r 还活着，这里会 panic
```

可变借用：

```rust
let x = RefCell::new(10);

{
    let mut w = x.borrow_mut();
    *w += 1;
}
```

`RefCell<T>` 仍然遵守借用规则：

- 可以有多个 `borrow()`。
- 只能有一个 `borrow_mut()`。
- `borrow()` 和 `borrow_mut()` 不能同时存在。
- 违规时不是编译错误，而是运行时 panic。

常见组合：

```rust
use std::cell::RefCell;
use std::rc::Rc;

let data = Rc::new(RefCell::new(10));
```

`Rc<RefCell<T>>` 的含义是：

- `Rc` 负责多个 owner。
- `RefCell` 负责运行时可变借用。

适合场景：

- 单线程树结构。
- 图结构。
- 回调场景。
- 需要共享状态但编译器难以静态证明借用关系的场景。

注意事项：

- 不要滥用 `Rc<RefCell<T>>`。
- 它降低了编译期安全性，换来建模灵活性。
- `borrow()` / `borrow_mut()` 返回的 guard 生命周期要尽量短。

---

## 7. `Cell<T>`：简单值的内部可变性

`Cell<T>` 也是内部可变性工具，但它更适合简单的 `Copy` 类型。

基本用法：

```rust
use std::cell::Cell;

let x = Cell::new(1);
x.set(2);
let v = x.get();
```

`Cell<T>` 和 `RefCell<T>` 的区别：

| 类型 | 适用对象 | 使用方式 | 检查方式 |
|---|---|---|---|
| `Cell<T>` | 简单 Copy 类型 | `get` / `set` 整体替换 | 不产生借用 guard |
| `RefCell<T>` | 复杂对象 | `borrow` / `borrow_mut` | 运行时借用检查 |

注意：

- `Cell<T>` 和 `RefCell<T>` 都只适合单线程内部可变性。
- 多线程场景应使用 `Mutex<T>`、`RwLock<T>` 或原子类型。

---

## 8. `Mutex<T>` / `RwLock<T>`：多线程共享可变状态

如果多个线程需要共享并修改同一份数据，需要使用锁。

常见组合：

```rust
use std::sync::{Arc, Mutex};

let data = Arc::new(Mutex::new(0));
```

含义：

- `Arc<T>`：让多个线程拥有同一份数据。
- `Mutex<T>`：保证同一时间只有一个线程可以可变访问。

基本用法：

```rust
let data = Arc::new(Mutex::new(0));

{
    let mut guard = data.lock().unwrap();
    *guard += 1;
} // guard 离开作用域，自动解锁
```

`RwLock<T>` 适合多读少写场景：

```rust
use std::sync::RwLock;

let lock = RwLock::new(10);
let r = lock.read().unwrap();
```

区别：

| 类型 | 语义 | 适用场景 |
|---|---|---|
| `Mutex<T>` | 同一时间只有一个访问者 | 读写都需要独占 |
| `RwLock<T>` | 多读单写 | 读多写少 |

工程注意事项：

- 锁持有时间要短。
- 不要在持锁期间做耗时 IO。
- 注意死锁和锁顺序。
- 注意 panic poisoning。
- `MutexGuard` / `RwLockGuard` 依靠 `Drop` 自动释放锁。

---

## 9. `Weak<T>`：非拥有引用，打破循环引用

`Weak<T>` 类似 C++ 的 `std::weak_ptr<T>`。

它表示：

- 不拥有对象。
- 不增加强引用计数。
- 不阻止对象释放。
- 使用前需要尝试升级为 `Rc<T>` 或 `Arc<T>`。

基本用法：

```rust
use std::rc::{Rc, Weak};

let rc = Rc::new(10);
let weak: Weak<i32> = Rc::downgrade(&rc);

if let Some(value) = weak.upgrade() {
    println!("{}", value);
}
```

`upgrade()` 返回：

```rust
Option<Rc<T>>
```

如果对象还活着，返回 `Some(Rc<T>)`；如果对象已经释放，返回 `None`。

典型设计：

```text
父节点拥有子节点：Rc<Node>
子节点指向父节点：Weak<Node>
```

这样可以避免父子节点互相强引用导致引用计数永远不归零。

注意事项：

- `Rc<T>` / `Arc<T>` 的循环引用不会自动回收。
- 双向关系中必须区分 owning 和 non-owning。
- 不要把所有关系都设计成强引用。

---

## 10. `Deref`：让智能指针像引用一样使用

`Deref` 的作用是定义一个类型如何被解引用。

核心定义：

```rust
use std::ops::Deref;

trait Deref {
    type Target;
    fn deref(&self) -> &Self::Target;
}
```

它决定：

```text
*p 得到什么类型的引用。
```

例如：

```rust
let x = Box::new(10);
println!("{}", *x);
```

这里 `*x` 背后依赖 `Box<T>` 对 `Deref` 的实现。

从 C++ 角度看：

```text
Deref ≈ operator* / operator->
```

### Deref coercion

`Deref` 最重要的工程价值是自动解引用转换。

例如：

```rust
fn print_str(s: &str) {
    println!("{}", s);
}

let s = String::from("hello");
print_str(&s);
```

虽然 `&s` 是 `&String`，但函数需要 `&str`，Rust 会自动完成：

```text
&String -> &str
```

对于更复杂的情况：

```text
&Box<String> -> &String -> &str
```

这就是为什么很多智能指针可以像普通引用一样使用。

### `DerefMut`

如果需要支持可变解引用，需要实现 `DerefMut`：

```rust
use std::ops::DerefMut;

trait DerefMut: Deref {
    fn deref_mut(&mut self) -> &mut Self::Target;
}
```

例如：

```rust
let mut x = Box::new(10);
*x += 1;
```

### 最佳实践

- 只为真正像指针或透明包装器的类型实现 `Deref`。
- 不要为了省字段访问而滥用 `Deref`。
- `Deref` 会让 API 行为变隐式，滥用会降低可读性。
- 普通业务类型更推荐显式提供方法。

---

## 11. `Drop`：Rust 的 RAII 和资源释放机制

`Drop` 类似 C++ 析构函数。

核心定义：

```rust
trait Drop {
    fn drop(&mut self);
}
```

当值离开作用域时，Rust 会自动调用 `drop`。

示例：

```rust
struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        println!("release resource");
    }
}

fn main() {
    let g = Guard;
} // 这里自动调用 drop
```

从 C++ 角度看：

```text
Drop ≈ destructor
```

### Drop 和所有权

Rust 的所有权保证：

- owner 离开作用域时触发 `Drop`。
- move 后只有新 owner 负责 `Drop`。
- 同一份资源不会被释放两次。

例如：

```rust
let a = String::from("hello");
let b = a;
// a 失效，最后只有 b 负责释放资源
```

### 提前释放：`std::mem::drop`

不能直接调用：

```rust
// obj.drop(); // 不允许
```

如果需要提前释放，应使用：

```rust
drop(obj);
```

`drop(obj)` 会消费对象所有权，之后不能再使用原变量。

### Drop 顺序

- 局部变量按声明顺序的反序释放。
- 结构体会先执行自身 `drop`，再释放字段。
- 工程中不要让复杂逻辑强依赖隐晦的释放顺序。

### 注意事项

- 实现 `Drop` 的类型不能同时实现 `Copy`。
- `Drop` 中不要 panic，尤其析构期间二次 panic 可能导致 abort。
- 文件、锁、内存、socket、系统句柄都适合通过 `Drop` 管理。

---

## 12. 智能指针与 `Deref` / `Drop` 的关系

以 `Box<T>` 为例：

- `Deref`：让 `Box<T>` 可以像 `&T` 一样访问内部对象。
- `Drop`：让 `Box<T>` 离开作用域时释放堆内存。

以 `Rc<T>` 为例：

- `Deref`：让 `Rc<T>` 可以读取内部 `T`。
- `Drop`：让 `Rc<T>` 离开作用域时减少强引用计数；如果强引用计数为 0，则释放对象。

以 `MutexGuard<T>` 为例：

- `Deref` / `DerefMut`：让 guard 可以像 `&T` / `&mut T` 一样访问数据。
- `Drop`：guard 离开作用域时自动解锁。

所以可以总结为：

```text
Deref 解决“怎么访问内部 T”
Drop 解决“什么时候释放资源”
```

---

## 13. 工程选择规则

优先级应该是：先用最简单的所有权模型，只有必要时再引入更复杂的智能指针。

推荐选择顺序：

1. 默认使用普通所有权 `T`。
2. 只是需要借用数据：使用 `&T` 或 `&mut T`。
3. 需要堆分配或递归类型：使用 `Box<T>`。
4. 单线程共享所有权：使用 `Rc<T>`。
5. 多线程共享所有权：使用 `Arc<T>`。
6. 单线程共享可变状态：使用 `Rc<RefCell<T>>`。
7. 多线程共享可变状态：使用 `Arc<Mutex<T>>` 或 `Arc<RwLock<T>>`。
8. 存在父子关系、双向关系、图结构：使用强引用配合 `Weak<T>`。

需要警惕的信号：

- 大量 `Rc<RefCell<T>>`：可能说明所有权关系设计过于复杂。
- 大量 `Arc<Mutex<T>>`：可能说明共享状态过多。
- 到处 clone `Arc`：可能说明数据流向不清晰。
- 锁粒度过大：可能造成性能问题和死锁风险。

更推荐的工程思路：

- 谁创建，谁拥有。
- 谁使用，谁借用。
- 能用不可变数据，就不要共享可变数据。
- 能用所有权转移，就不要共享所有权。
- 多线程场景下优先考虑消息传递，而不是到处 `Arc<Mutex<T>>`。

---

## 14. 最终总结

Rust 智能指针的核心不是“指针”，而是“所有权建模”。

可以这样记：

| 类型 | 一句话理解 |
|---|---|
| `Box<T>` | 独占堆所有权 |
| `Rc<T>` | 单线程共享所有权 |
| `Arc<T>` | 多线程共享所有权 |
| `RefCell<T>` | 单线程运行时借用检查 |
| `Cell<T>` | 简单值的内部可变性 |
| `Mutex<T>` | 多线程互斥可变访问 |
| `RwLock<T>` | 多线程多读单写访问 |
| `Weak<T>` | 非拥有引用，打破循环引用 |
| `Deref` | 决定怎么像引用一样访问内部值 |
| `Drop` | 决定 owner 结束时如何释放资源 |

从 C++ 视角看：

```text
unique_ptr<T>     ≈ Box<T>
shared_ptr<T>     ≈ Rc<T> / Arc<T>
weak_ptr<T>       ≈ Weak<T>
析构函数           ≈ Drop
operator* / ->    ≈ Deref
shared_ptr + lock ≈ Arc<Mutex<T>>
```

但 Rust 比 C++ 更严格，因为 Rust 试图把这些问题都放进类型系统中：

- 谁拥有？
- 谁能读？
- 谁能写？
- 能不能跨线程？
- 什么时候释放？
- 会不会循环引用？

这就是 Rust 智能指针被拆分成多个类型的根本原因。
