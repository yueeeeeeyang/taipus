# 多租户体系设计方案

## 1. 文档目标

本文档用于约定低代码平台多租户能力的总体设计、数据模型、请求解析、数据隔离、接口约束、权限边界、运维治理和测试验收要求。后续实现租户模块、认证模块、业务模块或后台任务时，必须以本文档作为租户边界的统一依据。

多租户体系的目标如下：

- **数据隔离稳定**：不同租户的业务数据默认不可互相访问，所有业务读写必须携带明确租户上下文。
- **接入方式统一**：HTTP 请求、内部 Rust Service、后台任务、导入导出和异步任务必须使用同一套租户解析和传递规则。
- **演进路径清晰**：首版采用共享库共享表的 `tenant_id` 隔离模式，后续可平滑演进到分 schema 或独立数据库。
- **越权风险可控**：普通前端请求不得通过 body、query 或普通 header 任意指定租户，租户来源必须受认证或可信网关约束。
- **实现保持显式**：业务 repository 继续显式编写 SQL 和 `tenant_id` 条件，不引入通用 CRUD、通用 Repository 或隐藏租户过滤的动态框架。

## 2. 总体结论

首版推荐采用 **共享库共享表 + `tenant_id` 字段隔离**。

该模式适合当前平台阶段：

- 部署和运维复杂度低，不需要为每个租户创建独立数据库或 schema。
- SQLx、Refinery 和现有模块分层可以直接复用。
- HRM、i18n 等业务表已经开始显式保留租户字段或请求上下文扩展点。
- 后续如果出现高隔离客户，可以基于租户配置逐步演进为分 schema 或独立数据库。

首版不建议直接实现分库分表或租户级独立数据库。原因是当前认证、权限、审计、计费和运维体系尚未完整落地，过早引入多数据源会显著增加事务、迁移、连接池、监控和故障恢复复杂度。

## 3. 术语定义

| 术语 | 说明 |
| --- | --- |
| 租户 | 平台内的数据隔离单元，通常对应一个客户、企业或组织。 |
| 默认租户 | 单租户部署或开发环境使用的兜底租户，建议编码为 `default`，不可作为生产越权兜底。 |
| 租户上下文 | 当前请求或任务所绑定的租户信息，后端统一写入 `RequestContext.tenant_id`。 |
| 共享表隔离 | 多个租户共用同一张业务表，通过 `tenant_id` 字段隔离数据。 |
| 可信租户来源 | 认证 token、可信网关、内部任务配置等后端可验证来源。 |
| 非可信租户来源 | 普通前端传入的 body、query 或未鉴权 header，不得直接决定最终租户。 |

## 4. 租户边界

### 4.1 首版包含能力

首版多租户设计包含以下能力：

- 定义租户主数据表和租户状态。
- 在 `RequestContext` 中统一保存当前租户 ID。
- HTTP 请求通过认证 token 或受控 header 解析租户。
- 业务表通过 `tenant_id` 做共享表隔离。
- 业务 repository 显式携带 `tenant_id` 查询和写入。
- 租户内业务编码唯一，逻辑删除后允许复用。
- 内部 Rust Service 必须接收 `RequestContext` 或显式 `TenantContext`。
- 测试覆盖租户字段、查询过滤、唯一约束和越权场景。

### 4.2 首版暂不包含能力

首版暂不实现以下能力，但设计应保留扩展空间：

- 每租户独立数据库。
- 每租户独立 schema。
- 租户套餐计费、用量统计和账单。
- 租户级自定义域名管理后台。
- 租户级数据加密密钥轮换。
- 租户间数据迁移、拆分和合并自动化工具。
- 跨租户集团视图和多租户管理员工作台。

## 5. 租户数据模型

### 5.1 租户表 `sys_tenants`

后续应新增系统租户表 `sys_tenants`。该表属于平台系统主数据，必须通过 Refinery migration 创建。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 租户主键，写入业务表 `tenant_id`。 |
| `tenant_code` | `VARCHAR(64)` | 是 | 租户编码，全局未删除数据唯一，用于后台识别和运维。 |
| `name` | `VARCHAR(128)` | 是 | 租户名称。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 租户名称全拼，用于搜索。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 租户名称简拼，用于搜索。 |
| `status` | `VARCHAR(32)` | 是 | 租户状态。 |
| `isolation_mode` | `VARCHAR(32)` | 是 | 隔离模式，首版固定为 `shared_schema`。 |
| `primary_domain` | `VARCHAR(255)` | 否 | 租户主域名或子域名。 |
| `remark` | `VARCHAR(512)` | 否 | 备注。 |
| 基础字段 | - | 是 | 统一基础字段。 |

