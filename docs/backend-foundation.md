# 后端基础底座设计说明

## 1. 文档目标

本文档用于约定低代码平台后端基础底座的工程结构、公共能力、接口响应、错误处理、数据库迁移、鉴权审计与可观测性扩展方式。后续后端代码实现必须优先遵守本文档，除非需求、容量目标或架构约束发生变化并重新评审。

后端基础底座的核心目标如下：

- **稳定性**：启动流程、配置加载、数据库迁移、错误处理和健康检查必须可预测，避免隐式副作用。
- **性能容量**：接口、查询、分页、连接池和批处理设计必须前置考虑数据规模、访问频率和最大返回规模。
- **数据一致性**：所有持久化实体必须具备统一基础字段，写入路径必须考虑事务、幂等、乐观锁和逻辑删除。
- **可扩展性**：工程结构按基础设施、通用能力和业务模块分层，避免业务代码直接依赖底层细节。
- **可观测性**：请求链路必须具备 `traceId`，错误、日志、审计和指标必须能关联到同一次请求。

## 2. 技术栈与基本约束

后端工程固定使用以下技术栈：

- Web 框架：Rust + Axum。
- 数据访问：SQLx。
- 数据库迁移：Refinery。
- 默认数据库：MySQL。
- 兼容数据库：PostgreSQL。
- 序列化：Serde。
- 日志与追踪：`tracing` 体系。
- 时间类型：统一使用 UTC 时间存储，对外输出 ISO 8601 字符串。

基础约束如下：

- 后端接口路径必须使用 `snake_case`，例如 `/api/v1/form_definitions`。
- JSON 请求体和响应体字段必须使用 `camelCase`，例如 `traceId`、`createdTime`。
- 所有业务接口必须使用统一业务码响应封装，不直接返回裸资源体。
- 所有业务 API 由后端应用正常构造出的标准响应，HTTP 状态码必须固定为 `200`，由响应体中的 `code` 字段区分业务状态。
- 健康检查接口属于部署探针接口，可以根据实例状态返回 HTTP `200` 或 `503`，但响应体仍保持统一结构。
- 所有列表和搜索接口必须支持分页，并设置服务端最大 `pageSize`。
- 所有数据库结构、默认数据和基础字典数据必须通过 Refinery migration 管理。
- 所有默认查询必须过滤逻辑删除数据，除非接口语义明确要求包含已删除数据。

## 3. 后端工程结构

后端采用 `backend/` 单工程结构。仓库后续可继续扩展 `frontend/` 和 `mobile/`，但后端基础能力全部收敛在 `backend/` 内。

```text
backend/
  Cargo.toml
  src/
    main.rs
    app.rs
    config/
      mod.rs
      settings.rs
    bootstrap/
      mod.rs
      database.rs
      migration.rs
      tracing.rs
    error/
      mod.rs
      app_error.rs
      error_code.rs
    response/
      mod.rs
      api_response.rs
      page.rs
    context/
      mod.rs
      request_context.rs
    middleware/
      mod.rs
      trace_id.rs
      auth.rs
      access_log.rs
    db/
      mod.rs
      executor.rs
      transaction.rs
    validation/
      mod.rs
      validate.rs
    utils/
      mod.rs
      id.rs
      time.rs
    health/
      mod.rs
      route.rs
      handler.rs
    modules/
      mod.rs
      example/
        mod.rs
        route.rs
        handler.rs
        service.rs
        repository.rs
        model.rs
        dto.rs
    tests/
      mod.rs
      fixture.rs
  migrations/
    mysql/
      V202604280001__init_foundation.sql
      V202604280002__fix_app_metadata_active_unique_index.sql
    postgres/
      V202604280001__init_foundation.sql
      V202604280002__fix_app_metadata_active_unique_index.sql
  tests/
    integration/
      health_check_test.rs
      api_response_test.rs
      migration_test.rs
```

目录职责如下：

