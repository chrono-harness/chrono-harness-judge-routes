# chrono-harness-judge-routes

将测试义务解析为已登记的方法与工具。

运行 `./check.sh` 验证独立组件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-routes)

判官模式通过 PATH 调用宿主安装的 `chrono-judge-filemap impact-summary`。本仓 mise 配置固定了 FILEMAP v0.1.0-alpha.1 及两平台 SHA-256。FILEMAP 解码整个影响文档，routes 检查 schema 与本次变更的身份，再用受影响文件和测试生成执行计划。

`chrono-judge-routes global-selection` 供输入组合工具选择全局测试。标准输入为一个 JSON 对象，`config` 是配置入口路径，`base` 和 `candidate` 各自按路径提供该端点的登记文档，`tests` 和 `retired_tests` 是 FILEMAP 给出的测试 ID 列表。输出是排序去重后的 JSON 数组，包含已登记的替代测试。共同退役须有已登记的下游判官负责。查询失败时返回 2，原因写入标准错误。这个查询只读取标准输入。

在本仓的 GitHub Releases 选择平台二进制及同标签的 SHA256SUMS，校验后即可安装或切换版本。