租户状态建议：

| 状态 | 说明 |
| --- | --- |
| `enabled` | 正常可用。 |
| `disabled` | 管理员禁用，拒绝普通业务请求。 |
| `suspended` | 欠费、风控或运维原因暂停，拒绝普通业务请求。 |

隔离模式建议：

| 隔离模式 | 说明 |
| --- | --- |
| `shared_schema` | 共享库共享表，通过 `tenant_id` 隔离。首版唯一支持。 |
| `schema_per_tenant` | 共享数据库、每租户独立 schema。后续预留。 |
| `database_per_tenant` | 每租户独立数据库。后续预留。 |

### 5.2 租户域名表 `sys_tenant_domains`

如果后续需要通过域名解析租户，应新增租户域名表。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `domain` | `VARCHAR(255)` | 是 | 域名或子域名。 |
| `verified` | `BOOLEAN` | 是 | 是否已完成域名验证。 |
| `status` | `VARCHAR(32)` | 是 | 域名状态。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束：

- `domain` 在未删除数据中全局唯一。
- 默认查询过滤 `deleted = false`。
- 未验证域名不得作为生产租户解析依据。

### 5.3 租户成员关系

租户与用户的关系应由认证或用户中心模块负责，不建议放在 HRM 内。

建议后续新增账号体系时设计：

```text
sys_accounts
sys_account_tenants
```

其中 `sys_account_tenants` 表达登录账号可访问哪些租户、默认租户和租户内身份。HRM 用户可以作为业务人员主数据被账号绑定，但 HRM 用户不应直接承担登录账号职责。

## 6. 业务表租户字段规范

所有租户业务表必须显式包含：

```text
tenant_id VARCHAR(64) NOT NULL
```

适用对象：

- HRM 用户、组织、岗位、任职关系。
- 表单、流程、页面、菜单、数据模型等后续低代码业务表。
- 业务翻译、附件、审计日志等需要租户隔离的数据。

不适用对象：

- 全局系统配置。
- 平台级租户表本身。
- 全局字典、系统资源、多语言系统文案。
- 健康检查、迁移历史等基础设施表。

命名要求：

- 数据库列名固定为 `tenant_id`。
- Rust 字段固定为 `tenant_id`。
- JSON 输出字段为 `tenantId`。
- 普通写入请求体默认不允许传 `tenantId`，由后端上下文写入。

## 7. 租户解析链路

### 7.1 推荐解析优先级

请求进入后，由统一租户解析中间件写入 `RequestContext.tenant_id`。

推荐优先级如下：

```text
认证 token 中的 tenant_id
> 认证 token 的默认租户
> 可信网关注入的 X-Tenant-Id
> 域名或子域名映射
> 开发/单租户环境默认租户
```

注意：

- 生产环境不得无条件信任普通客户端传入的 `X-Tenant-Id`。
- body 和 query 中的 `tenantId` 不得参与普通业务请求的租户解析。
- 如果 token 已绑定租户，请求 header 传入不同租户时必须拒绝，返回 `-403`。
- 如果无法解析租户，生产环境应返回 `-400` 或 `-401`，不应静默落到默认租户。

### 7.2 开发和单租户环境

开发环境和单租户部署可使用默认租户，建议配置：

```text
TENANT_DEFAULT_ID=default
TENANT_ALLOW_HEADER_OVERRIDE=true
```

生产环境建议：

```text
TENANT_ALLOW_HEADER_OVERRIDE=false
TENANT_REQUIRE_EXPLICIT=true
```

其中：

- `TENANT_DEFAULT_ID` 只服务开发、测试和单租户部署。
- `TENANT_ALLOW_HEADER_OVERRIDE` 只允许可信内部调用使用。
- `TENANT_REQUIRE_EXPLICIT` 开启后，无法解析租户的业务请求必须失败。

### 7.3 RequestContext 字段

`RequestContext` 至少应包含：

| 字段 | 说明 |
| --- | --- |
| `tenant_id` | 当前请求最终生效租户 ID。 |
| `tenant_source` | 租户来源，例如 `token`、`header`、`domain`、`default`。 |
| `user_id` | 当前登录用户或账号 ID。 |
| `trace_id` | 请求链路 ID。 |
| `locale` | 当前语言。 |
| `time_zone` | 当前时区。 |

首版可以先只落地 `tenant_id`，但中间件和日志结构应预留 `tenant_source`，方便排查越权和解析问题。

## 8. 接口契约