| 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 只负责启动入口，按顺序完成配置加载、日志初始化、数据库连接、迁移执行、路由装配和服务监听。 |
| `src/app.rs` | 构建 Axum `Router`，集中挂载中间件、健康检查和业务模块路由。 |
| `src/config/` | 负责配置结构定义、环境变量读取、默认值和配置校验。业务模块不得直接读取环境变量。 |
| `src/bootstrap/` | 负责启动期基础设施初始化，包括数据库连接池、Refinery migration 和 tracing。 |
| `src/error/` | 定义统一错误码、错误类型、业务状态映射和错误响应转换。 |
| `src/response/` | 定义 `ApiResponse<T>`、`PageResult<T>`、响应构造函数和序列化规则。 |
| `src/context/` | 定义 `RequestContext`，保存追踪 ID、租户、用户、角色、客户端和追踪信息。 |
| `src/middleware/` | 定义追踪 ID、鉴权占位、访问日志、上下文注入等 Axum 中间件。 |
| `src/db/` | 封装数据库执行器、事务边界和通用查询约定，避免业务层直接扩散连接池细节。 |
| `src/validation/` | 封装参数校验工具，统一把校验失败转换为 `-400`。 |
| `src/utils/` | 放置无业务含义的通用工具，例如 ID 生成、时间转换和安全字符串处理。 |
| `src/health/` | 提供健康检查和就绪检查接口，供部署、监控和发布流程使用。 |
| `src/modules/` | 按业务领域拆分模块，每个模块内部保持 route、handler、service、repository、model、dto 分层。 |
| `migrations/` | 存放 Refinery migration，按数据库方言分目录维护同版本脚本。 |
| `tests/` | 存放跨模块集成测试，重点覆盖启动、响应封装、错误映射和数据库行为。 |

业务模块内部职责边界如下：

- `route.rs`：只声明 URL、HTTP 方法、中间件局部挂载和 handler 绑定。
- `handler.rs`：只负责请求参数提取、调用 service、返回 `ApiResponse<T>`，不得直接写 SQL。
- `service.rs`：承载业务规则、事务编排、权限检查入口和幂等控制。
- `repository.rs`：承载 SQLx 查询和数据持久化逻辑，必须默认过滤 `deleted = false`。
- `model.rs`：定义数据库实体模型，必须包含统一基础字段。
- `dto.rs`：定义请求和响应结构，字段命名必须与接口契约一致。

## 4. 启动流程

后端服务启动必须遵守固定顺序：

1. 加载配置并执行配置校验。
2. 初始化 tracing 和日志格式。
3. 创建数据库连接池。
4. 根据数据库类型执行对应目录下的 Refinery migration。
5. 构建应用共享状态 `AppState`。
6. 装配全局中间件和业务路由。
7. 启动 HTTP 服务监听。

启动失败必须立即退出进程，并输出结构化错误日志。禁止在 migration 失败、数据库不可用或关键配置缺失时继续启动服务。

## 5. 工具类与基础设施

后端基础底座必须提供以下公共能力。

| 能力 | 建议类型或模块 | 说明 |
| --- | --- | --- |
| 配置加载 | `AppConfig` | 统一读取环境变量、配置文件和默认值，并在启动期完成必要字段校验。 |
| 数据库连接池 | `DatabasePool` | 基于 SQLx 创建连接池，配置最大连接数、最小连接数、连接超时和空闲回收。 |
| 数据库迁移 | `run_migrations` | 启动期按数据库类型选择 `migrations/mysql` 或 `migrations/postgres` 并执行 Refinery migration。 |
| 统一响应 | `ApiResponse<T>` | 所有业务接口统一返回 `code`、`message`、`data`、`traceId`、`timestamp`。 |
| 分页模型 | `PageQuery`、`PageResult<T>` | 统一处理 `pageNo`、`pageSize`、总数、总页数和是否存在下一页。 |
| 统一错误 | `AppError`、`ErrorCode` | 统一封装业务错误、参数错误、权限错误、资源不存在、并发冲突和系统错误。 |
| 请求上下文 | `RequestContext` | 统一保存追踪 ID、用户、租户、角色、客户端 IP、User-Agent 和追踪字段。 |
| 参数校验 | `ValidateExt` | 对请求 DTO 执行显式校验，禁止依赖数据库错误表达业务校验失败。 |
| 时间工具 | `time` | 统一获取当前 UTC 时间和格式化输出，避免业务代码各自处理时区。 |
| ID 工具 | `id` | 统一生成业务 ID 或 `traceId`，避免不同模块使用不一致的 ID 策略。 |
| 日志追踪 | `tracing` | 所有请求日志、错误日志、审计日志必须携带 `traceId`。 |
| 健康检查 | `health` | 提供存活检查和就绪检查，就绪检查必须覆盖数据库连接。 |
| 测试辅助 | `tests::fixture` | 提供测试配置、测试数据库、请求构造和响应断言工具。 |

