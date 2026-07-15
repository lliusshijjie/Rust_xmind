# Rust 迭代器 Iterator 总结

## 1. 核心理解

Rust 的迭代器可以理解为一个**不断产出元素的状态机**。

其核心是 `Iterator` trait：

```rust
pub trait Iterator {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
}
```

- `Item`：迭代器产出的元素类型。
- `Some(item)`：成功产出一个元素。
- `None`：迭代结束。
- `next(&mut self)`：调用会改变迭代器内部状态，使其向前推进。

例如：

```rust
let nums = vec![1, 2, 3];
let mut iter = nums.iter();

assert_eq!(iter.next(), Some(&1));
assert_eq!(iter.next(), Some(&2));
assert_eq!(iter.next(), Some(&3));
assert_eq!(iter.next(), None);
```

---

## 2. 与 C++ Iterator 的区别

C++ Iterator 更接近一种**泛化指针或游标**：

```cpp
for (auto it = nums.begin(); it != nums.end(); ++it) {
    std::cout << *it << '\n';
}
```

其核心操作是：

- `++it`：移动位置。
- `*it`：访问元素。
- `it != end`：判断是否结束。

Rust Iterator 的核心模型则是：

```text
next() -> Option<Item>
```

因此可以简单理解为：

| C++ Iterator | Rust Iterator |
|---|---|
| 更像可移动的指针或位置 | 更像不断产出元素的状态机 |
| 使用 `begin/end` 表示范围 | 使用 `Some/None` 表示结束 |
| 重点是位置、移动和解引用 | 重点是数据流和元素产出 |
| 失效规则主要由程序员保证 | 所有权和借用规则由编译器检查 |

Rust 并不是完全没有类似 C++ 游标的概念，只是标准迭代器接口更倾向于描述“下一个元素是什么”。

---

## 3. 三种主要迭代方式

### 3.1 `iter()`：不可变借用

```rust
let nums = vec![1, 2, 3];

for num in nums.iter() {
    println!("{num}");
}

println!("{nums:?}");
```

其元素类型为：

```rust
&T
```

特点：

- 不消费容器。
- 只能读取元素。
- 遍历结束后仍可继续使用容器。

工程中通常简写为：

```rust
for num in &nums {
    println!("{num}");
}
```

---

### 3.2 `iter_mut()`：可变借用

```rust
let mut nums = vec![1, 2, 3];

for num in nums.iter_mut() {
    *num *= 2;
}

assert_eq!(nums, vec![2, 4, 6]);
```

其元素类型为：

```rust
&mut T
```

特点：

- 不消费容器。
- 可以原地修改元素。
- 迭代期间独占对容器元素的可变借用。

常用简写：

```rust
for num in &mut nums {
    *num *= 2;
}
```

---

### 3.3 `into_iter()`：按值消费

```rust
let names = vec![
    String::from("Alice"),
    String::from("Bob"),
];

for name in names.into_iter() {
    println!("{name}");
}
```

其元素类型通常为：

```rust
T
```

特点：

- 容器被移动并消费。
- 可以直接取得元素所有权。
- 适合后续不再使用原容器的场景。
- 可以避免不必要的 `clone()`。

记忆规则：

```text
iter()       -> &T
iter_mut()   -> &mut T
into_iter()  -> T
```

---

## 4. `Iterator` 与 `IntoIterator`

二者含义不同：

- `Iterator`：对象本身已经是迭代器，可以调用 `next()`。
- `IntoIterator`：对象可以被转换为迭代器。

Rust 的 `for` 循环依赖的是 `IntoIterator`：

```rust
for item in collection {
    // ...
}
```

可以粗略理解为：

```rust
let mut iter = collection.into_iter();

loop {
    match iter.next() {
        Some(item) => {
            // 循环体
        }
        None => break,
    }
}
```

这也是为什么以下三种写法分别代表不同的所有权语义：

```rust
for item in &collection {}      // 不可变借用
for item in &mut collection {}  // 可变借用
for item in collection {}       // 按值消费
```

---

## 5. 惰性求值

Rust 的大多数迭代器适配器都是**惰性的**。

```rust
let nums = vec![1, 2, 3];

let iter = nums
    .iter()
    .map(|x| x * 2);
```

这里只是构造了一个新的迭代器，并没有真正执行 `map`。

只有调用消费器时，迭代才会开始：

```rust
let result: Vec<_> = nums
    .iter()
    .map(|x| x * 2)
    .collect();
```