### 8.1 普通业务接口

普通业务接口不得在请求体中接受 `tenantId` 作为最终写入值。

示例：

```json
{
  "employeeNo": "E001",
  "name": "张三",
  "sortNo": 1,
  "status": "enabled"
}
```

后端写入时从 `RequestContext.tenant_id` 取租户。

如果前端传入以下字段：

```json
{
  "tenantId": "other"
}
```

处理建议：

- 首版 DTO 未声明该字段时可忽略。
- 后续如果启用严格 JSON 字段校验，应返回 `-400`。
- 不得用该字段覆盖 `RequestContext.tenant_id`。

### 8.2 租户管理接口

租户管理接口属于平台管理能力，应与普通业务接口区分。

建议路径：

```text
/api/v1/system/tenants
```

能力：

- 新增租户。
- 修改租户。
- 启用、禁用、暂停租户。
- 根据 ID 查询租户。
- 分页查询租户。
- 逻辑删除租户。

权限：

- 仅平台管理员可调用。
- 租户管理员不得管理其他租户主数据。
- 租户删除必须先完成引用检查和数据归档策略确认。

### 8.3 内部服务接口

内部 Rust Service 必须显式接收租户上下文。

推荐签名：

```rust
pub async fn get_user(pool: &DatabasePool, ctx: &RequestContext, id: &str) -> AppResult<HrmUser>
```

或：

```rust
pub async fn get_user(pool: &DatabasePool, tenant_id: &str, id: &str) -> AppResult<HrmUser>
```

要求：

- 跨模块调用不得省略租户。
- 批量查询必须限制最大 ID 数量。
- 内部服务不得提供无租户全局查询，除非明确是平台管理能力。

## 9. Repository 与 SQL 约束

业务 repository 必须显式携带租户条件。

查询示例：

```sql
SELECT *
FROM hrm_users
WHERE tenant_id = ?
  AND id = ?
  AND deleted = FALSE
```

更新示例：

```sql
UPDATE hrm_users
SET name = ?, updated_by = ?, updated_time = ?, version = version + 1
WHERE tenant_id = ?
  AND id = ?
  AND version = ?
  AND deleted = FALSE
```

分页查询要求：

- `tenant_id` 必须是基础条件，不允许由前端可选传入决定。
- 其他条件使用白名单动态拼接，只追加前端实际传入的字段。
- 默认过滤 `deleted = false`。
- 必须设置分页上限，当前统一遵守 `pageSize <= 100`。

禁止事项：

- 禁止无租户条件查询业务表。
- 禁止通过前端 body/query 的 `tenantId` 直接拼接租户条件。
- 禁止为租户过滤引入通用动态 CRUD 框架。
- 禁止把 `tenant_id` 过滤隐藏在难以审查的通用 repository 中。

## 10. 唯一约束与索引

租户级业务编码必须按租户隔离。

示例：

```text
tenant_id + employee_no + deleted = false
tenant_id + org_code + deleted = false
tenant_id + post_code + deleted = false
```

MySQL 可以使用生成列实现未删除唯一约束：

```sql
active_employee_key VARCHAR(160)
    GENERATED ALWAYS AS (
        CASE
            WHEN deleted = FALSE THEN CONCAT(tenant_id, '#', employee_no)
            ELSE NULL
        END
    ) STORED
```

PostgreSQL 可以使用 partial unique index：

```sql
CREATE UNIQUE INDEX uk_hrm_users_active_employee
    ON hrm_users (tenant_id, employee_no)
    WHERE deleted = FALSE;
```

常见索引建议：

| 场景 | 索引建议 |
| --- | --- |
| ID 查询 | `tenant_id + id + deleted`，主键为全局 ID 时仍建议 SQL 显式带 `tenant_id`。 |
| 状态候选 | `tenant_id + deleted + status`。 |
| 列表排序 | `tenant_id + deleted + sort_no`。 |
| 更新时间排序 | `tenant_id + deleted + updated_time`。 |
| 树结构 | `tenant_id + parent_id + deleted + sort_no`。 |
| 关系查询 | `tenant_id + user_id + deleted`、`tenant_id + org_id + deleted`。 |

## 11. 认证与权限边界

租户体系必须与认证模块协同。

认证 token 中建议包含：

| 字段 | 说明 |
| --- | --- |
| `account_id` | 登录账号 ID。 |
| `tenant_id` | 当前登录租户。 |
| `tenant_ids` | 可选，账号可访问租户列表。 |
| `roles` | 当前租户下角色。 |
| `exp` | 过期时间。 |

权限规则：