公共工具必须保持无业务领域含义。若工具函数只服务于某个业务模块，应放在对应业务模块内部，避免污染全局工具包。

## 6. 标准接口响应

所有业务接口必须返回统一响应结构：

```json
{
  "code": 200,
  "message": "ok",
  "data": {},
  "traceId": "trace-id",
  "timestamp": "2026-04-28T00:00:00Z"
}
```

字段说明如下：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `code` | `number` | 是 | 数字业务码。成功使用正数，例如 `200`；错误使用负数，例如 `-500`。 |
| `message` | `string` | 是 | 面向调用方的简短说明。系统内部错误不得泄露敏感细节。 |
| `data` | `object/null/array` | 是 | 业务数据。错误响应固定为 `null`，无返回值成功响应可为 `{}`。 |
| `traceId` | `string` | 是 | 请求链路唯一标识，由 `X-Trace-Id` 请求头透传或服务端生成。 |
| `timestamp` | `string` | 是 | 服务端响应时间，使用 UTC ISO 8601 格式。 |

只要业务 API 请求进入后端应用并由应用正常构造标准响应，HTTP 状态码必须固定为 `200`。前端、移动端和外部调用方必须通过响应体中的 `code` 判断业务成功、参数错误、权限错误、资源不存在、并发冲突或系统错误。健康检查接口是该规则的例外，必须按部署探针语义返回 HTTP `200` 或 `503`。

`traceId` 透传规则如下：

- 请求头和响应头固定使用 `X-Trace-Id`。
- 请求头缺失时，由服务端生成新的 `traceId`。
- 请求头存在时，必须校验长度和字符集；非法值必须丢弃并由服务端重新生成。
- 服务端必须把最终采用的 `traceId` 同时写入响应头 `X-Trace-Id` 和响应体 `traceId` 字段。

标准错误响应如下：

```json
{
  "code": -400,
  "message": "请求参数不合法",
  "data": null,
  "traceId": "trace-id",
  "timestamp": "2026-04-28T00:00:00Z"
}
```

分页接口必须把分页结果放入 `data`，不得在响应顶层额外增加分页字段：

```json
{
  "code": 200,
  "message": "ok",
  "data": {
    "records": [],
    "pageNo": 1,
    "pageSize": 20,
    "total": 0,
    "totalPages": 0,
    "hasNext": false
  },
  "traceId": "trace-id",
  "timestamp": "2026-04-28T00:00:00Z"
}
```

后续实现必须提供以下核心类型：

| 类型 | 职责 |
| --- | --- |
| `ApiResponse<T>` | 表示统一接口响应，负责成功和失败响应的序列化。 |
| `PageResult<T>` | 表示分页结果，包含 `records`、`pageNo`、`pageSize`、`total`、`totalPages`、`hasNext`。 |
| `PageQuery` | 表示分页查询参数，负责默认页码、默认分页大小和最大分页大小约束。 |
| `AppError` | 表示应用错误，负责携带数字业务码、日志级别和安全错误消息。 |
| `ErrorCode` | 表示稳定数字业务码，序列化为 `i32`，禁止随意修改已开放错误码语义。 |
| `RequestContext` | 表示请求上下文，供 handler、service、repository 和日志审计链路使用。 |