执行过程并不是：

```text
先生成完整的 map 中间容器
再继续执行下一个操作
```

而是：

```text
取出一个元素
    ↓
执行 map
    ↓
执行 filter
    ↓
输出或丢弃
    ↓
处理下一个元素
```

这使得迭代器链通常不需要生成额外的中间容器。

---

## 6. 常用迭代器适配器

适配器接收一个迭代器，并返回新的迭代器。

### `map`

对元素进行转换：

```rust
let result: Vec<_> = [1, 2, 3]
    .iter()
    .map(|x| x * 2)
    .collect();
```

### `filter`

保留满足条件的元素：

```rust
let result: Vec<_> = [1, 2, 3, 4]
    .iter()
    .copied()
    .filter(|x| x % 2 == 0)
    .collect();
```

### `filter_map`

同时完成过滤和转换：

```rust
let inputs = ["10", "abc", "20"];

let nums: Vec<i32> = inputs
    .iter()
    .filter_map(|s| s.parse().ok())
    .collect();
```

### `enumerate`

附带下标：

```rust
for (index, value) in nums.iter().enumerate() {
    println!("{index}: {value}");
}
```

### `zip`

组合两个序列：

```rust
let names = ["Alice", "Bob"];
let scores = [90, 85];

for (name, score) in names.iter().zip(scores.iter()) {
    println!("{name}: {score}");
}
```

其他常用适配器：

- `flat_map`
- `chain`
- `take`
- `skip`
- `rev`
- `inspect`

---

## 7. 常用消费器

消费器会真正拉动迭代器执行。

常见消费器包括：

| 消费器 | 作用 |
|---|---|
| `collect` | 收集为 `Vec`、`HashMap`、`String` 等 |
| `sum` / `product` | 求和或求积 |
| `count` | 统计元素数量 |
| `fold` | 通用累加 |
| `reduce` | 无显式初始值的聚合 |
| `any` / `all` | 短路判断 |
| `find` / `position` | 查找元素或位置 |
| `max` / `min` | 求最大值或最小值 |
| `for_each` | 对每个元素执行操作 |

示例：

```rust
let sum: i32 = [1, 2, 3, 4]
    .iter()
    .copied()
    .sum();

let has_even = [1, 3, 4]
    .iter()
    .any(|x| x % 2 == 0);
```

---

## 8. 工程最佳实践

### 8.1 用迭代器表达数据转换

当逻辑可以描述为：

```text
输入 -> 过滤 -> 转换 -> 收集
```

适合使用迭代器链：

```rust
let names: Vec<String> = users
    .iter()
    .filter(|user| user.active)
    .map(|user| user.name.clone())
    .collect();
```

### 8.2 用 `for` 表达复杂控制流

出现以下情况时，直接使用 `for` 往往更清晰：

- 多层 `if` 或 `match`。
- 需要 `break`、`continue`。
- 需要记录日志或指标。
- 错误处理较复杂。
- 需要更新多个状态。

```rust
let mut result = Vec::new();

for user in &users {
    if !user.active {
        continue;
    }

    if user.age < 18 {
        continue;
    }

    result.push(user.name.clone());
}
```

不要为了追求“函数式写法”而牺牲可读性。

---

### 8.3 不要过早 `collect`

不推荐：

```rust
let count = nums
    .iter()
    .filter(|x| **x > 0)
    .collect::<Vec<_>>()
    .len();
```

推荐：

```rust
let count = nums
    .iter()
    .filter(|x| **x > 0)
    .count();
```

只在真正需要具体容器时才调用 `collect()`。

---

### 8.4 避免不必要的 `clone`

对于 `Copy` 类型：

```rust
let values: Vec<i32> = nums
    .iter()
    .copied()
    .collect();
```

对于非 `Copy` 类型，确实需要拥有值时再使用：

```rust
.cloned()
```

原容器不再需要时，可以直接使用：

```rust
.into_iter()
```

---

### 8.5 优先使用语义明确的方法

不推荐：

```rust
let has_even = nums
    .iter()
    .fold(false, |acc, x| acc || x % 2 == 0);
```

推荐：

```rust
let has_even = nums
    .iter()
    .any(|x| x % 2 == 0);
```

常见规则：

```text
布尔判断：any / all
计数：count
求和：sum
查找：find
通用聚合：fold
```

不要把所有操作都写成 `fold`。

---

### 8.6 需要下标时使用 `enumerate`

不推荐：

