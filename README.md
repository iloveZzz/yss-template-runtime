# yss-template-runtime

`create-yss-spec` 的 Rust 3.0 生产运行时。

当前处于受控 Slice 02：`--help`、显式/交互式 `init`、`attach`、`sync` 的 snapshot 与 metadata seam 已在 Rust 中逐步实现，并以 Node 2.x fixture 做行为回归。它仍不是可安装、可替代 Node 或可发布的版本。

开发者在 macOS arm64 可运行 `scripts/bootstrap-rust-toolchain.sh`，随后使用 `scripts/cargo test --locked`。这是仓库开发引导，不是面向用户的安装方式。

本地验证示例：

```text
create-yss-spec --project-name <name> --business-domain <domain> --target-dir <dir>
create-yss-spec attach --target-dir <dir> --project-name <name> --business-domain <domain> --dry-run|--apply [--force]
create-yss-spec sync [--target-dir <dir>] [--dry-run] [--force]
create-yss-spec verify-template [--target-dir <dir>]
```

这些命令会消费 binary 内嵌的固定模板 snapshot；实例操作不需要 Node/PNPM，但维护侧模板 gate 仍保留原有 Node/PNPM 工具链。
