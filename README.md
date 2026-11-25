# chrono-harness-judge-routes

将测试义务解析为已登记的方法与工具。

产品和专属测试只从 crates.io 获取 Rust 依赖。运行 `mise exec -- ./check.sh` 构建程序、执行路由测试，并验证实际运行及重读计划时采用的操作顺序、去重、退役选择、回执匹配和资源上限。宿主安装固定版本的 `chrono-harness`，用于核对原生判官请求、观察工具进程和读取固定提交的源码。编译器输出、源码分类及选中证明根的 Verus 检查随 runner 发布。

| 整 crate 证明操作 | 原生使用方和业务条件 | 独立验证的边界 |
| --- | --- | --- |
| `operation-bounds` 与 `operation-bounds-tests` | 声明与留存计划的超时和输出上限 | 正超时、1 字节至 64 MiB 闭区间 |
| `route-admission` 与 `route-admission-tests` | `Plan::steps` 只接纳尚未声明的操作。重读和构建执行图时，每个前驱都须已完成。接受登记判官的退役决定前，核对文档／收集请求身份与完整测试集合。退役时加入替代项，缺少替代时由下游负责决定是否可延后。回执绑定计划与操作，各输出流有独立长度、摘要和文本观察 | 首次插入与重复插入、缺一个与包含全部前驱、错误身份、缺少／多出／替换一个退役测试、替代与原错误、无下游的缺替代、不同计划或操作、单流超限和错误摘要 |

本仓的 `check.sh` 通过宿主登记按 Git 变更选择 routes 单元。四个 Verus 操作各自产生原始 JSON、编译库和 dep-info，原生操作核对 Cargo 实际编译的声明、选择和比较源码，运行独立的消费者用例。CI 汇总读取同一候选的单元回执。拓扑队列按 Rust 的有序映射遍历就绪项，原生测试核对结果顺序。登记判官提供退役和替代证据，runner 提供真实进程与编译输出检查。源码证明依赖集合键的比较契约。命令参数、文件路径、环境、序列化、哈希与有损 UTF-8 解码由原生适配层测试。


[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/judge-routes)

判官模式通过 PATH 调用宿主安装的 `chrono-judge-filemap impact-summary`。本仓 mise 配置固定了 FILEMAP v0.1.0-alpha.1 及两平台 SHA-256。FILEMAP 解码整个影响文档，routes 检查 schema 与本次变更的身份，再用受影响文件和测试生成执行计划。

`chrono-judge-routes global-selection` 供输入组合工具选择全局测试。标准输入为一个 JSON 对象，`config` 是配置入口路径，`base` 和 `candidate` 各自按路径提供该端点的登记文档，`tests` 和 `retired_tests` 是 FILEMAP 给出的测试 ID 列表。输出是排序去重后的 JSON 数组，包含已登记的替代测试。共同退役须有已登记的下游判官负责。查询失败时返回 2，原因写入标准错误。这个查询只读取标准输入。

在本仓的 GitHub Releases 选择平台二进制及同标签的 SHA256SUMS，校验后即可安装或切换版本。

完整 CI 收集从宿主根目录调用 `collected-selection-v1`。stdin 提供绑定当前检出的 collect `request`、FILEMAP 选定的 `tests` 和 `retired_tests`。历史转换需要 `migration`，其中 `units` 必须是登记要求的执行单元，`originals` 是这些单元的原始清单。routes 根据解释后的两端登记核对历史测试的替代或退役，向 stdout 返回排序去重的测试数组。输入和输出错误写入 stderr 并退出 2。

`check-plan-receipts-v1` 接收 JSON 对象 `{plan, receipts}`。调用方按操作名提供实际回执，命令使用 routes 的计划身份与回执核对逻辑。成功输出计划身份 `plan` 和计划内各操作的 `operations` 映射。匹配的回执对应 null，缺少或不匹配的回执对应原始错误。调用方按自己的完整性和执行成功条件采用这些结果。无效计划、无效输入或输出失败时返回 2 并写 stderr。