## 7. 分页与列表查询约定

列表接口必须统一接收以下分页参数：

| 字段 | 类型 | 默认值 | 约束 |
| --- | --- | --- | --- |
| `pageNo` | `u64` | `1` | 最小值为 `1`。 |
| `pageSize` | `u64` | `20` | 最小值为 `1`，最大值默认不超过 `100`。 |

列表查询必须遵守以下规则：

- 默认按稳定字段排序，禁止依赖数据库自然顺序。
- 默认过滤 `deleted = false`。
- 查询条件必须显式声明，不得把未知字段直接拼接到 SQL。
- 排序字段必须使用白名单映射，禁止直接信任前端传入的列名。
- 大结果集导出必须走异步任务，不得复用普通分页接口一次性返回全部数据。

## 8. 错误处理约定

错误处理必须区分错误来源和业务语义。`AppError` 需要同时表达数字业务码、用户可见消息、内部日志消息、日志级别和是否需要告警。

业务 API 由后端应用正常构造的错误响应也必须使用 HTTP `200`。只有业务请求未进入标准响应链路时，才可能由框架、网关或部署平台产生非 `200` HTTP 状态码；业务调用方不得依赖 HTTP 状态码判断业务结果。健康检查接口按部署探针语义单独处理。

基础错误码建议如下：

| 业务码 | 标准响应 HTTP 状态码 | 语义 | 典型场景 |
| --- | --- | --- | --- |
| `200` | `200` | 成功 | 请求处理成功。 |
| `-400` | `200` | 参数错误 | 必填字段缺失、格式错误、分页参数越界。 |
| `-401` | `200` | 未认证 | 缺少登录态、令牌无效或令牌过期。 |
| `-403` | `200` | 无权限 | 已认证但没有访问资源或操作的权限。 |
| `-404` | `200` | 资源不存在 | 查询、更新或删除的资源不存在，或已被逻辑删除。 |
| `-409` | `200` | 并发冲突 | 乐观锁版本不匹配、唯一业务键冲突、重复提交。 |
| `-422` | `200` | 业务规则失败 | 状态流转非法、业务前置条件不满足。 |
| `-429` | `200` | 请求过于频繁 | 高频接口触发限流策略。 |
| `-500` | `200` | 系统错误 | 未预期异常、依赖不可用、数据库执行失败。 |

错误处理规则如下：

- 参数错误必须在 handler 或 validation 层显式校验，禁止依赖数据库约束错误作为主要校验方式。
- 资源不存在必须返回 `-404`，不得用空对象伪装成功。
- 乐观锁更新失败必须返回 `-409`，并提示调用方刷新后重试。
- 系统错误响应不得暴露 SQL、堆栈、连接串、密钥或内部路径。
- 日志中必须记录内部错误细节，并带上 `traceId`、接口路径、HTTP 方法和调用方信息。
- `AppError` 不得在缺少 `RequestContext` 的兜底路径中自行生成响应 `traceId`；handler 必须显式携带当前请求上下文转换错误响应。

## 9. 数据库与迁移约定

数据库默认使用 MySQL，并保持 PostgreSQL 兼容能力。Refinery migration 必须按方言分目录维护：

```text
backend/migrations/
  mysql/
    V202604280001__init_foundation.sql
    V202604280002__fix_app_metadata_active_unique_index.sql
  postgres/
    V202604280001__init_foundation.sql
    V202604280002__fix_app_metadata_active_unique_index.sql
```

数据库适配策略如下：

- 首版运行目标默认为 MySQL，PostgreSQL 通过同版本 migration、连接池抽象和 repository 方言边界保持兼容能力。
- SQLx 编译配置必须同时启用 MySQL 与 PostgreSQL 能力，避免后续切换数据库时重构公共数据访问层。
- 数据库连接池必须根据 `DATABASE_TYPE` 选择 `MySqlPool` 或 `PgPool`，并通过统一数据库适配层暴露给业务模块。
- repository 层禁止散落方言判断；共享查询优先使用 `sqlx::query` 和参数绑定，确有方言差异时必须封装在 repository 内部或独立 dialect adapter 内。
- Refinery 必须根据 `DATABASE_TYPE` 选择 `migrations/mysql` 或 `migrations/postgres`。

