# 02 固定模板 snapshot 绑定证据

日期：2026-08-17

生产 Rust runtime 将 Node 2.1.6 package 已生成的模板目录作为不可变输入，固定到：

- `templateCommit`: `68c367a13d5006cca83f1c5e369678af28c4bf15`
- Node snapshot `snapshotHash`: `f4276bfa8e6ca7781f905372d912f8fd9ba806566e212550b4548eda0f877387`
- `template.manifest.json` SHA-256：`48549af09ac85a9e0caf97d9342e8ee31b1cc8b608704bc9f1aa0d546f9a635c`
- 内嵌 `assets/template.snapshot.tar` SHA-256：`f72c6bd76c48247ec31245f150be257b9eeb4388da32a29a5e958d3b2600778e`
- archive 内容：5,235 个文件、2,957 个目录；其中 `__yss_runtime/` 绑定 manifest/snapshot JSON；运行时不向实例输出这两个绑定文件。保留可执行 mode，拒绝符号链接和特殊文件。

运行时启动前校验 archive SHA-256，并解析绑定的 manifest/snapshot JSON，逐项核对 manifest render paths、template commit 与 snapshot hash；路径经过 `..` / absolute / root 拒绝和 dotfile encoded-path 解码。`AGENTS.md`、`README.md`、`yss-project.yaml` 由 Rust 重新渲染，其他 snapshot 文件按字节复制；不启动 Node、不下载模板。

当前证据仅覆盖这个固定 snapshot 的本地嵌入和 init/attach/sync 消费；尚不代表 snapshot 发布签名、跨平台 artifact 或 RC/stable release。
