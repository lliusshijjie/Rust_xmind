# Rust `unsafe` 机制总结

## 1. `unsafe` 是什么

Rust 的 `unsafe` 并不表示“关闭所有安全检查”，而是：

> 编译器无法证明某些操作一定安全，因此由程序员承担安全性证明责任。

进入 `unsafe` 后，Rust 仍然会检查：

- 所有权和移动规则
- 生命周期和借用规则
- 类型是否匹配
- Trait Bound 是否成立
- 变量是否初始化

`unsafe` 只是允许程序执行少数编译器无法验证的底层操作。

需要特别注意：

> `unsafe` 不会让未定义行为变得合法。

如果 `unsafe` 代码产生悬垂指针、数据竞争、越界访问、重复释放等问题，程序仍然可能产生 Undefined Behavior。

------

## 2. 为什么 Rust 需要 `unsafe`

Rust 的类型系统虽然能够保证绝大多数内存安全问题，但它无法覆盖所有底层编程场景。

常见原因包括：

- 操作系统和硬件本身不理解 Rust 所有权
- C/C++ 接口只提供裸指针
- 编译器无法证明复杂指针关系是安全的
- 底层容器需要直接管理内存
- 无锁数据结构需要精确控制并发和内存
- 驱动、内核、嵌入式程序需要访问硬件地址

如果 Rust 完全禁止不安全操作，那么很多基础设施都无法实现，例如：

- `Vec`
- `Box`
- `Arc`
- `Mutex`
- 内存分配器
- 操作系统内核
- C/C++ FFI
- SIMD 和底层并发原语

Rust 的设计思路不是彻底消灭不安全操作，而是：

```text
少量经过审查的 unsafe 实现
            ↓
维护明确的安全不变量
            ↓
向外提供无法被错误使用的安全 API
```

对于 C++ 程序员，可以这样理解：

```text
C++：
程序员默认承担大部分内存安全责任。

Rust：
编译器承担大部分内存安全责任，
只有 unsafe 部分由程序员承担证明责任。
```

------

## 3. `unsafe` 可以执行哪些操作

Rust 中主要有五类不安全操作。

### 3.1 解引用裸指针

Rust 中存在两种裸指针：

```rust
*const T // 类似 C++ const T*
*mut T   // 类似 C++ T*
```

创建裸指针不需要 `unsafe`：

```rust
let value = 42;
let ptr = &value as *const i32;
```

解引用裸指针需要 `unsafe`：

```rust
let result = unsafe {
    *ptr
};
```

原因是编译器无法确认：

- 指针是否为空
- 指针是否悬垂
- 地址是否正确对齐
- 指向的对象是否已经初始化
- 内存是否仍然有效
- 是否违反 Rust 的引用别名规则

------

### 3.2 调用 `unsafe fn`

```rust
unsafe fn read_value(ptr: *const i32) -> i32 {
    unsafe {
        *ptr
    }
}
```

调用时需要显式进入 `unsafe`：

```rust
let value = 10;

let result = unsafe {
    read_value(&value)
};
```

`unsafe fn` 的真正含义是：

> 调用者必须满足该函数规定的安全前置条件。

因此，公开的 `unsafe fn` 必须使用 `# Safety` 文档说明安全契约：

```rust
/// 从裸指针中读取一个 i32。
///
/// # Safety
///
/// 调用者必须保证：
///
/// - `ptr` 非空；
/// - `ptr` 正确对齐；
/// - `ptr` 指向已经初始化的 `i32`；
/// - 调用期间该内存保持有效。
pub unsafe fn read(ptr: *const i32) -> i32 {
    unsafe {
        ptr.read()
    }
}
```

在现代 Rust 中，即使函数本身是 `unsafe fn`，函数内部的危险操作也推荐使用独立的 `unsafe {}`。

这样可以明确区分：

- 函数的安全前置条件
- 函数内部真正执行的不安全操作

------

### 3.3 访问可变静态变量

```rust
static mut COUNTER: usize = 0;

unsafe {
    COUNTER += 1;
}
```