迁移规则如下：

- MySQL 和 PostgreSQL 脚本版本号必须保持一致，文件名中的版本号和业务意图必须一致。
- 默认运行 MySQL migration；当配置指定 PostgreSQL 时，运行 PostgreSQL migration。
- 所有表创建、表结构修改、默认数据和基础字典数据必须通过 migration 完成。
- migration 文件一旦进入共享环境，不得修改历史内容，只能新增后续版本修正。
- 表名、字段名和索引名必须清晰稳定，避免使用数据库关键字。
- 重要查询路径必须在建表或改表时同步规划索引。
- 删除业务数据默认采用逻辑删除，不得直接物理删除，除非需求明确允许并说明风险。
- 逻辑删除表的唯一约束必须只约束未删除数据；MySQL 可使用生成列承载未删除业务键并建立唯一索引，PostgreSQL 可使用 `WHERE deleted = FALSE` 的 partial unique index。

所有持久化业务实体必须包含以下统一基础字段，并明确区分数据库列名、Rust 持久化模型字段和 JSON DTO 字段：

| 数据库列名 | Rust 模型字段 | JSON DTO 字段 | 语义 | 写入规则 |
| --- | --- | --- | --- | --- |
| `version` | `version` | `version` | 数据版本号，用于乐观锁、并发控制或变更检测。 | 新增时为 `1`，更新和逻辑删除时递增。 |
| `deleted` | `deleted` | `deleted` | 逻辑删除标记。 | 新增时为 `false`，逻辑删除时为 `true`。 |
| `created_by` | `created_by` | `createdBy` | 创建人标识。 | 新增时写入当前用户或系统账号。 |
| `created_time` | `created_time` | `createdTime` | 创建时间。 | 新增时写入当前 UTC 时间。 |
| `updated_by` | `updated_by` | `updatedBy` | 最后更新人标识。 | 新增和更新时写入当前用户或系统账号。 |
| `updated_time` | `updated_time` | `updatedTime` | 最后更新时间。 | 新增和更新时写入当前 UTC 时间。 |
| `deleted_by` | `deleted_by` | `deletedBy` | 删除人标识。 | 未删除时为空，逻辑删除时写入当前用户或系统账号。 |
| `deleted_time` | `deleted_time` | `deletedTime` | 删除时间。 | 未删除时为空，逻辑删除时写入当前 UTC 时间。 |

数据库列名必须使用 `snake_case`，Rust 持久化模型字段建议保持 `snake_case` 以匹配 SQLx 映射；对外 JSON DTO 必须使用 `camelCase`。后端模型不得直接把数据库列命名暴露为对外接口字段，必须通过 Serde 配置或 DTO 映射保持跨层语义一致。

## 10. 鉴权、权限、审计与请求上下文

首版基础底座只定义鉴权、权限和审计的边界，不展开具体业务权限模型。

`RequestContext` 必须至少预留以下字段：

| 字段 | 说明 |
| --- | --- |
| `traceId` | 请求链路唯一标识，贯穿响应、日志、审计、指标和错误追踪。 |
| `tenantId` | 租户标识，单租户部署时可为空或使用默认租户。 |
| `userId` | 当前用户标识，未认证接口可为空。 |
| `roles` | 当前用户角色集合，供权限模块扩展。 |
| `clientIp` | 客户端 IP，供审计、风控和限流使用。 |
| `userAgent` | 客户端 User-Agent，供审计和问题排查使用。 |

鉴权与权限规则如下：

- 鉴权中间件负责解析令牌、构造用户上下文和处理未认证错误。
- 权限检查入口放在 service 层，避免 handler 只按 URL 做粗粒度判断。
- 公开接口必须显式标记为匿名可访问，禁止默认匿名开放。
- 权限失败统一返回 `-403`，未登录或令牌无效统一返回 `-401`。