- 登录用户只能访问 token 当前租户内数据。
- 切换租户必须重新签发 token 或通过受控接口刷新上下文。
- 平台管理员访问租户管理接口时，应使用平台管理权限，不应混用普通租户业务权限。
- 租户管理员只能管理本租户内业务数据。
- 跨租户运维接口必须单独设计审计和二次确认。

## 12. 审计与日志

所有租户业务写操作应记录：

| 字段 | 说明 |
| --- | --- |
| `trace_id` | 请求链路 ID。 |
| `tenant_id` | 当前租户。 |
| `operator_id` | 操作者。 |
| `resource_type` | 资源类型。 |
| `resource_id` | 资源 ID。 |
| `operation` | 操作类型。 |
| `result` | 成功或失败。 |
| `error_code` | 失败时的业务错误码。 |
| `client_ip` | 客户端 IP。 |
| `user_agent` | User-Agent。 |

访问日志也应输出 `tenant_id` 和租户来源，方便排查：

```text
traceId=... tenantId=... tenantSource=token userId=...
```

注意：

- 系统错误日志不得泄露数据库连接串、密钥或敏感 token。
- 跨租户管理操作必须有审计记录。
- 后台任务必须显式记录任务租户，不能只记录任务 ID。

## 13. 后台任务与异步处理

后台任务必须显式保存租户上下文。

任务表建议包含：

```text
tenant_id
created_by
trace_id
task_type
status
payload
```

任务执行规则：

- 创建任务时从当前 `RequestContext.tenant_id` 写入任务。
- 执行任务时从任务记录恢复租户上下文。
- 任务内调用业务 Service 必须传入租户上下文。
- 禁止后台任务使用进程级默认租户处理多租户业务数据。

适用场景：

- 导入导出。
- 批量更新。
- 流程回调。
- 定时统计。
- 异步通知。

## 14. 缓存与消息

缓存 key 必须包含租户维度。

推荐格式：

```text
tenant:{tenant_id}:hrm:user:{user_id}
tenant:{tenant_id}:dict:{dict_type}
```

消息事件必须包含租户字段：

```json
{
  "tenantId": "t_001",
  "eventType": "hrm.user.updated",
  "resourceId": "u_001",
  "traceId": "..."
}
```

要求：

- 消费者必须校验事件租户。
- 不同租户的缓存不得共用 key。
- 租户删除、禁用或迁移时必须有缓存清理方案。

## 15. 数据迁移与默认租户

### 15.1 新系统初始化

新系统初始化时应通过 migration 创建默认租户：

```text
id = default
tenant_code = default
name = 默认租户
status = enabled
isolation_mode = shared_schema
```

是否创建默认租户应由部署模式决定：

- 开发和单租户部署：创建默认租户。
- 多租户生产部署：可以创建平台初始化租户，但普通业务请求不得依赖默认租户兜底。

### 15.2 旧数据迁移

如果已有业务表缺少 `tenant_id`，迁移步骤应为：

1. 创建默认租户。
2. 给业务表新增 `tenant_id` 字段，初始允许空。
3. 批量回填默认租户 ID。
4. 将 `tenant_id` 改为 `NOT NULL`。
5. 重建租户内唯一约束和查询索引。
6. 修改 repository SQL，强制携带租户条件。
7. 增加契约测试，防止后续 SQL 遗漏租户过滤。

## 16. 安全约束

必须遵守以下安全规则：

- 普通业务请求不得信任 body/query 中的 `tenantId`。
- 生产环境不得无鉴权使用 `X-Tenant-Id`。
- token 租户和 header 租户冲突时必须拒绝。
- 禁用或暂停租户不得访问普通业务接口。
- 物理删除租户前必须完成数据归档、引用检查和二次确认。
- 平台管理员跨租户操作必须记录审计日志。
- 导出接口必须按租户过滤，避免全库导出。
- 错误响应不得泄露其他租户是否存在某条数据。

## 17. 错误码约定

| 错误码 | 场景 |
| --- | --- |
| `-400` | 租户参数非法、租户上下文缺失、租户解析失败。 |
| `-401` | 未认证，无法确认租户身份。 |
| `-403` | 当前用户无权访问该租户或发生跨租户访问。 |
| `-404` | 租户不存在、已删除，或租户内资源不存在。 |
| `-409` | 租户编码冲突、并发修改冲突。 |
| `-422` | 租户状态不允许执行当前操作，例如已暂停租户访问业务接口。 |
| `-500` | 租户解析依赖、数据库或系统异常。 |

建议错误消息：

