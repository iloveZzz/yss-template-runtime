# 02 启动与单文件候选证据

日期：2026-08-18

在 macOS arm64 本地 release build（`scripts/cargo build --release --locked`）上，当前候选为一个 `Mach-O arm64` 可执行文件：

- 路径：`target/release/create-yss-spec`
- 体积：29,048,240 bytes（约 27.7 MiB）
- SHA-256：`3394ce315e56fe5e0c32af128bd2a18a5f3da8c993d2cf2aadd9925b6cdd6b3d`
- `--help` wall time：`0.00s`（`/usr/bin/time -p`，本机单次冷启动观测）
- `init` 生成 5,234 个文件 wall time：`0.78s`（同一台 macOS arm64、本地临时目录、单次观测）

同一台机器、同一 snapshot 的 Node 2.1.6 oracle 单次对照为：`--help` `0.09s`、`init` `11.01s`（同样生成 5,234 个文件）。这只是方向性本机观测，未做多次采样、统计区间或跨平台基准，不能直接作为正式性能承诺。

Rust runtime 将模板 snapshot、manifest 和 snapshot binding JSON 编译进该 binary；实例 `init` / `attach` / `sync` / `verify-template` 不要求 Node、npm、PNPM 或网络。macOS Mach-O 仍链接系统 `libSystem` / `libiconv`，所以“单文件分发”指用户侧一个 runtime 文件，不等同于静态链接或已完成签名/公证。

该数据是 Slice 02 的本机基线，不是跨平台性能结论，也不代表 RC/stable、签名、notarization、安装器或 release asset 已完成。
