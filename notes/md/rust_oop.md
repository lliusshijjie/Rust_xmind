# Rust 面向对象特性与工程实践

## 1. 核心认识

Rust 支持面向对象中的**封装**和**多态**，但不支持 C++、Java 那种传统的类继承体系。

Rust 的工程建模通常不是：

```text
BaseClass
├── DerivedClassA
└── DerivedClassB
```

而是：

```text
数据        → struct
行为        → impl
抽象能力    → trait
固定状态集合 → enum
代码复用    → composition（组合）
```

最重要的心智模型是：

> 数据用 `struct`，行为用 `impl`，能力用 `trait`，状态用 `enum`，复用优先使用组合。

---

## 2. 与 C++ / Java 的对应关系

| C++ / Java | Rust |
|---|---|
| `class` | `struct + impl` |
| interface / 抽象基类 | `trait` |
| template + concept | `T: Trait` |
| virtual function | `dyn Trait` |
| `std::unique_ptr<Base>` | `Box<dyn Trait>` |
| `std::shared_ptr<Base>` | `Arc<dyn Trait + Send + Sync>` |
| 类继承 | 结构体组合 |
| 固定的子类集合 | `enum + match` |

需要注意：`trait` 更接近“能力约束”，而不是父类。

---

## 3. 封装：`struct + impl`

Rust 使用 `struct` 保存数据，使用 `impl` 定义与该类型关联的行为。

```rust
pub struct User {
    name: String,
    age: u32,
}

impl User {
    pub fn new(name: String, age: u32) -> Self {
        Self { name, age }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
```

字段默认私有，可以通过公开方法维护类型的不变量。这种形式常用于：

- 业务实体：`User`、`Order`
- 服务对象：`UserService`
- 资源封装：`TcpConnection`、`File`
- 基础组件：`ThreadPool`、`Logger`、`Config`

它相当于 C++ 类中“数据封装 + 成员函数”的部分，但没有继承能力。

---

## 4. 组合替代继承

Rust 不支持继承字段和实现，因此状态复用一般通过组合完成。

```rust
struct Logger;

impl Logger {
    fn log(&self, message: &str) {
        println!("{message}");
    }
}

struct UserService {
    logger: Logger,
}
```

工程中可以按以下方式理解：

| 需求 | Rust 做法 |
|---|---|
| 复用状态 | 将公共组件保存为字段 |
| 复用行为 | trait 默认方法或独立函数 |
| 实现多态 | 泛型或 trait object |
| 表达固定类型分支 | `enum + match` |

组合能够让组件之间的依赖更加明确，也可以避免复杂继承层次带来的强耦合。

---

## 5. 抽象能力：`trait`

`trait` 用于描述一个类型具备哪些行为。

```rust
trait Speak {
    fn speak(&self);
}

struct Dog;

impl Speak for Dog {
    fn speak(&self) {
        println!("woof");
    }
}
```

`Dog` 并不是 `Speak` 的子类，而是实现了 `Speak` 规定的能力。

工程中常见的 trait 包括：

```text
Repository
Storage
Logger
Encoder
Serializer
Strategy
Plugin
```

引入 trait 应当有明确目的，例如：

- 隔离业务层与基础设施层
- 替换不同实现
- 为单元测试提供 Fake 或 Mock
- 隐藏第三方库和底层细节
- 表达泛型约束

不要只是为了模仿 Java 的 interface 而创建 trait。

---

## 6. 静态多态：泛型 + Trait Bound

```rust
fn make_speak<T: Speak>(value: &T) {
    value.speak();
}

struct UserService<R: UserRepository> {
    repo: R,
}
```

编译器会为具体类型生成对应代码，这和 C++ 的模板实例化较为相似。

### 特点

- 编译期确定具体类型
- 通常可以内联
- 没有虚调用开销
- 类型检查严格
- 可能增加类型签名复杂度、编译时间和代码体积

### 适用场景

