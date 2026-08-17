# yss-template-runtime

`create-yss-spec` 的 Rust 3.0 生产运行时。

当前仅处于受控 bootstrap：首个公开 seam 是 `create-yss-spec --help` 与 Node 2.x oracle 的一致性。它不是可安装、可替代 Node 或可发布的版本。

开发者在 macOS arm64 可运行 `scripts/bootstrap-rust-toolchain.sh`，随后使用 `scripts/cargo test --locked`。这是仓库开发引导，不是面向用户的安装方式。
