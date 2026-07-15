# Rust Cargo 知识体系总结

Cargo 不只是 Rust 的构建工具，它同时承担了以下职责：

- 编译和运行项目；
- 管理依赖与版本；
- 组织多 crate 工程；
- 执行测试、生成文档；
- 打包和发布 crate；
- 安装 Rust 命令行工具。

可以用下面的线索记忆本章内容：

```text
profile       → 怎么构建
publish       → 怎么发布 crate
pub use       → 怎么设计公共 API
workspace     → 怎么组织多 crate 工程
cargo install → 怎么安装 Rust 命令行工具
doctest       → 怎么让文档示例参与测试
```

---

## 1. 发布配置：Profile

Cargo 通过 `Cargo.toml` 中的 `[profile.*]` 配置不同构建模式。

它的本质是在以下目标之间做权衡：

- 编译速度；
- 调试能力；
- 运行性能；
- 二进制体积。

这与 CMake 中的 `Debug`、`Release`、`MinSizeRel` 等构建类型类似。

### 1.1 基础用法

```bash
cargo build
cargo run
```

默认使用：

```toml
[profile.dev]
```

构建产物位于：

```text
target/debug/
```

发布模式：

```bash
cargo build --release
cargo run --release
```

使用：

```toml
[profile.release]
```

构建产物位于：

```text
target/release/
```

### 1.2 常见配置项

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

各项含义：

| 配置 | 含义 |
|---|---|
| `opt-level = 0` | 基本不优化，编译快，适合开发调试 |
| `opt-level = 3` | 强性能优化，适合正式发布 |
| `opt-level = "s"` | 优先优化二进制体积 |
| `opt-level = "z"` | 更激进地优化二进制体积 |
| `lto = true` | 开启链接时优化，可能提升性能，但增加编译时间 |
| `codegen-units = 1` | 提高跨代码单元优化机会，但降低并行编译速度 |
| `panic = "abort"` | panic 时直接终止进程，减小运行时和二进制体积 |
| `strip = true` | 剥离符号信息，减小发布产物 |

### 1.3 最佳实践

1. 日常开发使用默认的 `dev` 配置。
2. 不要使用 debug 构建结果判断真实性能。
3. 性能测试、压测和 benchmark 必须使用 `--release`。
4. 不要一开始就堆叠所有优化选项，应先保证正确性和可调试性。
5. CLI 小工具、嵌入式程序可考虑 `opt-level = "z"` 和 `strip`。
6. 线上需要 profiling 时，可以在 release 构建中保留调试信息。

---

## 2. 发布 Crate

发布 crate，是指把 Rust package 上传到 crates.io，使其他项目能够通过 `Cargo.toml` 依赖它。

一个 package 中可以包含：

- library target；
- binary target；
- examples；
- tests；
- benchmarks。

### 2.1 发布前准备

首先需要：

1. 注册 crates.io 账号并验证邮箱；
2. 创建 API Token；
3. 在本机登录：

```bash
cargo login
```

Token 等价于 crate 发布权限，不应提交到 Git 仓库；泄露后应立即撤销。

### 2.2 Cargo.toml 元数据

典型配置如下：

```toml
[package]
name = "my_crate"
version = "0.1.0"
edition = "2024"
description = "A short description of this crate"
license = "MIT OR Apache-2.0"
repository = "https://github.com/example/my_crate"
readme = "README.md"
keywords = ["rust", "cli"]
categories = ["command-line-utilities"]
```

重要字段：

| 字段 | 作用 |
|---|---|
| `name` | crate 名称，在 crates.io 上先到先得 |
| `version` | 语义化版本号 |
| `edition` | Rust Edition，例如 2021、2024 |
| `description` | crate 的简短说明 |
| `license` | 开源协议，建议使用 SPDX 表达式 |
| `repository` | 源码仓库地址 |
| `readme` | crates.io 页面展示的主要文档 |
| `keywords` | 搜索关键词 |
| `categories` | crates.io 分类 |

### 2.3 发布前检查流程

