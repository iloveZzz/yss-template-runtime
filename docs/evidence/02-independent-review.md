# 02 独立审查证据

日期：2026-08-18

审查者：`/root/runtime_help_slice_review`（实现者之外的独立 Agent）。审查范围为当前生产 worktree、Slice 02 合同、Node 2.1.6 oracle 和 fresh verification；审查者未修改、提交或推送文件。

## 第一轮结果

第一轮 fresh verification 已通过 `scripts/cargo fmt --check`、`scripts/cargo test --locked`、`scripts/cargo clippy --locked -- -D warnings`、`scripts/cargo build --release --locked` 和 `git diff --check`，但发现以下 P1：snapshot binding 未逐字节校验 manifest/snapshot、Node v2 metadata 缺少 runtime 时无法迁移、TTY 交互只支持 EOF、attach 不转换 `template-source` identity、native verify 接受 required-file symlink、dry-run 输出过于摘要化、unmanaged conflict 与 Node 语义不同，以及本证据文件缺失。

## 修复记录

- 将 `template.manifest.json` 和 `template.snapshot.json` 作为 `__yss_runtime/` 绑定文件随 archive 嵌入；运行时校验 archive SHA-256、manifest SHA-256、manifest 排除/渲染数组、snapshot commit/hash 和 encoded paths；新增 `tests/snapshot_binding.rs`。
- v2 metadata 缺少 native `runtime` 时进入兼容迁移；增加 Node 2.x v2 fixture 回归。
- TTY 使用 `stdin.is_terminal()` 逐问逐答，pipe/buffered fixture 保留 EOF 读取路径。
- attach 对 `repository_mode: template-source` 记录 identity conversion 并无 force 写为 `project-instance`。
- `safe_join`、native verify 和 metadata writer 拒绝 required-file/父路径符号链接，并校验 managed file content hashes。
- attach/sync dry-run 输出逐项操作、迁移、冲突和统计；sync 未受管冲突保留本地文件并成功返回。
- 增加已有空目录失败回滚、Node v2 migration、identity conversion、symlink verify、unmanaged conflict 等回归。

## 第三轮结果与当前状态

第三轮只读复审的 fresh 结果：P0=0、P1=2、P2=3。前一轮的 managedFiles 完整集合校验、Node v2 冲突基线保留、`init --force` 保留/恢复 `.git`、Node/Rust fixture 对照和本证据文件缺口均已关闭。

第三轮指出的两个 P1 已处理：v2 metadata 缺少 `runtime` 时现在仍校验 `templateCommit`、`templateSource`、manifest 版本、managedFiles 等核心字段，只放宽 runtime 专属字段；本文件已记录最终审查状态。P2 保留为完整 binding schema 的进一步扩展、PTY 自动化覆盖和 Windows/发布级 verifier 等后续边界。

第三轮当时的历史 fresh 验证通过 `scripts/cargo fmt --check`、`scripts/cargo test --locked`（23 tests）、`scripts/cargo clippy --locked -- -D warnings`、`scripts/cargo build --release --locked`、`git diff --check`，并通过 harness 的 `scripts/verify-template`。Slice 02 仍未进入 Git checkpoint；本文件不把它宣称为 Node 完整替代、RC 或 stable。

## 第四轮发现与修复

第四轮只读复审发现一个 P1：`metadataSchemaVersion` 使用非整数值（例如 `2.5`）时不能降级为 legacy。现已改为只接受正整数；缺失字段仍按 v1 迁移，非整数、零、负数和其他类型均 fail-closed，并新增 `sync_rejects_non_integer_metadata_schema_version` 回归。

第四轮保留的 P2 是完整 binding schema 扩展、PTY 自动化覆盖以及 Windows/发布级 verifier；它们属于后续切片边界，不改变 Slice 02 的当前合同范围。

本轮修复后的 fresh 验证：`scripts/cargo test --locked` 为 26 tests 全部通过，`scripts/cargo clippy --locked --all-targets --all-features -- -D warnings`、`scripts/cargo build --locked --release`、`git diff --check` 全部通过；release 候选及启动证据已刷新。

## 第五轮发现与修复

第五轮复审发现 v2 metadata 的 `variables` 与 `cliVersion` 仍可能缺失或类型错误而继续同步。现已在 v2 公共字段校验中要求 `variables` 为 JSON object、`cliVersion` 为非空字符串，并新增两条 `sync` fail-closed 回归；v1 metadata 迁移路径保持兼容。

## 最终独立复审

2026-08-18 最终只读复审结论：P0=0、P1=0、P2=4。审查者确认 `metadataSchemaVersion`、v2 `variables` / `cliVersion`、runtime 缺失时的公共字段、managedFiles 集合、符号链接和事务回滚均已由代码与回归覆盖。P2 为完整 binding schema 扩展、PTY 自动化、Windows launcher 以及发布级 verifier，均保留到后续切片。

最终 fresh verification 为 26 tests 全部通过，`scripts/cargo fmt -- --check`、`scripts/cargo clippy --locked --all-targets --all-features -- -D warnings`、`scripts/cargo build --locked --release`、`git diff --check` 和 harness `scripts/verify-template` 全部通过。Slice 02 仍未提交或推送；本证据不把它宣称为 Node 完整替代、RC 或 stable。