执行单个操作后，projects 使用 `check-operation-receipt-v1`。输入中的 `plan` 是执行前已验完整计划的身份，`root`、`environment`、`operation` 和 `tool` 从该计划取得，`receipt` 是本次操作的原始回执。输出的 `plan`、`operation` 标明核对对象，`error` 为 null 或原始校验错误。离线收集整份计划及回执仍使用 `check-plan-receipts-v1`。

`plan-context-v1` 从 stdin 读取 `request`、`effective_inputs` 和按操作名索引的原始参数模板 `templates`。stdout 包含 `request_id`、routes 的请求绑定 `binding` 和展开后的 `expanded`。重放收集计划时，`collection-plan-v1` 接收原坐标 `root`、`binding`、全局测试 `selected`、已解释的 `filemap` 和 `projects`、每个测试的操作声明列表 `test_operations` 以及 `environment`，返回完整的原生执行计划。这两个查询使用同一套绑定和排序逻辑，成功退出 0，错误写入 stderr 并退出 2。

projects 在宿主选定的 routes 发行上使用以下查询。每个查询从 stdin 读取一个 JSON 值，通过 stdout 返回 JSON，失败时写 stderr 并退出 2。

- `check-plan-v1` 校验原始计划的身份、操作顺序、资源声明和执行限制，返回 `plan` 身份。
- `dispatch-plan-v1` 在执行前校验同一原始计划，返回其 `plan` 身份、`dispatch.max_running` 并发上限和 `dispatch.order`。次序为覆盖全部操作的零基下标，优先项按登记次序排列，其余沿用原计划次序。projects 按这个次序查找就绪操作，并与正在运行的操作核对原计划中的资源和输出路径。查询和回放均可在空 PATH 下使用。
- `unit-declarations-v1` 接收原始 `request`、`effective_inputs`、两组登记 `filemap` 和 `projects`、每个测试的 `test_operations`、全局选择 `global` 和其中一个执行单元的选择 `unit`。返回请求身份、绑定、全局操作图及该单元展开后的操作。
- `retained-tool-v1` 核对采集时的 `tool`、`program`、`version_argv`、`environment`、`root` 及 `expected_version`，成功返回工具的 `sha256`。收集端可在原执行目录和工具已经清除后核对此凭证。

限定范围的 CI 把已选测试和本仓登记的计划、方法及执行动作放在 `graph` 对象的 `selected`、`plans`、`methods`、`execute` 字段中。

- 调度和收集从标准输入向 `scoped-operations-v1` 提交 `{graph, binding}`。省略 binding 时返回原始参数模板，提供 binding 时展开每个操作的参数。输出包含选中的计划和依赖排序后的操作。
- 执行前，调用方在宿主根目录运行 `prepare-scoped-plan-v1`，提供 `graph`、`binding`、`declarations`、`environment`、`reused` 和可选的 `scheduling`。routes 校验方法、限制和调度声明，获取缺少的工具版本观察，输出有身份绑定的完整计划。两个命令成功时向 stdout 写 JSON 并退出 0，失败时向 stderr 写原因并退出 2。

完整 CI 在调度与收集时向 `full-operations-v1` 提交已解释的 `filemap`、`projects`、按测试名索引的终结操作声明 `test_operations` 和本次选择 `selected`。调度时额外传入 `execution_units`，routes 先验证所有测试和共享操作的单元归属，再返回 `required_units`。收集时省略该字段，返回的 `required_units` 为 null。两种调用都返回选中的计划、按依赖排序的操作及供证明核对使用的完整方法声明。命令从 stdin 读 JSON，成功向 stdout 写 JSON，拒绝输入时向 stderr 写原因并退出 2。

完整和限定范围的 CI 可以调用 `source-plan-v1` 核对源码证明计划。stdin 提供解释后的 `previous`（初始提交为 null）和 `candidate`，每个端点含固定 `commit`、`filemap`、`projects` 和产物目录 `artifacts`。另传受影响的节点 `affected`、登记方法 `methods` 和有序 `operations`。已绑定 Git 的调用方传入 `git.config` 和原读者的 `git.git_facts`（可略去进程数组）；旧式未绑定读者传 null。