推荐按下面的顺序执行：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test
cargo doc --no-deps
cargo package --list
cargo publish --dry-run
```

各命令的作用：

- `cargo fmt --check`：检查格式是否统一；
- `cargo clippy`：检查常见错误和不符合 Rust 惯用法的代码；
- `cargo test`：运行测试，包括文档测试；
- `cargo doc`：检查公开 API 文档；
- `cargo package --list`：查看最终会被打包的文件；
- `cargo publish --dry-run`：模拟发布，但不真正上传。

可以通过 `include` 或 `exclude` 控制发布包内容：

```toml
[package]
exclude = [
    "/ci",
    "/images",
    "*.log",
]
```

### 2.4 正式发布

发布当前 package：

```bash
cargo publish
```

在 workspace 中发布指定 package：

```bash
cargo publish -p my_crate
```

发布后，其他项目可以添加依赖：

```toml
[dependencies]
my_crate = "0.1"
```

### 2.5 版本管理与限制

一个版本发布后：

- 不能覆盖同一版本；
- 不能真正从 crates.io 删除代码；
- 出现问题时只能发布新的版本修复。

错误版本可以撤回：

```bash
cargo yank my_crate@0.1.0
```

`yank` 并不是删除：

- 新的依赖解析通常不会再选择该版本；
- 已经在 `Cargo.lock` 中锁定该版本的项目仍可能继续构建。

### 2.6 语义化版本

```text
1.2.3
│ │ └─ PATCH：兼容性 Bug 修复
│ └─── MINOR：新增向后兼容功能
└───── MAJOR：存在破坏性 API 变更
```

### 2.7 最佳实践

1. 发布前确认公共 API 是否足够稳定。
2. README 至少包含用途、快速开始、核心示例和功能说明。
3. 公共函数、结构体、枚举和 trait 应尽量编写 rustdoc。
4. 使用 `include` / `exclude` 避免上传测试数据、大图片和无关文件。
5. 严格遵循 SemVer，谨慎进行破坏性变更。
6. CI 自动发布时，优先考虑 Trusted Publishing，降低长期 Token 泄露风险。

---

## 3. `pub use` 与公共 API 设计

`pub use` 的作用是重新导出，即 re-export。

```rust
pub use path::Item;
```

它同时完成两件事：

1. 将 `Item` 引入当前模块；
2. 通过当前模块的公共路径再次暴露 `Item`。

### 3.1 `use` 与 `pub use` 的区别

普通导入：

```rust
use parser::ast::Expr;
```

`Expr` 只在当前模块中可直接使用。

重新导出：

```rust
pub use parser::ast::Expr;
```

外部用户可以直接使用新的公共路径：

```rust
use my_crate::Expr;
```

而不必知道真实内部路径：

```rust
use my_crate::parser::ast::Expr;
```

### 3.2 为什么需要 `pub use`

工程代码的内部组织结构通常会逐渐复杂：

```text
my_crate
└── parser
    └── ast
        └── Expr
```

但公共 API 应尽量简单、稳定：

```rust
use my_crate::Expr;
```

因此，`pub use` 能够：

- 分离内部模块结构与外部 API 结构；
- 隐藏深层模块路径；
- 降低使用者的认知成本；
- 允许内部重构时保持外部路径稳定；
- 为 crate 建立统一的公共入口。

### 3.3 常见使用方式

在 `lib.rs` 中重新导出核心类型：

```rust
mod parser;
mod runtime;

pub use parser::ast::Expr;
pub use runtime::Executor;
```

提供 `prelude`：

```rust
pub mod prelude {
    pub use crate::Executor;
    pub use crate::Expr;
    pub use crate::Parser;
}
```

用户统一导入：

```rust
use my_crate::prelude::*;
```

### 3.4 最佳实践

1. 核心类型和核心 trait 可以 re-export 到 crate 顶层。
2. 内部实现细节不要对外重新导出。
3. 不要为了缩短路径而滥用 `pub use *`。
4. 公共路径一旦发布，应将其视为 API 兼容性的一部分。
5. `pub use` 是 API 设计工具，而不仅是语法糖。

从 C++ 角度，可以把它理解为：在公共头文件中包含内部头文件，再通过 `using` 为用户提供统一入口。

---

## 4. Workspace

Workspace 是 Cargo 管理多个相关 package 的机制。

适用于：

- 一个仓库中包含多个 crate；
- 多个程序共享公共库；
- 希望统一构建、测试和依赖版本；
- 大型项目需要建立清晰的模块边界。

### 4.1 基本结构

```text
my_workspace/
├── Cargo.toml
├── app/
│   ├── Cargo.toml
│   └── src/main.rs
├── domain/
│   ├── Cargo.toml
│   └── src/lib.rs
└── storage/
    ├── Cargo.toml
    └── src/lib.rs