`static mut` 非常危险，因为多个线程可能同时访问它，产生数据竞争。

工程中通常应优先改为：

```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn increment() {
    COUNTER.fetch_add(1, Ordering::Relaxed);
}
```

也可以使用：

- `Mutex`
- `RwLock`
- `OnceLock`
- `LazyLock`
- 原子类型

普通业务代码中通常不应该使用 `static mut`。

------

### 3.4 实现 `unsafe trait`

```rust
unsafe trait TrustedBuffer {
    fn ptr(&self) -> *const u8;
}
```

实现时也必须显式标记：

```rust
struct Buffer(Vec<u8>);

unsafe impl TrustedBuffer for Buffer {
    fn ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}
```

`unsafe trait` 表示：

> 其他安全代码会依赖该 Trait 实现所保证的安全不变量。

最典型的例子是：

```rust
unsafe impl Send for MyType {}
unsafe impl Sync for MyType {}
```

错误实现 `Send` 或 `Sync`，可能导致：

- 数据竞争
- 跨线程访问无效内存
- 线程不安全对象被错误共享
- Safe Rust 代码产生未定义行为

因此不要为了通过编译而手动实现 `Send` 或 `Sync`。

------

### 3.5 访问 `union` 字段

```rust
union Number {
    integer: i32,
    float: f32,
}

let number = Number { integer: 42 };

let value = unsafe {
    number.integer
};
```

Rust 无法确认 `union` 当前有效的是哪个字段，因此读取字段需要 `unsafe`。

`union` 常见于：

- C FFI
- 操作系统接口
- 二进制协议
- 特殊内存布局
- 底层类型转换

普通业务代码通常使用 `enum` 更安全。

------

## 4. `unsafe` 的核心工程模式

使用 `unsafe` 的正确方式不是让整个程序充满裸指针，而是：

> 用少量 `unsafe` 实现底层能力，再向外提供安全接口。

例如：

```rust
pub fn get_value(values: &[i32], index: usize) -> Option<i32> {
    if index >= values.len() {
        return None;
    }

    // SAFETY:
    // index 已经过边界检查，因此小于 values.len()；
    // ptr 来自当前有效的 slice；
    // slice 中的元素已经初始化并正确对齐。
    Some(unsafe {
        *values.as_ptr().add(index)
    })
}
```

调用方只需要使用安全接口：

```rust
let values = [10, 20, 30];

assert_eq!(get_value(&values, 1), Some(20));
assert_eq!(get_value(&values, 10), None);
```

调用者不需要理解内部裸指针，也无法通过正常调用触发未定义行为。

### Sound Abstraction

如果一个安全 API 无法通过正常调用触发未定义行为，那么它可以称为一个合理的安全抽象。

相反，如果一个普通安全函数可以被正常调用，却导致 Undefined Behavior，那么这个封装就是 `unsound` 的。

------

## 5. 工程中什么时候会使用 `unsafe`

### 5.1 FFI：调用 C/C++ 接口

这是业务工程中最常见的 `unsafe` 场景。

```rust
unsafe extern "C" {
    fn native_api(data: *const u8, len: usize) -> i32;
}
```

常见应用包括：

- 调用 C/C++ 动态库
- Windows API
- Linux 系统库
- CUDA
- ONNX Runtime
- OpenSSL
- 数据库原生客户端
- 自定义 C++ 算法库

在 FFI 边界需要确认：

- ABI 是否匹配
- 参数类型是否匹配
- 是否需要 `#[repr(C)]`
- 指针和长度是否对应
- 字符串是否以 `\0` 结尾
- 字符串编码是否一致
- 内存由谁分配
- 内存由谁释放
- 使用哪个分配器释放
- 回调函数能存活多久
- panic 是否可能跨越 FFI 边界

------

### 5.2 实现底层容器

例如实现：

- 自定义 `Vec`
- 环形缓冲区
- Arena
- 对象池
- Intrusive List
- 自定义智能指针
- 自定义分配器

