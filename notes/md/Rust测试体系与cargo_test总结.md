# Rust 测试基础与工程实践总结

## 1. Rust 测试的定位

Rust 的测试体系可以理解为：

> `#[test]` 测试属性 + 标准断言宏 + `cargo test` 命令 + Cargo 约定目录结构。

从 C++ 视角看，Rust 测试和 GoogleTest / Catch2 / CTest 的角色类似，但 Rust 的特点是测试能力已经进入标准工具链：

- `rustc` 提供 test harness，用于自动发现和执行测试函数；
- `cargo test` 负责编译并运行测试；
- `#[test]` 标记测试函数；
- `assert!` / `assert_eq!` / `assert_ne!` 等宏负责断言。

测试失败的本质是 **panic**：

```text
断言失败 / 主动 panic  => 测试失败
无 panic / 返回 Ok(()) => 测试通过
```

所以 Rust 测试函数本质上仍然是普通 Rust 函数，只是被 test harness 自动识别和执行。

---

## 2. 单元测试基础结构

### 2.1 推荐模板

Rust 单元测试一般写在被测 `.rs` 文件的末尾：

```rust
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_returns_sum() {
        assert_eq!(add(1, 2), 3);
    }
}
```

这个结构不是语言强制要求，但属于 Rust 社区非常常见的工程习惯。

### 2.2 `#[cfg(test)]`

`#[cfg(test)]` 是条件编译属性。

作用：

```text
cargo test       编译 tests 模块
cargo build      不编译 tests 模块
cargo run        不编译 tests 模块
```

好处是：测试代码、测试数据构造函数、测试辅助函数不会进入正常构建产物。

### 2.3 `mod tests`

`mod tests` 不是必须的，但强烈推荐。

它的主要作用：

- 隔离测试代码；
- 统一管理测试用例、测试 helper、测试 fixture；
- 避免测试代码污染主逻辑；
- 作为当前模块的子模块，可以通过 `use super::*` 访问父模块内容。

也就是说，下面这样也能跑：

```rust
#[test]
fn add_returns_sum() {
    assert_eq!(add(1, 2), 3);
}
```

但工程上更推荐：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_returns_sum() {
        assert_eq!(add(1, 2), 3);
    }
}
```

### 2.4 `#[test]`

`#[test]` 用于标记测试函数。

测试函数通常满足：

- 无参数；
- 返回 `()` 或 `Result<(), E>`；
- 无 panic 表示通过；
- panic 或返回 `Err` 表示失败。

普通写法：

```rust
#[test]
fn parse_valid_number() {
    let value = "42".parse::<i32>().unwrap();
    assert_eq!(value, 42);
}
```

返回 `Result` 的写法：

```rust
#[test]
fn parse_valid_number() -> Result<(), Box<dyn std::error::Error>> {
    let value = "42".parse::<i32>()?;
    assert_eq!(value, 42);
    Ok(())
}
```

对于涉及 IO、解析、临时文件的测试，返回 `Result` 往往比大量 `unwrap()` 更自然。

### 2.5 `use super::*`

在单元测试中常见：

```rust
#[cfg(test)]
mod tests {
    use super::*;
}
```

含义是引入父模块中的符号。

因为 `tests` 是当前模块的子模块，所以它可以访问父模块中的私有函数、私有类型：

```rust
fn internal_add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_add() {
        assert_eq!(internal_add(1, 2), 3);
    }
}
```

这也带来一个重要工程原则：

> 不要为了测试把内部函数强行改成 `pub`。

---

## 3. 常见断言宏与匹配方式

### 3.1 `assert!`

用于断言表达式为 `true`：

```rust
#[test]
fn value_is_positive() {
    let value = 10;
    assert!(value > 0);
}
```

适合验证布尔条件。

### 3.2 `assert_eq!`

用于判断两个值相等：

```rust
#[test]
fn add_returns_sum() {
    assert_eq!(1 + 2, 3);
}
```

失败时会打印左右两边的值，所以这是最常用的断言宏之一。

### 3.3 `assert_ne!`

用于判断两个值不相等：