- 租户上下文缺失：`租户上下文缺失`
- 无权访问租户：`无权访问当前租户`
- 租户已禁用：`租户已禁用，不能访问业务接口`
- 租户已暂停：`租户已暂停，不能访问业务接口`

## 18. 实施计划

### 阶段一：租户上下文与文档约束

- 完成本文档。
- 明确业务接口不接受 body/query 租户字段。
- 保持 HRM 当前 `tenant_id` 设计。
- 在 code review 中检查新增业务 SQL 是否携带租户条件。

### 阶段二：租户主数据

- 新增 `sys_tenants` migration。
- 实现租户 model、dto、repository、service、handler 和 route。
- 创建默认租户初始化数据。
- 实现租户分页、详情、启停和逻辑删除。

### 阶段三：租户解析中间件

- 增加租户配置项。
- 实现租户解析中间件。
- 将最终租户写入 `RequestContext.tenant_id`。
- 访问日志输出 `tenantId` 和 `tenantSource`。
- 生产环境禁止不可信 header 覆盖租户。

### 阶段四：认证与权限接入

- 认证 token 写入当前租户。
- 实现租户切换接口。
- 校验账号可访问租户列表。
- 平台管理员和租户管理员权限分离。

### 阶段五：治理能力

- 审计日志接入租户字段。
- 后台任务保存租户上下文。
- 缓存和消息 key 增加租户维度。
- 增加租户容量、限流和用量统计。

## 19. 测试计划

### 19.1 Migration 契约测试

- `sys_tenants` 表包含统一基础字段。
- 租户名称包含全拼和简拼字段。
- `tenant_code` 未删除数据全局唯一。
- 默认租户初始化数据符合约定。
- 业务表新增时必须包含 `tenant_id`，不适用对象需在测试白名单中说明。

### 19.2 中间件测试

- token 租户优先于 header。
- header 租户只在允许配置下生效。
- 域名解析能正确写入 `RequestContext.tenant_id`。
- 生产配置下缺少租户返回错误，不静默使用默认租户。
- token 租户和 header 租户冲突返回 `-403`。

### 19.3 Repository 测试

- 查询必须包含 `tenant_id` 条件。
- 更新必须包含 `tenant_id + id + version + deleted = false` 条件。
- 同一业务编码允许不同租户重复。
- 同一租户未删除业务编码不允许重复。
- 逻辑删除后允许同租户复用业务编码。

### 19.4 Service 测试

- 禁用租户不能访问普通业务接口。
- 内部 Service 缺少租户上下文返回 `-400`。
- 跨租户 ID 查询返回不存在或无权限，不泄露其他租户数据。
- 后台任务能恢复创建时租户上下文。

### 19.5 API 测试

- 普通业务请求体传入 `tenantId` 不会覆盖上下文租户。
- 非法租户 header 返回统一 `ApiResponse`。
- 当前租户无权限访问时返回 `-403`。
- 租户管理接口仅平台管理员可访问。
- JSON 字段保持 `camelCase`，接口路径保持 `snake_case`。

## 20. 与现有模块的关系

### 20.1 HRM 模块

HRM 已按租户内主数据设计：

- 表包含 `tenant_id`。
- repository 显式过滤 `tenant_id`。
- 工号、组织编码、岗位编码按租户内未删除数据唯一。
- service 从 `RequestContext.tenant_id` 读取租户，空值暂时使用默认租户。

后续租户中间件落地后，HRM 应移除模块内随意兜底默认租户的行为，改为由统一租户解析层决定是否允许默认租户。

### 20.2 i18n 模块

系统多语言资源属于全局能力，不需要租户隔离。

业务翻译是否需要租户隔离取决于资源类型：

- 系统资源翻译：全局。
- 租户业务数据翻译：必须包含 `tenant_id`。
- 平台管理资源翻译：由平台权限控制。

### 20.3 认证和权限模块

认证模块负责确认账号身份和当前租户。

权限模块负责判断当前账号在当前租户下是否具备操作权限。

HRM 不应直接实现登录账号、密码和角色授权。

## 21. 待确认问题

以下问题会影响后续实现，需要在租户模块落地前确认：

- 默认租户 ID 是否固定为 `default`，还是由配置强制指定。
- 生产环境是否允许通过可信网关 header 传入租户。
- 租户管理员是否允许修改租户名称、域名和状态。
- 是否需要租户级套餐、容量、用户数和存储限制。
- 是否需要支持用户同时加入多个租户并在线切换。
- 是否需要支持租户自定义域名。
- 是否有高隔离客户要求独立数据库或独立 schema。

