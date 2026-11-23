# chrono-harness-judge-routes

将测试义务解析为已登记的方法与工具。

运行 `./check.sh` 验证独立组件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-routes)

判官模式通过 PATH 调用宿主安装的 `chrono-judge-filemap impact-summary`。本仓 mise 配置固定了 FILEMAP v0.1.0-alpha.1 及两平台 SHA-256。FILEMAP 解码整个影响文档，routes 检查 schema 与本次变更的身份，再用受影响文件和测试生成执行计划。

`chrono-judge-routes global-selection` 供输入组合工具选择全局测试。标准输入为一个 JSON 对象，`config` 是配置入口路径，`base` 和 `candidate` 各自按路径提供该端点的登记文档，`tests` 和 `retired_tests` 是 FILEMAP 给出的测试 ID 列表。输出是排序去重后的 JSON 数组，包含已登记的替代测试。共同退役须有已登记的下游判官负责。查询失败时返回 2，原因写入标准错误。这个查询只读取标准输入。

在本仓的 GitHub Releases 选择平台二进制及同标签的 SHA256SUMS，校验后即可安装或切换版本。

完整 CI 收集从宿主根目录调用 `collected-selection-v1`。stdin 提供绑定当前检出的 collect `request`、FILEMAP 选定的 `tests` 和 `retired_tests`。历史转换需要 `migration`，其中 `units` 必须是登记要求的执行单元，`originals` 是这些单元的原始清单。routes 根据解释后的两端登记核对历史测试的替代或退役，向 stdout 返回排序去重的测试数组。输入和输出错误写入 stderr 并退出 2。

`check-plan-receipts-v1` 接收 JSON 对象 `{plan, receipts}`。调用方按操作名提供实际回执，命令使用 routes 的计划身份与回执核对逻辑。成功输出计划身份 `plan` 和计划内各操作的 `operations` 映射。匹配的回执对应 null，缺少或不匹配的回执对应原始错误。调用方按自己的完整性和执行成功条件采用这些结果。无效计划、无效输入或输出失败时返回 2 并写 stderr。

`plan-context-v1` 从 stdin 读取 `request`、`effective_inputs` 和按操作名索引的原始参数模板 `templates`。stdout 包含 `request_id`、routes 的请求绑定 `binding` 和展开后的 `expanded`。重放收集计划时，`collection-plan-v1` 接收原坐标 `root`、`binding`、全局测试 `selected`、已解释的 `filemap` 和 `projects`、每个测试的操作声明列表 `test_operations` 以及 `environment`，返回完整的原生执行计划。这两个查询使用同一套绑定和排序逻辑，成功退出 0，错误写入 stderr 并退出 2。

projects 在宿主选定的 routes 发行上使用以下查询。每个查询从 stdin 读取一个 JSON 值，通过 stdout 返回 JSON，失败时写 stderr 并退出 2。

- `check-plan-v1` 校验原始计划的身份、操作顺序、资源声明和执行限制，返回 `plan` 身份。
- `unit-declarations-v1` 接收原始 `request`、`effective_inputs`、两组登记 `filemap` 和 `projects`、每个测试的 `test_operations`、全局选择 `global` 和其中一个执行单元的选择 `unit`。返回请求身份、绑定、全局操作图及该单元展开后的操作。
- `retained-tool-v1` 核对采集时的 `tool`、`program`、`version_argv`、`environment`、`root` 及 `expected_version`，成功返回工具的 `sha256`。收集端可在原执行目录和工具已经清除后核对此凭证。

限定范围的 CI 把已选测试和本仓登记的计划、方法及执行动作放在 `graph` 对象的 `selected`、`plans`、`methods`、`execute` 字段中。

- 调度和收集从标准输入向 `scoped-operations-v1` 提交 `{graph, binding}`。省略 binding 时返回原始参数模板，提供 binding 时展开每个操作的参数。输出包含选中的计划和依赖排序后的操作。
- 执行前，调用方在宿主根目录运行 `prepare-scoped-plan-v1`，提供 `graph`、`binding`、`declarations`、`environment`、`reused` 和可选的 `scheduling`。routes 校验方法、限制和调度声明，获取缺少的工具版本观察，输出有身份绑定的完整计划。两个命令成功时向 stdout 写 JSON 并退出 0，失败时向 stderr 写原因并退出 2。

完整 CI 在调度与收集时向 `full-operations-v1` 提交已解释的 `filemap`、`projects`、按测试名索引的终结操作声明 `test_operations` 和本次选择 `selected`。调度时额外传入 `execution_units`，routes 先验证所有测试和共享操作的单元归属，再返回 `required_units`。收集时省略该字段，返回的 `required_units` 为 null。两种调用都返回选中的计划、按依赖排序的操作及供证明核对使用的完整方法声明。命令从 stdin 读 JSON，成功向 stdout 写 JSON，拒绝输入时向 stderr 写原因并退出 2。