```rust
#[test]
fn id_should_change() {
    let old_id = 1;
    let new_id = 2;
    assert_ne!(old_id, new_id);
}
```

适合测试唯一 ID、状态变化等场景。

### 3.4 `panic!`

主动触发 panic：

```rust
#[test]
fn force_fail() {
    panic!("this test fails intentionally");
}
```

测试失败的本质就是 panic。

### 3.5 自定义错误信息

断言宏可以添加自定义错误信息：

```rust
#[test]
fn add_returns_sum() {
    let result = 2 + 2;
    assert_eq!(result, 4, "加法结果不符合预期: result = {}", result);
}
```

建议错误信息用于说明场景，而不是重复断言本身。

### 3.6 测试 `Option` / `Result`

常见写法：

```rust
#[test]
fn option_is_some() {
    let value = Some(10);
    assert!(value.is_some());
    assert_eq!(value.unwrap(), 10);
}

#[test]
fn result_is_error() {
    let result: Result<i32, &str> = Err("invalid input");
    assert!(result.is_err());
}
```

如果需要检查错误内容：

```rust
#[test]
fn check_error_content() {
    let result: Result<i32, &str> = Err("invalid input");
    assert_eq!(result.unwrap_err(), "invalid input");
}
```

### 3.7 `matches!` 宏

`matches!` 适合测试枚举分支、`Option`、`Result`：

```rust
#[test]
fn option_matches_condition() {
    let value = Some(10);
    assert!(matches!(value, Some(x) if x > 5));
}
```

对比大量 `unwrap()`，`matches!` 更能表达业务分支。

---

## 4. 常见测试属性

### 4.1 `#[should_panic]`

表示测试函数应该 panic。

```rust
fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic!("divide by zero");
    }
    a / b
}

#[test]
#[should_panic]
fn divide_by_zero_should_panic() {
    divide(10, 0);
}
```

规则：

```text
发生 panic   => 测试通过
没有 panic   => 测试失败
```

适合测试非法状态、违反 API 前置条件、断言式 API。

### 4.2 `#[should_panic(expected = "...")]`

要求 panic 信息包含指定字符串：

```rust
#[test]
#[should_panic(expected = "divide by zero")]
fn divide_by_zero_should_panic_with_message() {
    divide(10, 0);
}
```

这样可以避免“任何 panic 都让测试通过”的误判。

### 4.3 `#[ignore]`

默认 `cargo test` 不运行该测试，但仍然会编译：

```rust
#[test]
#[ignore]
fn expensive_test() {
    // 慢测试、网络测试、数据库测试等
}
```

也可以写明原因：

```rust
#[test]
#[ignore = "requires local database"]
fn database_test() {
    // 依赖本地数据库
}
```

### 4.4 异步测试补充

标准库的 `#[test]` 不能直接用于 `async fn`：

```rust
#[test]
async fn async_test() {
    // 标准 #[test] 不支持这种形式
}
```

Tokio 项目中常见：

```rust
#[tokio::test]
async fn async_api_test() {
    // async test
}
```

注意：`#[tokio::test]` 是 Tokio 提供的测试宏，不是 Rust 标准库内置能力。

---

## 5. 测试类型与目录结构

Rust 工程中常见三类测试：

```text
单元测试 unit test
集成测试 integration test
文档测试 doctest
```

### 5.1 单元测试：写在 `src/` 内部

单元测试通常写在被测模块所在的 `.rs` 文件末尾：

```text
src/
  lib.rs
  parser.rs      // 内部可以有 #[cfg(test)] mod tests
  store.rs       // 内部可以有 #[cfg(test)] mod tests
```

特点：

- 贴近被测代码；
- 可以访问 private 函数和类型；
- 适合测试算法、解析器、状态机、小模块逻辑。

### 5.2 集成测试：写在 `tests/` 顶层

集成测试放在项目根目录的 `tests/` 下：

```text
my_crate/
├── src/
│   └── lib.rs
└── tests/
    └── api.rs
```

示例：

```rust
// tests/api.rs

use my_crate::add;

#[test]
fn add_from_public_api() {
    assert_eq!(add(1, 2), 3);
}
```

特点：

