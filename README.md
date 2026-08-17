# yss-template-runtime

`create-yss-spec` 的 Rust 3.0 生产运行时。

当前发布的是 `v3.0.0-rc.1` 双平台非稳定预览：`--help`、显式/交互式 `init`、`attach`、`sync` 的 snapshot 与 metadata seam 已在 Rust 中逐步实现，并以 Node 2.x fixture 做行为回归。该预览只覆盖 macOS arm64 与 Linux x64 musl，资产未签名，Windows 延后；它不是稳定版，也不代表已替代 Node。

开发者在 macOS arm64 可运行 `scripts/bootstrap-rust-toolchain.sh`，随后使用 `scripts/cargo test --locked`。这是仓库开发引导，不是面向用户的安装方式。

本地验证示例：

```text
create-yss-spec --project-name <name> --business-domain <domain> --target-dir <dir>
create-yss-spec attach --target-dir <dir> --project-name <name> --business-domain <domain> --dry-run|--apply [--force]
create-yss-spec sync [--target-dir <dir>] [--dry-run] [--force]
create-yss-spec verify-template [--target-dir <dir>]
```

这些命令会消费 binary 内嵌的固定模板 snapshot；实例操作不需要 Node/PNPM，但维护侧模板 gate 仍保留原有 Node/PNPM 工具链。

## 预览下载

GitHub draft/prerelease Release 会由 Tag `v3.0.0-rc.1` 触发 GitHub Actions 构建，提供以下目标平台资产：

- `aarch64-apple-darwin`：macOS arm64
- `x86_64-unknown-linux-musl`：Linux x64 musl

每个 `.tar.gz` 资产都附带 SHA-256 校验文件。该预览不提供 Windows、签名、公证或安装器；正式稳定发布仍需单独完成这些门禁。
