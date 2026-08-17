# 01 隔离工具链引导证据

日期：2026-08-17

`scripts/bootstrap-rust-toolchain.sh` 是 bootstrap slice 的开发工具，不是最终用户安装器。

- 仅接受 `Darwin-arm64`；其他平台直接退出 2。
- 从固定的 Rust 官方 `rustup-init` 1.29.0 URL 下载，并先校验 SHA-256 `aeb4105778ca1bd3c6b0e75768f581c656633cd51368fa61289b6a71696ac7e1`。
- 工具链固定为 Rust 1.97.1，包含 `rustfmt` 与 `clippy`，安装在被 Git 忽略的 `.tooling/`。
- 为处理中断下载，脚本会仅卸载同一隔离目录内的目标 Rust toolchain 后重装；它不接触用户全局 Rust toolchain。

从隔离目录重新建立后，`scripts/cargo --version` 输出 `cargo 1.97.1`，随后以下命令均通过：

- `scripts/cargo fmt --check`
- `scripts/cargo test --locked`
- `scripts/cargo clippy --locked -- -D warnings`
- `scripts/cargo build --release --locked`

这只证明 macOS arm64 的开发 bootstrap 和首个 help oracle；不证明三平台构建、用户安装器、签名或发布。