- `tests/*.rs` 每个文件都会被编译成独立测试 crate；
- 只能访问 `pub` API；
- 模拟外部用户使用你的 crate；
- 适合测试模块协作、端到端流程、公共接口兼容性。

### 5.3 `tests/` 下的辅助代码

错误写法：

```text
tests/
  api.rs
  common.rs
```

`tests/common.rs` 也会被 Cargo 当成一个独立测试入口，即使里面没有 `#[test]`，也可能出现 `running 0 tests`。

推荐写法：

```text
tests/
  api.rs
  common/
    mod.rs
```

`tests/common/mod.rs`：

```rust
pub fn setup() {
    // 初始化测试环境
}
```

`tests/api.rs`：

```rust
mod common;

#[test]
fn api_test() {
    common::setup();
}
```

规则总结：

```text
tests/*.rs             会被当成集成测试入口
tests/common.rs       不推荐，会被当成测试入口
tests/common/mod.rs   推荐，作为辅助模块
```

### 5.4 文档测试 doctest

Rust 文档注释里的代码块可以被 `cargo test` 执行：

```rust
/// Adds two numbers.
///
/// # Examples
///
/// ```
/// let result = my_crate::add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

价值：

- 保证文档示例不会过期；
- 适合库代码和公共 API；
- 用户看到的示例就是可编译、可运行的代码。

---

## 6. `cargo test` 常用命令

### 6.1 基础运行

运行所有测试：

```bash
cargo test
```

运行名字中包含 `test_name` 的测试：

```bash
cargo test test_name
```

精确匹配测试名：

```bash
cargo test test_name -- --exact
```

### 6.2 选择测试目标

只运行 library 单元测试：

```bash
cargo test --lib
```

只运行某个集成测试文件：

```bash
cargo test --test api
```

这里对应文件：

```text
tests/api.rs
```

只运行集成测试目标：

```bash
cargo test --tests
```

只运行文档测试：

```bash
cargo test --doc
```

### 6.3 输出与调试

显示成功测试的标准输出：

```bash
cargo test -- --show-output
```

不捕获 `println!` 输出：

```bash
cargo test -- --nocapture
```

单线程运行测试：

```bash
cargo test -- --test-threads=1
```

适合排查共享状态、固定文件、环境变量互相影响的问题。

### 6.4 ignored 测试

只运行被 `#[ignore]` 标记的测试：

```bash
cargo test -- --ignored
```

运行普通测试和 ignored 测试：

```bash
cargo test -- --include-ignored
```

### 6.5 编译与构建模式

只编译测试，不运行：

```bash
cargo test --no-run
```

release 模式运行测试：

```bash
cargo test --release
```

### 6.6 Workspace / Package

测试整个 workspace：

```bash
cargo test --workspace
```

只测试指定 package：

```bash
cargo test -p crate_name
```

### 6.7 Feature 组合

开启指定 feature：

```bash
cargo test --features feature_name
```

开启所有 feature：

```bash
cargo test --all-features
```

禁用默认 feature：

```bash
cargo test --no-default-features
```

### 6.8 参数分界线 `--`

需要记住：

```text
-- 前面：Cargo 参数
-- 后面：test harness 参数
```

例如：

```bash
cargo test parse -- --nocapture
```

含义：

```text
parse         传给 Cargo，用于过滤测试名
--nocapture   传给 test harness，用于显示 println! 输出
```

---

## 7. 工程最佳实践

### 7.1 `mod tests` 不是必须，但强烈推荐

结论：

```text
#[test]        是测试函数必须的
mod tests      不是语法必须的
#[cfg(test)]   对 src 内单元测试强烈推荐
```