```rust
for i in 0..nums.len() {
    println!("{}", nums[i]);
}
```

推荐：

```rust
for (i, value) in nums.iter().enumerate() {
    println!("{i}: {value}");
}
```

这样更安全，也更符合 Rust 风格。

---

## 9. API 设计

### 接收 `IntoIterator`

如果函数只要求参数能够遍历，可以接收 `IntoIterator`：

```rust
fn sum_all<I>(values: I) -> i32
where
    I: IntoIterator<Item = i32>,
{
    values.into_iter().sum()
}
```

这样可以接受数组、`Vec` 或其他可迭代类型。

### 返回 `impl Iterator`

如果函数只是构造一个迭代过程，可以返回迭代器，避免提前分配：

```rust
fn even_numbers(values: &[i32]) -> impl Iterator<Item = i32> + '_ {
    values
        .iter()
        .copied()
        .filter(|x| x % 2 == 0)
}
```

调用方可以决定是否收集：

```rust
let result: Vec<_> = even_numbers(&nums).collect();
```

### `Box<dyn Iterator>`

只有需要在运行时统一表示不同迭代器类型时，才考虑：

```rust
Box<dyn Iterator<Item = i32>>
```

它会引入动态分发，并降低编译器内联优化的空间。

---

## 10. 循环与迭代器的性能

在 `release` 模式下，Rust 迭代器通常可以达到与手写 `for` 循环接近的性能。

```rust
let sum1: i32 = nums.iter().copied().sum();

let mut sum2 = 0;
for value in &nums {
    sum2 += value;
}
```

两种写法经过优化后，往往会生成非常接近的机器代码。

主要优化机制包括：

1. **泛型单态化**  
   编译器为具体迭代器类型生成专用代码。

2. **静态分发**  
   普通迭代器链通常不需要虚函数调用。

3. **函数与闭包内联**  
   `map`、`filter` 和闭包逻辑能够被展开到循环中。

4. **惰性求值**  
   不会自动产生多个中间集合。

5. **循环融合**  
   多个迭代器适配器可以被优化为一次遍历。

6. **短路执行**  
   `any`、`all`、`find` 等找到结果后可以立即停止。

7. **边界检查优化**  
   遍历 slice 时，迭代器有时比手写下标更容易消除边界检查。

---

## 11. 可能影响性能的情况

以下场景可能降低迭代器性能：

- 使用 Debug 模式测试性能。
- 使用 `Box<dyn Iterator>` 导致动态分发。
- 中途多次 `collect()`。
- 大量不必要的 `clone()`。
- 迭代器链过度复杂，影响内联和自动向量化。
- 在数值计算热路径中使用难以优化的复杂闭包。

性能测试应使用：

```bash
cargo run --release
```

对于真正的热路径，应通过 benchmark 或 profiler 测量，而不是根据代码形式猜测性能。

---

## 12. 常见坑点

### `iter()` 产生引用

```rust
let result: Vec<_> = nums.iter().collect();
```

这里得到的是：

```rust
Vec<&T>
```

需要复制值时，可以使用：

```rust
nums.iter().copied()
```

需要克隆对象时：

```rust
items.iter().cloned()
```

需要取得所有权时：

```rust
items.into_iter()
```

### `filter` 参数可能多一层引用

```rust
let result: Vec<_> = nums
    .iter()
    .filter(|x| **x > 0)
    .collect();
```

因为：

```text
iter() 的 Item = &T
filter 接收 &Item
所以闭包参数可能是 &&T
```

可以先调用 `copied()` 简化：

```rust
let result: Vec<_> = nums
    .iter()
    .copied()
    .filter(|x| *x > 0)
    .collect();
```

### 没有消费器就不会执行

```rust
nums.iter().map(|x| println!("{x}"));
```

这段代码不会真正打印，因为迭代器没有被消费。

---

## 13. 最后总结

```text
只读遍历：for x in &items
修改元素：for x in &mut items
消费元素：for x in items

数据转换：iter + filter / map / filter_map + collect
复杂控制流：直接使用 for
需要下标：enumerate
组合序列：zip
提前结束：any / all / find
聚合计算：sum / fold

能不 collect 就不 collect
能不 clone 就不 clone
性能测试必须使用 release 模式
```

Rust 迭代器最重要的工程理解是：

> **它不仅是遍历容器的工具，也是一个与所有权系统结合的数据处理抽象。简单数据流使用迭代器链，复杂控制流使用 `for`，通常能够同时兼顾安全性、可读性和性能。**
