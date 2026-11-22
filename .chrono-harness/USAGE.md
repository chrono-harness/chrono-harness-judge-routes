# chrono-harness-judge-routes 使用说明

将测试义务解析为已登记的方法与工具。

开发分支为 `dev`。日常 PR 合入开发分支，正式版从 `main` 发布。

首次检出运行 `mise trust` 和 `mise run harness-setup`。工具版本见 [.config/mise/conf.d/chrono-harness.toml](../.config/mise/conf.d/chrono-harness.toml)。

开始每项任务时运行 `mise exec -- chrono-instructions rules --host-root .`，读取并遵循标准输出中的上游规则。完整规则和本仓开发步骤见根 [CLAUDE.md](../CLAUDE.md)。

用 `mise exec -- chrono-pr pr-reminders` 查看本地提醒，再运行 `mise exec -- chrono-pr pr-status --pr NUMBER --head-sha SHA` 查询 GitHub。

## 本仓验证

产品位于 [`crates/judge-routes`](../crates/judge-routes)。原生测试位于 [`crates/judge-routes-tests`](../crates/judge-routes-tests)。在仓库根目录运行 `mise exec -- ./check.sh` 执行本仓的检查。