- 性能敏感路径
- 库内部抽象
- 具体实现能在编译期确定
- 不需要把不同具体类型放入同一个集合

---

## 7. 动态多态：`dyn Trait`

```rust
fn make_speak(value: &dyn Speak) {
    value.speak();
}

struct UserService {
    repo: Box<dyn UserRepository>,
}
```

`dyn Trait` 表示通过 trait object 在运行时进行动态分发，类似 C++ 虚函数。

常见所有权形式：

```rust
Box<dyn Trait>
Arc<dyn Trait + Send + Sync>
```

其中：

- `Box<dyn Trait>`：独占拥有一个堆上的 trait object
- `Arc<dyn Trait + Send + Sync>`：允许多个线程共享同一个 trait object

动态分发适合：

- 根据配置选择实现
- 插件系统
- 需要存储异构对象
- 希望降低泛型在上层传播造成的类型复杂度

它存在一次间接调用开销，但工程中更主要的取舍通常是**灵活性与类型复杂度**，而不是这点调用成本。

---

## 8. `enum + match`：封闭类型集合

当所有可能类型在设计时已经确定，优先考虑 `enum`。

```rust
enum TaskState {
    Pending,
    Running,
    Finished,
    Failed(String),
}

impl TaskState {
    fn is_done(&self) -> bool {
        matches!(self, Self::Finished | Self::Failed(_))
    }
}
```

适合使用 `enum` 的场景包括：

- 状态机
- 错误类型
- 协议消息
- AST 节点
- 固定业务分支

核心选择规则：

```text
类型集合封闭 → enum
类型集合开放 → trait
```

`match` 具有穷尽性检查，新增状态后，编译器可以帮助定位所有需要更新的分支。

---

## 9. 工程中最常见的组合

### 9.1 `struct + impl`

用于封装一个具体对象的数据和行为。

```rust
struct OrderService {
    // dependencies
}

impl OrderService {
    fn create_order(&self) {
        // business logic
    }
}
```

### 9.2 泛型依赖注入

```rust
struct UserService<R: UserRepository> {
    repo: R,
}
```

适合编译期确定实现、性能敏感或库内部的代码。

### 9.3 动态依赖注入

```rust
use std::sync::Arc;

struct UserService {
    repo: Arc<dyn UserRepository + Send + Sync>,
}
```

适合服务端程序、运行时选择实现或希望简化上层类型签名的场景。

### 9.4 应用状态组合

Web 服务中常把各种依赖组合进共享状态：

```rust
struct AppState {
    user_service: Arc<UserService>,
    order_service: Arc<OrderService>,
    config: Arc<Config>,
    db_pool: DbPool,
    cache: Cache,
}
```

这是 Rust 工程中非常典型的“对象组合”，而不是继承体系。

---

## 10. Repository 示例

```rust
trait UserRepository {
    fn find_by_id(&self, id: u64) -> Option<User>;
    fn save(&self, user: User);
}

struct MysqlUserRepository;

impl UserRepository for MysqlUserRepository {
    fn find_by_id(&self, id: u64) -> Option<User> {
        todo!()
    }

    fn save(&self, user: User) {
        todo!()
    }
}
```

静态分发版本：

```rust
struct UserService<R: UserRepository> {
    repo: R,
}
```

动态分发版本：

```rust
use std::sync::Arc;

struct UserService {
    repo: Arc<dyn UserRepository + Send + Sync>,
}
```

这种架构表达了：

```text
业务层依赖抽象 trait
基础设施层提供具体 impl
应用组装层选择实际实现
```

这比构建 `AbstractUserService → UserServiceImpl` 一类继承结构更符合 Rust 风格。

---

## 11. 常见设计模式在 Rust 中的体现