这些结构可能需要手动维护：

- 指针地址
- 长度与容量
- 初始化状态
- 析构次数
- 内存布局
- 所有权转移

------

### 5.3 操作系统、驱动和嵌入式

常见场景包括：

- MMIO
- 设备寄存器
- 中断处理
- 内核数据结构
- 系统调用封装
- 内联汇编
- 裸机开发

这些场景本身就脱离了普通 Rust 引用和所有权模型。

------

### 5.4 高性能底层优化

例如：

- SIMD
- 手动向量化
- 避免重复边界检查
- 自定义内存布局
- 无锁数据结构
- 批量内存操作

但不要仅仅因为“感觉 `unsafe` 更快”就使用它。

正确流程应是：

```text
先实现安全版本
    ↓
进行性能测试
    ↓
定位真实瓶颈
    ↓
确认安全 API 无法满足要求
    ↓
局部引入 unsafe
    ↓
重新进行基准测试和正确性验证
```

------

## 6. `unsafe` 中需要重点防范的问题

### 6.1 悬垂指针

```rust
let ptr = {
    let value = 42;
    &value as *const i32
};

// value 已经被销毁
// unsafe { *ptr }; // Undefined Behavior
```

裸指针不会携带生命周期约束，因此需要程序员保证指向对象仍然存活。

------

### 6.2 越界访问

```rust
let values = [1, 2, 3];
let ptr = values.as_ptr();

unsafe {
    let value = *ptr.add(10); // Undefined Behavior
}
```

裸指针运算不会自动进行边界检查。

------

### 6.3 未对齐访问

某些指针操作要求地址符合类型的对齐要求。

```rust
let ptr: *const u32 = /* ... */;
```

读取前必须确认地址满足 `u32` 的对齐约束。

对于非对齐数据，应考虑：

```rust
std::ptr::read_unaligned
```

但仍然需要保证地址有效并且内存中存放合法的值。

------

### 6.4 未初始化内存

不能将任意未初始化内存直接当成某个类型使用。

通常需要使用：

```rust
std::mem::MaybeUninit<T>
```

例如：

```rust
use std::mem::MaybeUninit;

let mut value = MaybeUninit::<i32>::uninit();

value.write(42);

let value = unsafe {
    value.assume_init()
};
```

调用 `assume_init()` 前，必须保证对象已经正确初始化。

------

### 6.5 违反引用别名规则

Rust 对引用有比裸指针更严格的语义：

- `&T` 表示共享只读访问
- `&mut T` 表示独占访问

不能因为地址相同就随意创建冲突引用。

```rust
let reference: &mut i32 = unsafe {
    &mut *ptr
};
```

创建 `&mut T` 时，必须保证对应范围内不存在其他正在使用的引用。

------

### 6.6 重复释放

以下 API 会重新接管裸指针的所有权：

```rust
Box::from_raw(ptr);
Vec::from_raw_parts(ptr, len, capacity);
CString::from_raw(ptr);
```

例如：

```rust
let boxed = Box::new(42);
let ptr = Box::into_raw(boxed);

let boxed = unsafe {
    Box::from_raw(ptr)
};
```

`Box::from_raw(ptr)` 只能执行一次。

如果对同一个指针调用两次，两个 `Box` 都认为自己拥有该内存，最终可能造成 double free。

------

### 6.7 错误恢复 `Vec` 所有权

```rust
Vec::from_raw_parts(ptr, len, capacity)
```

使用时必须保证：

- `ptr` 来自兼容的内存分配器
- 内存按照 `T` 的布局和对齐方式分配
- 前 `len` 个元素已经初始化
- `len <= capacity`
- 分配空间足够容纳 `capacity` 个元素
- 当前没有其他对象拥有这段内存

任何一个条件不满足，都可能产生未定义行为。

------

### 6.8 数据竞争

Rust 中数据竞争属于 Undefined Behavior。

例如：

- 多线程同时写普通变量
- 一个线程读，另一个线程写
- 无同步访问 `static mut`
- 错误实现 `Send` 或 `Sync`
- 跨线程共享线程不安全的 FFI 对象