```

根目录的 `Cargo.toml`：

```toml
[workspace]
members = [
    "app",
    "domain",
    "storage",
]
resolver = "3"
```

根目录可以只作为管理入口，本身不一定是 crate。

### 4.2 Workspace 共享内容

Workspace 成员通常共享：

- 根目录的 `Cargo.lock`；
- 根目录的 `target/`；
- 统一的 Cargo 命令入口。

在根目录执行：

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace
```

指定某个 package：

```bash
cargo build -p app
cargo test -p domain
cargo run -p app
```

### 4.3 成员之间的依赖

例如 `app` 依赖 `domain`：

```toml
[dependencies]
domain = { path = "../domain" }
```

然后直接使用：

```rust
use domain::User;
```

注意：不要把业务 crate 命名为 `core`、`std` 等容易与标准库混淆的名字。

### 4.4 常见工程拆分

服务端项目常见结构：

```text
crates/
├── domain/   # 业务模型与规则
├── service/  # 业务用例和服务
├── storage/  # 数据库、缓存等基础设施
├── api/      # HTTP / RPC 接口
└── common/   # 通用能力

apps/
└── server/   # 二进制入口
```

### 4.5 最佳实践

1. 小项目不要过早引入 workspace。
2. 当多个 crate 确实需要共享构建、测试和依赖时再拆分。
3. 按稳定的职责边界拆 crate，而不是按文件数量拆分。
4. 公共底层 crate 不应反向依赖上层业务 crate。
5. 通过依赖方向避免循环依赖。
6. 将 workspace 根目录作为 CI 的统一入口。

---

## 5. 安装二进制 Crate

`cargo install` 用于安装 Rust 编写的命令行工具。

它与添加项目依赖不同：

```text
cargo add / Cargo.toml [dependencies]
    → 给当前项目添加库依赖

cargo install
    → 在本机安装可执行程序
```

### 5.1 常用命令

```bash
# 从 crates.io 安装
cargo install ripgrep

# 查看已安装工具
cargo install --list

# 卸载
cargo uninstall ripgrep

# 安装本地项目
cargo install --path .

# 从 Git 仓库安装
cargo install --git https://github.com/example/tool.git

# 安装 package 中指定的 binary
cargo install tool --bin tool_name
```

### 5.2 安装位置

默认安装目录：

```text
~/.cargo/bin
```

Windows 通常为：

```text
%USERPROFILE%\.cargo\bin
```

安装后命令无法识别时，应优先检查该目录是否已经加入 `PATH`。

### 5.3 固定版本

```bash
cargo install tool --version 1.2.3
```

这里的 `1.2.3` 默认表示精确版本，不等同于 `Cargo.toml` 中常见的兼容版本范围。

强制重新安装：

```bash
cargo install tool --force
```

### 5.4 `--locked`

```bash
cargo install tool --version 1.2.3 --locked
```

`--locked` 表示使用发布包附带的 `Cargo.lock`，尽量采用作者测试过的依赖组合。

重要工具和 CI 环境推荐：

```text
固定工具版本 + --locked
```

这样可以降低依赖重新解析导致的构建失败或行为变化。

### 5.5 安全注意事项

`cargo install` 会：

- 下载并编译第三方源代码；
- 执行依赖中的构建脚本 `build.rs`；
- 可能调用本机 C/C++ 编译器和系统工具。

因此：

1. 不要随意安装来源不明的 crate。
2. 优先选择知名、活跃维护、源码可信的项目。
3. CI 中应固定版本，避免每次拉取不受控的新版本。

安装失败时，依次检查：

- Rust 工具链版本；
- C/C++ 编译器；
- 系统开发库；
- 环境变量 `PATH`；
- crate 要求的额外系统依赖。

---

## 6. 文档测试：Doctest

文档测试是 Rust 很有特色的机制：文档注释中的 Rust 代码块会被 `rustdoc` 提取并作为测试执行。

它同时具有两种价值：

- 面向用户的 API 使用示例；
- 防止示例失效的回归测试。

### 6.1 文档注释

`///` 用于描述函数、结构体、枚举、trait 等 item：

```rust
/// Adds two numbers.
///
/// ```rust
/// assert_eq!(my_crate::add(1, 2), 3);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

`//!` 用于 crate 或模块级文档，通常写在文件顶部：

