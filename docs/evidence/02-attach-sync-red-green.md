# 02 attach / sync RED / GREEN 证据

## RED

在 Slice 02 初始实现中，`attach` / `sync` 公开入口以 exit 1 fail-closed；新增 `tests/cli_attach_sync.rs` 先锁定 dry-run、apply、force、无 Node 环境和 metadata migration seam。

## GREEN

当前测试覆盖：

- attach dry-run 不修改已有目录；apply/force 只写受管 snapshot 文件，保留未受管文件；
- sync dry-run 不写入；本地修改的受管文件默认跳过，`--force` 才覆盖；
- v1 metadata 在事务中升级为 v2 并增加 `runtime.kind=native-rust`，缺失受管文件恢复；
- Node 2.x 生成的 v2 metadata（无 `runtime` 字段）也会按兼容迁移路径接受并写入 native runtime metadata；
- attach 可无 force 将 `repository_mode: template-source` 转为 `project-instance`；
- `PATH=""` 下 attach/sync 仍通过 Rust native verify，不启动 Node/PNPM；
- native `verify-template` 拒绝必需文件及其父路径上的符号链接；
- dry-run 输出逐项 `add` / `update` / `skip` / `conflict` / `legacy` / `removed` 与统计；未受管冲突保留本地文件并成功返回；
- transaction 失败恢复原目录，现有 `.git` 不在 snapshot 写路径中。

验证命令：`scripts/cargo test --locked --test cli_attach_sync`（10 tests）。

## 尚未覆盖

完整 legacy 路径迁移（旧 skills、旧 Ticket 目录、旧 scratch）、unsafe symlink/flat Ticket 冲突、模板删除报告、Windows launcher 和发布级 verifier 仍需后续垂直测试；因此不能把当前 Slice 02 视为最终 CLI 替代。