| 传统设计模式 | Rust 中常见实现 |
|---|---|
| Interface | `trait` |
| Abstract Base Class | `trait + dyn Trait` |
| Strategy | trait、泛型或闭包 |
| Repository / DAO | `trait + struct impl` |
| Adapter | 包装类型并实现目标 trait |
| Factory | 普通函数或关联函数 |
| Builder | Builder 结构体 |
| State | 优先 `enum + match` |
| Dependency Injection | 泛型参数或 `Arc<dyn Trait>` |
| Visitor | trait 或 `enum + match` |

Rust 中不少传统设计模式可以直接由语言特性表达，因此通常不需要完整照搬经典 OOP 模板。

---

## 12. 工程选型规则

| 需求 | 推荐方式 |
|---|---|
| 绑定数据和行为 | `struct + impl` |
| 描述抽象能力 | `trait` |
| 类型集合固定 | `enum` |
| 类型集合允许外部扩展 | `trait` |
| 重视静态优化 | `T: Trait` |
| 重视运行时灵活性 | `dyn Trait` |
| 独占堆对象 | `Box` |
| 共享所有权 | `Arc` |
| 共享可变状态 | `Mutex` / `RwLock` / 原子类型 |
| 跨线程共享 trait object | `Arc<dyn Trait + Send + Sync>` |

---

## 13. 注意事项

### 13.1 不要把 Rust 写成 Java

应避免没有实际价值的抽象层级：

```text
BaseService
AbstractUserService
DefaultUserService
UserServiceImpl
```

Rust 中通常直接使用具体结构体，只在确实需要替换实现或隔离依赖时引入 trait。

### 13.2 trait 不是父类

trait 描述的是能力，不代表继承和“is-a”关系。设计时应优先思考：

```text
这个类型拥有哪些能力？
它由哪些组件构成？
它可能处于哪些状态？
```

### 13.3 `dyn Trait` 存在限制

并非所有 trait 都能直接用于 trait object。依赖具体 `Self`、返回 `Self` 或包含泛型方法的接口，通常需要额外约束或重新设计。

设计动态接口时，应尽量让方法能够通过 `&self`、`&mut self` 或 `Box<Self>` 等明确的接收方式调用。

### 13.4 `Arc` 不等于线程安全可变性

`Arc<T>` 只提供线程安全的共享所有权，不允许直接修改内部数据。

需要共享修改时通常使用：

```rust
Arc<Mutex<T>>
Arc<RwLock<T>>
```

或者根据场景使用原子类型、消息传递和并发容器。

### 13.5 不要过早使用 trait object

对于固定状态集合，`enum` 往往更加简单、安全，并且能够获得穷尽性检查。

对于只有一个实现、没有测试替换需求的模块，也不一定需要提前创建 trait。

### 13.6 控制 trait 粒度

trait 应当小而清晰，避免把大量无关行为集中到一个“God Trait”中。调用方只应依赖自己真正需要的能力。

---

## 14. 最佳实践总结

1. 优先设计清晰的所有权关系，再设计抽象层。
2. 优先组合，不使用继承式思维强行组织代码。
3. 类型集合封闭时使用 `enum + match`。
4. 类型需要开放扩展时使用 `trait`。
5. 性能敏感且类型确定时使用静态分发。
6. 运行时切换实现、插件或异构集合使用动态分发。
7. 业务层依赖 trait，基础设施层实现 trait。
8. trait 应当小而专一，抽象必须解决真实问题。
9. 不要把所有依赖都包装成 `Arc<dyn Trait>`。
10. 多线程场景需要同时考虑所有权、内部可变性以及 `Send + Sync`。

---

## 15. 最终理解

Rust 不是没有面向对象，而是将传统 class 拆分成了更独立的语言机制：

```text
struct 负责数据
impl   负责行为
trait  负责能力抽象
enum   负责封闭状态
组合    负责组件复用
所有权 负责资源与生命周期
```

对于 C++ 程序员，真正需要完成的思维转换是：

> 从设计“父类和子类的继承体系”，转向设计“所有权关系、能力边界和组件组合”。
