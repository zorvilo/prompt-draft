# Rust 分层与模块约定

词稿保持一个 Rust crate，参考 CC Switch 的包内分层以及 DBX 对核心逻辑与桌面入口的拆分方式。模块按职责划分，不引入多 crate workspace、仓储 trait 或依赖注入容器。

```text
src-tauri/src/
  main.rs                   二进制入口，调用 prompt_draft::run
  lib.rs                    插件、数据目录、状态初始化及 IPC 注册
  state.rs                  数据库连接、HTTP 客户端、请求与互斥状态
  error.rs                  IPC 错误边界，保持字符串返回格式
  commands/
    workspace.rs            工作区读取与增量保存
    providers.rs            密钥、连接测试与模型列表
    optimization.rs         优化、预览和取消的 IPC 适配
    usage.rs                用量统计与价格目录的 IPC 适配
    backup.rs               文件导出、完整备份和恢复的 IPC 适配
    update.rs               更新检查、下载、安装的 IPC 适配
    desktop.rs              窗口、自启与公开支持信息
  services/
    optimizer.rs            模型消息构建与回复校验
    generation.rs           优化流程、取消和用量落库协调
    providers.rs            连接测试与模型目录请求
    pricing.rs              价格同步、汇率回退和目录缓存协调
    backup.rs               备份格式、文件读写及恢复流程
    update.rs               更新状态、来源匹配与安装前互斥
  models/
    workspace.rs            工作区实体、设置和增量保存结构
    provider.rs             不含 API Key 的供应商结构
    conversation.rs         会话类型、优化输入输出和会话校验
    usage.rs                接口报告的用量与价格输入结构
    update.rs               更新来源和可用版本结构
    backup.rs               完整备份结构和范围常量
    catalog.rs              远程模型目录的纯筛选与转换
    validation.rs           草稿消息与快捷键校验
    pricing.rs              十进制计费和报价转换规则
  database/
    connection.rs           打开数据库、初始化当前结构与计价函数
    schema.sql              当前表结构
    workspace.rs            工作区查询、增量写入及事务内关系校验
    providers.rs            供应商写入与按 ID/接口地址绑定的密钥访问
    usage.rs                用量记录、统计和查询
    pricing.rs              价格持久化、SQL 计价函数接线和费用快照
    catalog.rs              远程目录缓存和同步状态读写
    backup.rs               完整表快照、事务恢复与数据库内容校验
    resources/              内置价格及汇率快照
    tests/                  数据库行为回归测试
  clients/
    model.rs                模型 HTTP、模型列表、流式响应解析
    models_dev.rs           models.dev 下载及响应大小限制
    exchange_rate.rs        汇率 HTTP 请求及响应校验
  desktop/
    window.rs               显示、隐藏和退出窗口
    tray.rs                 系统托盘与托盘事件
    autostart.rs            自启查询、修改和失败恢复
    updater.rs              官方 Updater 检查、验签下载和安装
    support.rs              公开环境信息和仓库链接打开
```

## 依赖与职责

主要调用方向为 `commands → services → database / clients / desktop`，各层共享 `models`。简单查询或单次写入可由命令直接调用 `database`，不用额外建立转发服务。

- `commands` 保留 Tauri 参数名、Channel、返回值及错误适配，不直接写 SQL 或发 HTTP 请求。
- `services` 负责多个步骤的协调和操作锁，不直接编写 SQL。生成流程通过回调输出增量文本，Tauri Channel 由命令层适配。
- `models` 不访问数据库、网络、文件或 Tauri；除数据结构外，可以维护纯校验与金额计算规则。
- `database` 不反向调用服务、HTTP 客户端或桌面 SDK。跨层的完整备份回归测试可组合多个模块。
- `clients` 的生产实现只依赖共享类型及 HTTP 能力；测试可以结合临时数据库验证流式用量保存。
- `desktop` 集中系统及 Tauri 插件操作。更新服务是桌面应用服务，会接收 AppHandle，并在状态中使用官方 Updater 类型，不为此建立通用接口或假设可直接移植到 Web。
- `lib.rs` 是组装入口，`state.rs` 持有共享资源，不把业务流程集中到状态对象中。

## 必须保持的行为

IPC 命令名、参数的 camelCase 对应关系和 JSON 返回结构保持稳定。`save_changes` 接收可选的 `providerKey`，使供应商配置与输入的密钥在同一事务中保存；密钥只作为本次请求参数，不加入工作区快照或持久化的前端状态。Rust 按供应商 ID 和接口地址匹配更新，任一步失败都回滚。`CommandResult` 的错误通过透明序列化继续返回字符串；它不自动脱敏，调用方仍须使用明确的用户提示，不能传入原始密钥或包含密钥的底层错误。

优化期间保持操作读锁，完整备份、恢复和安装更新必须取得独占锁。取消后清除请求登记，并记录本轮取消状态；模型输入、流式处理和最终回复校验继续相互分离。

SQLite 表结构保持原样，不引入迁移，不重建用户数据库。工作区增量写入和完整备份恢复保留原事务边界，拆文件不能把一项原子操作改成多个独立事务。

供应商普通结构不包含 API Key。密钥由数据库访问模块读取，仍按供应商 ID 与接口地址共同匹配，接口改变时清空旧密钥。金额使用 Decimal 和 TEXT，缺失用量保持未知，已经计价的请求保留费用快照。

完整备份中的 schema 文本使用 `database::SCHEMA`。移动 schema 文件时必须保持其内容与换行字节，避免让同版本备份因文本不一致而无法恢复。

## 检查

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run test:frontend
```

测试使用内存或独立临时数据库、回环地址上的模拟接口，不访问用户数据库或真实供应商。完整应用检查仍使用 `npm run check`，打安装包和发布需要用户另行提出。