```rust
//! A small parser library.
//! 
//! This crate provides the [`Parser`] type.
```

### 6.2 执行方式

```bash
# 运行单元测试、集成测试和文档测试
cargo test

# 只运行文档测试
cargo test --doc

# workspace 中测试指定 package
cargo test -p my_crate --doc

# 生成并打开文档
cargo doc --open

# 不生成依赖文档
cargo doc --no-deps --open
```

### 6.3 外部用户视角

Doctest 通常像外部 crate 一样访问当前库，因此应尽量使用公开路径：

```rust
/// ```rust
/// use my_crate::Parser;
///
/// let parser = Parser::new();
/// assert!(parser.is_empty());
/// ```
```

如果示例依赖私有模块，文档测试会编译失败。

### 6.4 隐藏样板代码

以 `#` 开头的行会参与编译和执行，但不会显示在最终文档中：

```rust
/// ```rust
/// # use my_crate::Parser;
/// let parser = Parser::new();
/// assert!(parser.is_empty());
/// ```
```

适合隐藏：

- `use` 导入；
- 初始化代码；
- `Result` 返回包装；
- 与核心示例无关的样板代码。

### 6.5 代码块属性

#### `no_run`

要求代码能够编译，但不执行：

```rust
/// ```no_run
/// std::fs::write("output.txt", "hello")?;
/// # Ok::<(), std::io::Error>(())
/// ```
```

适用于：

- 文件操作；
- 网络请求；
- 长时间运行任务；
- 依赖外部环境的代码。

#### `compile_fail`

期望示例编译失败：

```rust
/// ```compile_fail
/// let value = String::from("hello");
/// let moved = value;
/// println!("{value}");
/// ```
```

适合展示：

- 所有权错误；
- 借用检查错误；
- trait bound 不满足；
- API 禁止的错误用法。

#### `should_panic`

示例预期运行时发生 panic：

```rust
/// ```should_panic
/// panic!("expected panic");
/// ```
```

#### `ignore`

完全跳过该代码块：

```rust
/// ```ignore
/// some_platform_specific_code();
/// ```
```

`ignore` 会让示例失去测试价值，应尽量少用。

#### 非 Rust 代码

使用语言标识避免被当作 Rust 测试：

````markdown
```bash
cargo run --release
```

```text
This is plain text.
```
````

### 6.6 最佳实践

1. 库 crate 的核心公共 API 应优先编写 doctest。
2. 示例应短小、稳定，并且能够直接复制使用。
3. 示例中使用断言，提高回归测试价值。
4. 避免依赖网络、时间、随机数和本机环境。
5. 每个代码块应当相互独立，自己完成必要的导入和初始化。
6. 示例应展示推荐用法，而不是暴露内部实现细节。
7. 结合 `pub use` 提供稳定、简洁的公共路径。
8. 发布 crate 前至少运行：

```bash
cargo test
cargo test --doc
```

### 6.7 常见误区

- 使用私有模块路径，导致 doctest 编译失败；
- 示例只有输出，没有断言，测试价值较弱；
- 大量使用 `ignore`，导致文档长期失效；
- 把 doctest 当成内部单元测试，它实际上更接近外部用户视角；
- 一个示例依赖另一个代码块的变量，每个代码块实际上是独立测试。

---

## 7. 总体工程原则

Cargo 相关知识最终应落到以下工程意识：

### 构建

- 开发模式关注编译速度和调试体验；
- 发布模式关注性能和体积；
- 性能结论必须基于 release 构建。

### API

- 使用 `pub use` 设计稳定、简洁的公共路径；
- 内部模块结构可以复杂，对外接口应保持清晰；
- 公共路径也是兼容性承诺的一部分。

### 工程组织

- 小项目保持单 crate；
- 工程边界稳定后再拆分 workspace；
- 控制 crate 之间的依赖方向，避免循环依赖。

### 发布

- 发布前执行格式、静态检查、测试、文档和 dry-run；
- 遵循语义化版本；
- 已发布版本不可覆盖，发布操作必须谨慎。

### 安全与可复现性

- 第三方 crate 和 `build.rs` 都属于可执行代码；
- 安装工具时应检查来源；
- CI 中固定版本并使用 `--locked`。

### 文档

- 文档示例应当可编译、可执行、可复制；
- doctest 让文档与代码同步演进；
- 高质量 crate 的文档本身就是 API 的一部分。
