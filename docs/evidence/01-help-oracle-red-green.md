# 01 `--help` Node oracle：RED / GREEN

## Oracle

Node 2.1.6 的公开命令：

```text
node /Users/zhudaoming/Projects/create-yss-spec/bin/create-yss-spec.js --help
```

输出固定于 `fixtures/node-oracle/help.txt`。

## RED

在没有 `src/main.rs` 时执行 `cargo test`，Cargo 以“找不到 bin `create-yss-spec`”失败。这证明测试先于 CLI 实现存在。

## GREEN

最小 Rust 实现仅接受 `--help` / `-h`，stdout 与 fixture 完全相同、stderr 为空、退出码为 0。`tests/help_cli.rs` 经 binary 公开入口断言这些可观察结果。

该测试也锁定尚未实现命令（例如 `attach`）必须 stderr 输出当前受控 bootstrap 提示、stdout 为空并退出 2，防止在完整命令实现前误进入不确定行为。

## 边界

默认 init、attach、sync 和其他参数尚未实现；它们在当前构建中明确退出 2，不能将本 slice 当作 CLI 替代结论。
