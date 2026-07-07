# Rust 学习笔记

Rust 知识点思维导图整理，按学习时间记录。

## 知识图谱

```mermaid
graph LR
    A["06-29 错误处理"] --> B["06-30 Option / Result"]
    B --> C["06-30 泛型与 Trait"]
    C --> D["07-01 生命周期"]
    D --> E["07-02 测试体系"]
    E --> F["07-03 minigrep 实战"]
    F --> G["07-05 闭包"]
    G --> H["07-05 迭代器"]
    F --> I["07-06 Cargo"]
    H --> J["07-07 智能指针"]

    C -.-> G
    D -.-> G
    G -.-> H
    D -.-> J
```

| 日期 | 知识点 | 文件 |
|------|--------|------|
| 2026-06-29 | 错误处理机制 | [rust_error_mechanism.xmind](notes/rust_error_mechanism.xmind) |
| 2026-06-30 | Option / Result 方法 | [rust_option_result_methods.xmind](notes/rust_option_result_methods.xmind) |
| 2026-06-30 | 泛型与 Trait 工程实践 | [rust_generics_and_trait_engineering_practice.xmind](notes/rust_generics_and_trait_engineering_practice.xmind) |
| 2026-07-01 | 生命周期 (Lifetime) | [rust_lifetime.xmind](notes/rust_lifetime.xmind) |
| 2026-07-02 | 测试体系、cargo test | [rust_testing_basics_cargo_test_engineering_practice.xmind](notes/rust_testing_basics_cargo_test_engineering_practice.xmind) |
| 2026-07-03 | 第12章 minigrep 项目 | [思维导图](notes/rust_chapter12_minigrep_project_summary.xmind) · [代码](projects/minigrep/) |
| 2026-07-05 | 闭包 (Closure) | [rust_closure_summary.xmind](notes/rust_closure_summary.xmind) |
| 2026-07-05 | 迭代器 (Iterator) | [rust_iterator_summary.xmind](notes/rust_iterator_summary.xmind) |
| 2026-07-06 | Cargo 知识体系 | [rust_cargo_summary.xmind](notes/rust_cargo_summary.xmind) |
| 2026-07-07 | 智能指针 (所有权 / Deref / Drop) | [rust_smart_pointers_ownership_deref_drop.xmind](notes/rust_smart_pointers_ownership_deref_drop.xmind) |