审计规则如下：

- 重要写操作必须记录操作者、操作对象、操作类型、`traceId`、客户端信息和操作结果。
- 审计日志不得记录明文密码、令牌、密钥或敏感个人信息。
- 审计能力首版可先定义接口和扩展点，具体存储表可在权限模型明确后补充 migration。

## 11. 可观测性与健康检查

可观测性必须覆盖日志、追踪、指标和健康检查。

日志规则如下：

- 每个请求必须生成或透传 `traceId`。
- 请求头和响应头固定使用 `X-Trace-Id`，响应体固定使用 `traceId`。
- `X-Trace-Id` 缺失时服务端生成新值，存在但长度或字符集非法时丢弃并重新生成。
- 访问日志必须记录 HTTP 方法、路径、响应业务码、耗时、客户端 IP 和 `traceId`。
- 错误日志必须记录错误码、内部错误原因、调用路径和 `traceId`。
- 高频接口应避免记录大请求体或大响应体，防止日志放大影响性能。

健康检查接口如下：

| 路径 | 说明 |
| --- | --- |
| `GET /health/live` | 存活检查，只验证进程可响应，不依赖数据库。 |
| `GET /health/ready` | 就绪检查，验证数据库连接池和必要依赖可用。 |

健康检查响应也使用统一响应结构，但 HTTP 状态码必须服务于部署探针语义：检查成功返回 HTTP `200`，检查失败返回 HTTP `503`。就绪检查失败时响应体必须返回负数业务码，并保留 `code`、`message`、`data`、`traceId`、`timestamp` 字段。数据库原始错误只能进入服务端日志，响应体只能返回稳定原因码。

## 12. 测试与验收要求

后端基础底座实现后必须至少覆盖以下测试场景：

- `ApiResponse<T>` 成功响应序列化字段完整，字段名为 `camelCase`。
- 错误响应 `data` 固定为 `null`，并携带稳定 `code`、`message`、`traceId` 和 `timestamp`。
- 业务 API 错误响应 HTTP 状态码固定为 `200`，响应体 `code` 必须小于 `0`。
- `AppError` 能正确映射到数字业务码，且业务 API 标准响应 HTTP 状态码固定为 `200`。
- `X-Trace-Id` 缺失、合法、非法三种场景都能得到稳定响应头和响应体 `traceId`。
- `PageQuery` 能处理默认值、最小值、最大 `pageSize` 和非法参数。
- `PageQuery` 对超大 `pageNo` 计算 offset 时必须返回参数错误，不得 panic 或整数回绕。
- 列表查询默认过滤逻辑删除数据。
- 更新和逻辑删除能正确维护 `version`、`updatedBy`、`updatedTime`、`deletedBy`、`deletedTime`。
- 服务启动时能按配置执行对应数据库方言的 Refinery migration。
- MySQL 与 PostgreSQL migration 文件版本号必须保持一致。
- `/health/live` 在进程可响应时返回成功。
- `/health/ready` 在数据库不可用时返回 HTTP `503`，响应体仍包含统一结构和负数业务码。
- `/health/ready` 在数据库不可用时不得在响应体暴露原始数据库错误。
- 请求日志、错误日志和响应体能关联同一个 `traceId`。

## 13. 后续实现顺序建议

建议按以下顺序落地后端基础底座：

1. 创建 `backend/` Rust 工程和基础依赖。
2. 实现配置加载、日志初始化和启动流程。
3. 实现 SQLx 数据库连接池和 Refinery migration。
4. 实现 `ApiResponse<T>`、`PageResult<T>`、`PageQuery`、`ErrorCode`、`AppError`。
5. 实现追踪 ID 中间件、`RequestContext` 和访问日志。
6. 实现健康检查接口。
7. 补齐基础集成测试。
8. 在第一个真实业务模块中验证 route、handler、service、repository、model、dto 分层约定。

以上顺序可以降低基础能力之间的耦合风险，并尽早通过健康检查、响应封装和 migration 验证服务可运行性。
