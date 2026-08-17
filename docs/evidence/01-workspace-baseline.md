# 01 workspace baseline

日期：2026-08-17

本 workspace 只实现 `create-yss-spec --help` tracer slice。生产仓以 `rust-toolchain.toml` 固定 Rust 1.97.1；本机开发通过 `scripts/bootstrap-rust-toolchain.sh` 建立仓库内隔离工具链，再以 `scripts/cargo` 运行验证。该脚本当前仅支持 macOS arm64，不修改用户全局 Rust 安装。

通过命令：

- `scripts/cargo fmt --check`
- `scripts/cargo test --locked`
- `scripts/cargo clippy --locked -- -D warnings`
- `scripts/cargo build --release --locked`

以上命令在干净的仓库内 `.tooling/` 重建后通过。release binary 的体积仅供本地基线参考，不代表嵌入 snapshot、签名、三平台或发布体积。