并发代码应优先使用：

- `Arc`
- `Mutex`
- `RwLock`
- 原子类型
- Channel
- 已验证的并发容器

------

## 7. 安全不变量

`unsafe` 代码的核心不是语法，而是维护安全不变量。

例如自定义动态数组可能存在以下不变量：

```text
1. ptr 始终指向一块有效分配；
2. 分配空间能够容纳 capacity 个 T；
3. 前 len 个元素已经正确初始化；
4. len 始终小于或等于 capacity；
5. 每个初始化元素只析构一次；
6. 当前结构拥有这块内存；
7. 结构存活期间，内存不会被其他对象释放。
```

所有可能修改该结构状态的方法，都必须维护这些条件。

一旦某个函数破坏了不变量，错误可能不会立即出现，而是在之后某次析构、扩容或访问时才表现出来。

------

## 8. 工程最佳实践

### 8.1 优先使用安全 API

一般优先级为：

```text
标准库安全 API
    >
成熟 crate 提供的安全封装
    >
自己封装 unsafe
    >
业务代码中直接散布 unsafe
```

大多数 Web 服务、业务逻辑和普通工具程序都不需要直接使用 `unsafe`。

------

### 8.2 缩小 `unsafe` 块范围

不推荐：

```rust
unsafe {
    check_argument();
    update_state();
    calculate_result();
    *ptr = 10;
    write_log();
}
```

推荐：

```rust
check_argument();
update_state();
calculate_result();

// SAFETY: ptr 已验证为有效、可写且正确对齐。
unsafe {
    *ptr = 10;
}

write_log();
```

`unsafe` 范围越小，越容易审查其安全前提。

------

### 8.3 为每个 `unsafe` 块添加 `SAFETY` 注释

```rust
// SAFETY:
// - index 已经过边界检查；
// - ptr 来自当前 slice；
// - slice 在读取期间保持有效；
// - 指针正确对齐；
// - 指向的元素已经初始化。
unsafe {
    *ptr.add(index)
}
```

注释应该回答：

> 为什么这里是安全的？

而不是简单描述代码做了什么。

------

### 8.4 为 `unsafe fn` 编写安全契约

```rust
/// 从指定位置读取元素。
///
/// # Safety
///
/// 调用者必须保证：
///
/// - `ptr` 对读取有效；
/// - 地址正确对齐；
/// - 指向的对象已经初始化；
/// - 调用期间内存不会失效；
/// - 不违反 Rust 的别名规则。
pub unsafe fn read<T>(ptr: *const T) -> T {
    unsafe {
        ptr.read()
    }
}
```

安全契约应明确、完整，并且能够被代码审查。

------

### 8.5 不要让调用者承担不必要的责任

如果函数可以在内部检查条件，就应该提供安全接口：

```rust
pub fn get(values: &[i32], index: usize) -> Option<&i32> {
    values.get(index)
}
```

而不是直接暴露：

```rust
pub unsafe fn get_unchecked(
    values: &[i32],
    index: usize,
) -> &i32 {
    unsafe {
        values.get_unchecked(index)
    }
}
```

只有确实无法在内部验证时，才应将函数设计为 `unsafe fn`。

------

### 8.6 避免随意实现 `Send` 和 `Sync`

不要为了消除编译错误而写：

```rust
unsafe impl Send for MyType {}
unsafe impl Sync for MyType {}
```

在实现前必须确认：

- 类型是否允许跨线程移动
- 内部裸指针指向的对象是否线程安全
- 是否存在内部可变性
- 内部可变性是否有同步保护
- 析构是否可能发生在线程不安全的上下文
- 底层 C/C++ 对象是否允许跨线程使用

------

### 8.7 不要使用 `unsafe` 绕过借用检查器

当代码无法通过借用检查时，首先应检查设计是否存在问题，例如：

