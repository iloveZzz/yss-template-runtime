# 02 init RED / GREEN 证据

## RED

在只实现 `--help` 的 Slice 01 上，`tests/cli_init.rs` 的 dry-run 与 apply 均以 exit 2 失败；测试先锁定 public binary seam，而非内部函数。

## GREEN

当前测试覆盖：

- 显式参数 init dry-run：输出 frozen snapshot commit/hash，目标目录保持不存在；
- 显式参数 init apply：解包 snapshot、解码 dotfiles、渲染三文件、写 `.yss-template.json` metadata v2 与 native runtime metadata；
- `--force`：先建立外部备份，`git init` 失败时恢复原目标树；
- `--force` 替换时保留原目标的活动 `.git`，其余受替换范围内的未受管文件进入外部备份；
- 已存在空目录在失败时保留目录本身并清理本次部分写入；
- 无参数 buffered stdin：按 Node 2.x 顺序读取项目名、业务领域、团队规模和目标目录；
- 真实 TTY 使用逐问逐答 `read_line`，非 TTY 才使用已关闭 stdin 的 buffered fixture；
- `PATH=""` 下 init 不依赖 Node/PNPM。

验证命令：`scripts/cargo test --locked --test cli_init`。

## 边界

init 的完整模板维护校验仍由模板仓维护侧 gate 负责；实例侧 native verify、attach/sync 事务和 legacy migration 在同一 Slice 02 继续收敛，尚未形成发布结论。
