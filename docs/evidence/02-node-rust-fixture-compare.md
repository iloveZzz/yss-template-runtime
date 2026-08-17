# 02 Node / Rust 同 fixture 对照证据

日期：2026-08-17

对照对象：`/Users/zhudaoming/Projects/create-yss-spec` Node 2.1.6 oracle 与 `/Users/zhudaoming/Projects/yss-template-runtime/target/release/create-yss-spec` 当前 Rust binary。两者使用相同项目变量和同一 frozen snapshot；每次都在独立临时目录执行。

## 已执行对照

### help

Node stdout 与 `fixtures/node-oracle/help.txt` 一致；Rust stdout 与 Node `--help` 使用 `diff -q` 比较为 `exact`，两者 exit `0`、stderr 为空。

### init

最近一次同 fixture 结果：

```text
node_init_status=0 rust_init_status=0
node_files=5233 rust_files=5233
init_diff_without_metadata=0
help_equal=yes
```

这里的文件树比较排除 `.yss-template.json`（Node 2.x 与 native runtime metadata 的预期差异），其余内容逐文件 `diff -qr` 为零差异；两侧生成文件 mode 保持一致。两侧 init stdout 的下一步提示相同，目标目录均无 `.git`。

### attach dry-run

同一空目标（仅含一个未受管 `runtime.txt`）两侧均 exit `0`、均保持目标未变、统计均为：

```text
统计：新增 5233，一致 0，身份转换 0，冲突 0，unsafe 0
```

Node 和 Rust 均使用最多 40 项逐项清单加省略提示；Rust 保留 native 运行时自己的路径迁移和错误文案。apply/force、rollback、metadata migration、symlink 和 identity conversion 由 `tests/cli_init.rs` / `tests/cli_attach_sync.rs` 的 public-binary 回归覆盖。

### Node metadata → Rust sync

实际用 Node 2.1.6 先生成实例，再用 Rust `sync`（`PATH=""`）迁移；结果 exit `0`，metadata 变为 schema v2 且 `runtime.kind=native-rust`，template commit 保持 `68c367a13d5006cca83f1c5e369678af28c4bf15`。包含本地 README 修改的 fixture 会保留本地内容，并保留原 managed baseline hash。

## 仍然明确的差异与边界

- `init --force` 是 3.0 有意安全变更：Rust 保留活动 `.git`，Node 2.x 旧行为会整体替换目标；这是合同冻结的迁移差异，不宣称 byte-exact。
- Node 维护侧三项完整 template gate 仍依赖 Node/PNPM；实例侧 `verify-template` 是 native seam，当前校验 snapshot 对应 managed 文件集合、hash、metadata 和安全路径。
- Windows launcher、跨平台 artifact、签名/notarization、安装器和 RC/stable release 不属于 Slice 02；本证据不把当前 binary 标为可发布。