推荐写法：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_name() {
        // ...
    }
}
```

理由：

- 测试代码和主代码隔离；
- 测试 helper 不进入正常构建；
- 测试模块可以访问父模块私有实现；
- 文件结构更清晰，通常放在 `.rs` 文件末尾。

### 7.2 不要为了测试暴露内部实现

不推荐：

```rust
pub fn internal_parse() {
    // 只是为了测试而改成 pub
}
```

推荐：

```rust
fn internal_parse() {
    // private
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_parse() {
        internal_parse();
    }
}
```

单元测试可以测 private 逻辑，集成测试只测 public API。

### 7.3 优先测试行为，而不是实现细节

好的测试应该关注：

- 输入输出是否正确；
- 错误语义是否正确；
- 状态变化是否正确；
- public API 行为是否稳定。

不要过度绑定内部实现步骤。否则重构时，行为没变，测试却大量失败。

### 7.4 测试命名要表达场景

不推荐：

```rust
#[test]
fn test_parse() {}
```

推荐：

```rust
#[test]
fn parse_returns_error_when_port_is_invalid() {}

#[test]
fn parse_uses_default_timeout_when_missing() {}

#[test]
fn parse_accepts_valid_ipv4_address() {}
```

推荐格式：

```text
被测对象 + 输入场景 + 期望结果
```

### 7.5 错误路径优先测试 `Result::Err`

业务错误更推荐用 `Result` 表达：

```rust
#[test]
fn parse_invalid_config_returns_error() {
    let result = parse_config("bad config");
    assert!(result.is_err());
}
```

不要滥用 `#[should_panic]` 表达业务失败。

`panic` 更适合：

- 不可恢复错误；
- 内部不变量被破坏；
- API 前置条件被违反；
- 明确设计为 panic 的函数。

### 7.6 测试之间保持独立

Rust 测试默认可能并行运行，因此测试之间不要依赖：

- 执行顺序；
- 全局状态；
- 固定文件名；
- 数据库残留；
- 环境变量残留。

更好的方式：

- 每个测试自己创建资源；
- 每个测试自己清理资源；
- 使用临时目录；
- 使用 mock / fixture；
- 必要时使用 `tempfile` crate。

如果临时排查问题，可以：

```bash
cargo test -- --test-threads=1
```

但这通常是兜底手段，不应成为常态。

### 7.7 慢测试和外部依赖测试要隔离

数据库、网络、真实文件系统、第三方服务相关测试需要控制边界。

常见处理方式：

- 使用 `#[ignore]` 标记慢测试；
- 使用 feature gate 控制是否编译；
- 使用环境变量控制是否运行；
- 使用 mock 替代真实外部服务；
- 使用测试数据库或临时目录。

### 7.8 文档和 CI

公共 API 尽量写 doctest，保证示例代码不会过期。

常见 CI 组合：

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --workspace --all-features
```

这套组合大致对应：

```text
格式检查 + 静态检查 + 全量测试
```

---

## 8. 推荐测试策略

### 8.1 核心纯逻辑

对于算法、解析器、状态机、数据结构等纯逻辑代码：

- 多写单元测试；
- 覆盖正常路径；
- 覆盖边界条件；
- 覆盖错误路径。

### 8.2 公共 API

对于 crate 暴露出去的能力：

- 使用集成测试；
- 只通过 public API 测试；
- 模拟真实用户使用方式；
- 保护 API 行为不被无意破坏。

### 8.3 文档示例

对于库代码：

- 公共 API 文档中写示例；
- 示例尽量可运行；
- 让 doctest 保证文档不过期。

### 8.4 异步 / IO / 数据库

这类测试要重点关注环境隔离：

- 使用临时目录；
- 使用测试数据库；
- 使用 mock；
- 控制并发影响；
- 必要时 `#[ignore]` 或 feature 控制。

---

## 9. 一句话总结

Rust 测试的核心可以概括为：

> 单元测试贴近实现，集成测试面向 public API，文档测试保证示例不失效；测试应该稳定、独立、可维护。

对于工程实践，可以先记住这套规则：

```text
1. src 内部写 #[cfg(test)] mod tests
2. tests/ 顶层写集成测试入口
3. tests/common/mod.rs 放测试辅助代码
4. 不要为了测试把 private 改成 pub
5. 优先测试行为，不要过度绑定实现细节
6. 业务错误测试 Result::Err，少用 should_panic
7. 测试之间保持独立，不依赖执行顺序
8. 慢测试、外部依赖测试用 #[ignore] 或 feature 隔离
9. 公共 API 示例尽量写 doctest
10. CI 中至少跑 fmt、clippy、test
```