在宿主检出中执行查询。需要连续性证据时，工具从固定提交实际观察树和文件，按配置核对 Git 工具及环境。空路径集可完全在内存中核对。stdout 的 `previous`、`candidate` 标识所查提交，`git_facts` 保存本次读者产生的观察。调用方必须处理 `error`，null 表示当前计划通过，字符串保留原始拒绝原因。请求无法解码或提交格式无效时退出 2。

scoped CI 可向 `prepare-scoped-plan-v1` 提供原始报告的宿主相对路径 `report_path`，在 `binding` 中提供原请求字段及字符串 `run`。routes 从报告路径与运行身份生成 `operation_state`。收集时向 `check-scoped-plan-v1` 提交 `{plan, binding, report_path}`；它校验原计划身份与整份绑定，成功返回 `{"plan":"<identity>"}`。

`global-selection` 和 `collected-selection-v1` 通过 PATH 上已安装的 `chrono-judge-registration test-retirements-v1` 解释两个端点。文档模式传入固定登记，收集模式传入原封存请求和迁移原件。routes 按输入顺序处理各退役决定，对替代测试和 FILEMAP 的测试取并集。

`collection-plan-v1`、`full-operations-v1` 和 `unit-declarations-v1` 把收到的 FILEMAP 和项目文档原样交给宿主选定的 `chrono-judge-registration execution-declarations-v1`。routes 使用返回的计划和方法验证单元归属、合并共享操作并展开参数。收集计划仍按原执行目录重建，登记程序从本次检出的 PATH 解析。

普通判官把封存请求交给已安装的 registration 的 `route-views-v1`。routes 读取 FILEMAP 的影响、处理本次退役选择，再通过 `route-inputs-v1` 核对选中范围的输入。调用保留请求的原始工作目录和 Git 观察，回执查询仍可在原工具与原检出已经移除后运行。

宿主通过 PATH 配置独立安装的 runner。若 PATH 中找不到 runner，且调用方通过其工具目录内的显式路径启动 routes，routes 使用该路径旁的 `chrono-harness` 别名。这样每个别名可以指向各自独立的发布与安装目录。

Registration readiness uses `chrono-judge-routes check-unit-assignments-v1`. It reads one JSON object with `execution_units` and the already interpreted `plans` on stdin. It writes `null` on success. Invalid input or assignment exits with code 2 and the diagnostic on stderr.

## 本地检查产物

从主检出运行 `mise exec -- ./check.sh`。检查持有工作树租约，结束时控制台给出原报告路径。准备输入和原报告在 `.chrono-harness/state/preparation/`，单元结果在 `.chrono-harness/state/routes/`，汇总结果在 `.chrono-harness/state/collection/`。证明编译件和依赖清单位于本次执行报告所属目录。需要留存的文件应在当次滚动前复制到持久存储。GitHub Actions 对上传的单元结果保留一天。汇总结果采用仓库的默认保留期，在本次核对的实际上传中约为 90 天。

在主检出运行 `mise exec -- chrono-worktree maintain --host-root . --config .chrono-harness/worktree.json
```` 可执行一轮工作树维护。独占执行者核对身份、租约与源码保全情况，然后回收符合条件的产物。

每次维护统一清理本批次已登记的准备、单元、汇总和原生检查临时目录 `.chrono-harness/state/native-check/`。刚写入的文件也使用本批次截止时刻。工作树租约仍在使用时推迟到下一次能取得排他租约的维护。系统调度服务、主检出和工具必须在当时可用。首次登记时已存在的目录受保护，须核对内容、转存所需文件，再按工作树工具的历史缓存接管接口逐项采用。安装包位于 mise 共享目录，`.chrono-harness/bin/` 只包含本仓使用的工具链接。

对新任务使用已登记的工作树。普通关联工作树满 23 小时可复用，超过 24 小时可在维护中回收。处置前会保全源码和 Git 改动。工作树操作记录以一天为保留窗口。检查结束、结果已交接且缓存成功处置后，首份检查报告和这一代的最终处置收据也会在后续的到期维护中释放。处置中的原件继续保管。