- 所有权划分不清晰
- 生命周期设计错误
- 同时持有过多可变状态
- 数据结构不适合当前访问模式
- 应使用索引、句柄或 ID
- 应使用 `RefCell`、`Mutex` 等安全内部可变性工具

`unsafe` 不应该只是“让编译器闭嘴”的工具。

------

## 9. 测试和验证工具

### 9.1 Clippy

```bash
cargo clippy
```

可以在项目中启用：

```rust
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::missing_safety_doc)]
```

用于检查：

- `unsafe` 块是否缺少安全说明
- `unsafe fn` 是否缺少 `# Safety` 文档
- 常见不规范代码

------

### 9.2 Miri

```bash
cargo +nightly miri test
```

Miri 可以发现部分问题，例如：

- 越界访问
- use-after-free
- 非法对齐
- 无效值
- 部分别名规则错误
- 部分未初始化内存访问

但 Miri 不能证明代码绝对安全。

------

### 9.3 Sanitizer

常见工具包括：

- AddressSanitizer
- ThreadSanitizer
- MemorySanitizer
- LeakSanitizer

它们可以辅助发现：

- 越界访问
- use-after-free
- 数据竞争
- 未初始化内存
- 内存泄漏

------

### 9.4 Fuzz Testing

对于解析器、协议处理和 FFI 边界，可以使用模糊测试生成大量异常输入。

重点验证：

- 长度和指针组合
- 空指针
- 极端边界值
- 非法编码
- 异常内存布局
- 重复释放路径

------

### 9.5 并发状态验证

复杂并发组件可以使用 Loom 等工具，枚举不同的线程调度和原子操作顺序。

适合验证：

- 自定义锁
- 无锁队列
- Channel
- 原子状态机
- 引用计数结构

------

## 10. 代码审查清单

审查 `unsafe` 代码时，可以依次确认：

### 指针

- 指针是否可能为空？
- 指针是否可能悬垂？
- 指针是否正确对齐？
- 指针是否位于有效分配范围内？
- 指针运算是否可能越界？

### 初始化

- 内存是否已经初始化？
- 位模式对目标类型是否合法？
- 是否提前调用了 `assume_init()`？

### 生命周期

- 指向对象是否比指针使用时间更长？
- FFI 回调中保存的指针是否仍然有效？
- 是否从短生命周期数据创建了长生命周期引用？

### 别名

- 是否同时存在冲突的 `&mut T`？
- 是否在存在共享引用时进行非法写入？
- 是否从裸指针创建了不符合要求的引用？

### 所有权

- 谁拥有这段内存？
- 谁负责释放？
- 是否可能重复接管所有权？
- 分配和释放是否使用相同的分配器？
- 是否可能重复析构？

### 并发

- 是否存在无同步读写？
- 类型是否真的满足 `Send` 和 `Sync`？
- 原子内存序是否正确？
- 底层 FFI 类型是否线程安全？

### FFI

- ABI 是否一致？
- `repr(C)` 是否正确？
- 字符串编码是否一致？
- 长度单位是字节还是元素？
- panic 是否可能跨越 FFI 边界？
- 回调函数及其上下文是否保持有效？

------

## 11. 重点结论

`unsafe` 不是 Rust 安全机制的漏洞，而是 Rust 安全模型的重要组成部分。

它的目标是：

> 将无法由编译器证明的操作，集中在少量可以被人工审查的区域中。

可以将工程原则浓缩为：

```text
能不用就不用；
必须用时集中封装；
明确安全不变量；
缩小 unsafe 范围；
记录安全证明；
向外提供安全接口；
通过测试和工具进行辅助验证。
```

最终应形成这样的代码结构：

```text
业务逻辑：Safe Rust
        ↓
安全封装层：检查参数和状态
        ↓
极小的 unsafe 实现
        ↓
裸指针、FFI、操作系统或硬件
```

对于大多数 Rust 开发者而言，不需要经常编写 `unsafe`。

但需要具备阅读和审查 `unsafe` 的能力，因为标准库、异步运行时、FFI、底层容器以及高性能组件都可能依赖它。