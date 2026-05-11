# 权限模块设计方案

## 1. 文档目标

本文档用于约定低代码平台权限模块的总体设计、资源模型、角色模型、统一授权模型、鉴权链路、缓存策略、接口契约、数据一致性和测试验收要求。后续实现权限模块、菜单渲染、按钮控制、接口拦截、低代码资源接入和跨端权限消费时，必须优先遵守本文档，除非安全边界、容量目标或业务授权模型发生变化并重新评审。

权限模块的核心目标如下：

- **统一授权**：应用、菜单、按钮和接口资源全部抽象为统一资源，角色授权和资源授权都落到同一套授权规则，不为不同资源类型拆出多套权限表和多套鉴权逻辑。
- **高性能鉴权**：普通接口请求不得实时递归角色树或扫描授权明细，必须基于预计算角色闭包、有效权限快照或缓存完成快速判断。
- **边界清晰**：认证模块只证明当前账号是谁、属于哪个租户和当前会话是否有效；权限模块负责判断该账号能访问哪些资源和执行哪些动作。
- **继承可控**：角色支持父子角色，子角色继承父角色全部有效权限；继承关系必须防止循环，并能明确区分直接授权与继承授权。
- **跨端一致**：Web 前端、移动端和后端接口鉴权必须使用同一份资源编码、动作编码和授权结果，避免出现前端可见但后端拒绝，或后端放行但前端隐藏的权限漂移。
- **可观测可恢复**：授权变更、鉴权拒绝、缓存刷新和权限快照生成必须可审计、可追踪、可诊断，并能在缓存失效时回退到数据库权威数据。

## 2. 总体结论

首版推荐采用 **独立资源表 + 统一资源引用 + 统一授权规则 + 角色闭包表 + 有效权限缓存**。

核心设计如下：

- 应用、菜单、按钮和接口资源分别保存在 `sys_permission_applications`、`sys_permission_menus`、`sys_permission_buttons`、`sys_permission_apis`，避免不同资源类型字段差异过大导致单表稀疏和约束不清。
- 权限动作统一使用 `action` 表达，例如 `view`、`manage`、`click`、`call`，不同资源类型只能使用白名单动作。
- 角色统一保存在 `sys_roles`，角色继承关系保存在 `sys_role_relations`，角色祖先闭包保存在 `sys_role_closures`。
- 授权规则统一保存在 `sys_permission_grants`，通过 `subject_type + subject_id + resource_type + resource_id + action + effect` 表达“谁对哪类资源的哪个对象有什么动作权限”。
- 首版必须支持 `subject_type = role` 的角色授权；表结构预留 `account`、`org`、`post`、`tenant` 等主体类型，后续可扩展账号直授权、组织授权和岗位授权，但首版普通管理端只开放角色授权。
- 接口请求鉴权使用 `method + normalized_path` 命中 API 资源，再判断当前账号有效角色集合是否拥有 `call` 权限。
- 菜单和按钮权限由后端按当前账号、租户、平台和 locale 生成可见资源树，前端和移动端只消费结果，不自行推导角色继承。
- 角色继承必须通过闭包表和事务维护，读取有效权限时直接 join 闭包，不在请求链路递归遍历父角色。
- 高频鉴权必须支持缓存，缓存 key 至少包含 `tenant_id`、`account_id`、`resource_type`、`resource_id`、`action`、`permission_version`。

不建议首版直接引入复杂策略引擎、ABAC 表达式语言或动态脚本授权。原因是当前需求重点是低代码平台内置资源、角色继承和跨端一致性；过早引入策略语言会显著增加调试成本、缓存失效复杂度和安全审计难度。后续如需要字段权限、数据权限或条件权限，可在统一授权规则上增加 `condition_type` 和受控策略扩展点，而不是替换整套模型。

## 3. 术语定义

| 术语 | 说明 |
| --- | --- |
| 权限主体 | 被授予权限的一方，首版主要是角色，后续可扩展账号、组织、岗位或系统租户。 |
| 权限资源 | 被访问或操作的对象，统一包括应用、菜单、按钮和接口资源。 |
| 权限动作 | 主体对资源执行的动作，例如查看菜单、点击按钮、调用接口或管理应用。 |
| 授权规则 | 一条主体、资源、动作和效果的关系记录，是权限判断的最小持久化单元。 |
| 角色 | 权限主体的一种，用于把一组权限授予一批账号。 |
| 父角色 | 被继承权限的角色。 |
| 子角色 | 继承父角色全部有效权限，并可追加自身直接权限的角色。 |
| 角色闭包 | 保存角色祖先、后代和距离的预计算关系，用于快速查询继承链。 |
| 有效权限 | 一个账号在当前租户下通过直接角色、父角色继承和授权规则合并后的最终权限集合。 |
| 权限版本 | 租户内权限数据变更版本，用于缓存失效和客户端资源刷新。 |

## 4. 模块边界

### 4.1 首版包含能力

首版权限模块包含以下能力：

- 应用资源管理。
- 菜单资源管理，支持应用下菜单树。
- 按钮资源管理，支持绑定菜单或页面资源。
- 接口资源管理，支持 HTTP method、路径模板、公开接口标记和鉴权动作。
- 角色管理，支持租户内角色编码唯一、启用禁用、排序和备注。
- 角色父子关系管理，支持多父角色，子角色继承所有父角色有效权限。
- 角色授权，支持把资源动作授权给角色。
- 统一权限查询，支持查询角色直接权限、继承权限和账号有效权限。
- 当前账号菜单树、按钮权限和接口权限判断。
- 权限变更版本维护和缓存失效事件。
- 权限审计日志，覆盖资源变更、角色变更、继承关系变更、授权变更和接口拒绝。

### 4.2 首版暂不包含能力

首版暂不实现以下能力，但模型必须预留扩展空间：

- 字段级权限。
- 行级数据权限。
- 动态 ABAC 条件表达式。
- 临时授权和按时间段授权。
- 用户组、组织、岗位的管理端授权入口。
- 跨租户平台管理员的完整授权矩阵。
- 外部身份源角色映射。
- 审批流驱动的授权申请和回收。

## 5. 设计原则

权限模块必须遵守以下原则：

- 默认拒绝：无法识别资源、账号未认证、角色无效、租户无效或缓存异常时，受保护接口默认拒绝。
- 后端兜底：前端菜单和按钮控制只用于体验优化，后端接口必须独立完成权限校验。
- 显式资源：所有受保护接口必须注册为 API 资源，禁止只依赖 URL 前缀或 handler 名称隐式授权。
- 显式 SQL：repository 必须显式编写授权、继承和权限查询 SQL，不引入通用 CRUD 或动态 Repository 隐藏关键约束。
- 租户隔离：租户内资源、角色、授权和账号角色关系默认通过 `tenant_id` 隔离；平台内置资源可通过 `scope` 标记为全局资源。
- 稳定编码：资源编码、角色编码和动作编码一旦对前端或低代码配置开放，修改前必须评估兼容性。
- 最小授权：管理端默认只授予必要资源动作，不通过超级角色绕过权限模型，除非明确记录平台级运维场景。

## 6. 统一权限模型

### 6.1 权限三元组

权限判断统一抽象为：

```text
subject -> resource + action -> effect
```

字段语义如下：

| 维度 | 示例 | 说明 |
| --- | --- | --- |
| `subject_type` | `role` | 授权主体类型，首版开放角色授权。 |
| `subject_id` | `role_admin` | 授权主体 ID。 |
| `resource_type` | `menu` | 资源类型，用于路由到应用、菜单、按钮或接口资源表。 |
| `resource_id` | `menu_hrm_user` | 被授权资源 ID。 |
| `action` | `view` | 对资源执行的动作。 |
| `effect` | `allow` / `deny` | 授权效果，首版管理端默认只使用 `allow`，保留 `deny` 兜底冲突策略。 |

角色授权与资源授权都使用同一张授权规则表：

- 角色授权：`subject_type = role`，表示某角色拥有某资源动作。
- 资源授权：以资源为中心维护哪些主体拥有该资源动作，写入时仍然落到同一张 `sys_permission_grants`。

两者只是管理视角不同：角色授权页面从角色出发批量勾选资源，资源授权页面从资源出发批量选择角色，底层存储和鉴权逻辑必须完全一致。

### 6.2 动作规范

首版动作建议如下：

| 资源类型 | 推荐动作 | 说明 |
| --- | --- | --- |
| `application` | `access`、`manage` | 访问应用、管理应用配置。 |
| `menu` | `view`、`manage` | 查看菜单、管理菜单配置。 |
| `button` | `click`、`manage` | 使用按钮动作、管理按钮配置。 |
| `api` | `call`、`manage` | 调用接口、管理接口资源。 |

约束如下：

- 资源类型和动作必须通过后端白名单校验。
- 按钮资源通常需要与 API 资源配套授权，但不能只依赖按钮权限放行接口。
- `manage` 是资源管理动作，不等同于该资源下所有业务动作；是否提供批量授权由 service 明确展开。
- 后续新增动作必须同步维护后端枚举、前端多语言文案、接口文档和测试用例。

### 6.3 授权效果合并规则

有效权限合并顺序如下：

1. 收集账号在当前租户下所有启用角色。
2. 通过 `sys_role_closures` 查出这些角色的全部祖先角色和自身角色。
3. 查询这些角色的未删除、启用授权规则。
4. 按 `resource_type + resource_id + action` 合并授权结果。
5. 如果存在 `deny`，优先拒绝；否则存在 `allow` 即允许；都不存在则拒绝。

首版管理端不开放 `deny` 配置，只保留数据库字段和合并语义。这样可以避免早期产品复杂度过高，同时为后续“禁止某子角色继承某父权限”或风控拒绝策略预留空间。

## 7. 资源模型

### 7.1 资源拆表原则

应用、菜单、按钮和接口资源字段差异较大，首版不使用单一资源表。资源拆表后仍必须保持统一资源引用约定：

```text
resource_type + resource_id
```

其中 `resource_type` 固定为 `application`、`menu`、`button`、`api`。所有授权、缓存、审计、权限版本和接口返回都使用该统一引用，而资源详情读取按 `resource_type` 分发到对应 repository。这样可以同时获得两类收益：

- 资源表结构可以按业务类型表达清晰字段、约束和索引，避免大量空字段和跨类型校验。
- 授权表、角色继承、有效权限合并、缓存失效和鉴权中间件仍然保持统一，不产生四套授权系统。

所有资源表必须包含统一基础字段。所有资源编码在各自资源类型内保持租户内唯一；跨类型是否允许同名编码由 service 统一校验，首版建议允许不同类型使用相同业务后缀，但对外组合展示时必须携带 `resourceType`。

### 7.2 应用资源表 `sys_permission_applications`

应用资源表达低代码应用或系统应用，是菜单、按钮和接口资源的归属边界。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 应用资源主键。 |
| `tenant_id` | `VARCHAR(64)` | 否 | 租户应用所属租户；平台内置应用为空或固定为 `platform`。 |
| `app_code` | `VARCHAR(128)` | 是 | 应用编码，建议使用稳定业务编码，例如 `hrm`。 |
| `name` | `VARCHAR(128)` | 是 | 应用默认名称。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 应用名称全拼，用于搜索。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 应用名称简拼，用于搜索。 |
| `platform` | `VARCHAR(32)` | 是 | 适用端，例如 `web`、`mobile`、`both`。 |
| `home_path` | `VARCHAR(512)` | 否 | 应用首页路径。 |
| `icon` | `VARCHAR(128)` | 否 | 应用图标标识。 |
| `sort_no` | `BIGINT` | 是 | 应用排序号。 |
| `status` | `VARCHAR(32)` | 是 | 状态：`enabled`、`disabled`。 |
| `remark` | `VARCHAR(512)` | 否 | 备注。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + app_code` 在未删除数据中唯一。
- 建立 `tenant_id + deleted + status` 索引，支持可访问应用查询。
- 建立 `tenant_id + deleted + sort_no` 索引，支持应用排序展示。

### 7.3 菜单资源表 `sys_permission_menus`

菜单资源表达 Web 或移动端导航、页面入口和隐藏路由。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 菜单资源主键。 |
| `tenant_id` | `VARCHAR(64)` | 否 | 租户菜单所属租户；平台内置菜单为空或固定为 `platform`。 |
| `app_id` | `VARCHAR(64)` | 是 | 所属应用 ID。 |
| `parent_id` | `VARCHAR(64)` | 否 | 父菜单 ID，根菜单为空。 |
| `menu_code` | `VARCHAR(128)` | 是 | 菜单编码，例如 `hrm.user`。 |
| `name` | `VARCHAR(128)` | 是 | 菜单默认名称。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 菜单名称全拼。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 菜单名称简拼。 |
| `platform` | `VARCHAR(32)` | 是 | 适用端，例如 `web`、`mobile`、`both`。 |
| `route_path` | `VARCHAR(512)` | 是 | 前端路由路径。 |
| `component` | `VARCHAR(255)` | 否 | 前端组件路径或低代码页面标识。 |
| `icon` | `VARCHAR(128)` | 否 | 菜单图标标识。 |
| `visible` | `BOOLEAN` | 是 | 是否在导航中展示。 |
| `keep_alive` | `BOOLEAN` | 是 | 前端是否缓存页面状态。 |
| `sort_no` | `BIGINT` | 是 | 同级排序号。 |
| `status` | `VARCHAR(32)` | 是 | 状态：`enabled`、`disabled`。 |
| `remark` | `VARCHAR(512)` | 否 | 备注。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + menu_code` 在未删除数据中唯一。
- 建立 `tenant_id + app_id + deleted + status` 索引，支持按应用加载菜单。
- 建立 `tenant_id + parent_id + deleted + sort_no` 索引，支持菜单树查询。
- `app_id` 必须引用同租户或平台内置有效应用。
- `parent_id` 不为空时必须引用同应用下有效菜单，并禁止形成菜单树循环。

### 7.4 按钮资源表 `sys_permission_buttons`

按钮资源表达页面内可点击动作，也可用于工具栏动作、行操作和移动端操作入口。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 按钮资源主键。 |
| `tenant_id` | `VARCHAR(64)` | 否 | 租户按钮所属租户；平台内置按钮为空或固定为 `platform`。 |
| `app_id` | `VARCHAR(64)` | 是 | 所属应用 ID。 |
| `menu_id` | `VARCHAR(64)` | 是 | 所属菜单或页面 ID。 |
| `button_code` | `VARCHAR(128)` | 是 | 按钮编码，例如 `hrm.user.create`。 |
| `name` | `VARCHAR(128)` | 是 | 按钮默认名称。 |
| `action_key` | `VARCHAR(64)` | 是 | 前端动作标识，例如 `create`、`update`、`delete`。 |
| `button_type` | `VARCHAR(32)` | 是 | 按钮类型，例如 `toolbar`、`row`、`batch`、`mobile_action`。 |
| `icon` | `VARCHAR(128)` | 否 | 按钮图标标识。 |
| `sort_no` | `BIGINT` | 是 | 按钮排序号。 |
| `status` | `VARCHAR(32)` | 是 | 状态：`enabled`、`disabled`。 |
| `remark` | `VARCHAR(512)` | 否 | 备注。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + button_code` 在未删除数据中唯一。
- 建立 `tenant_id + menu_id + deleted + status + sort_no` 索引，支持按菜单加载按钮。
- `app_id` 和 `menu_id` 必须引用同租户或平台内置有效资源。
- 按钮权限只控制前端操作入口，不得替代 API 资源的后端接口鉴权。

### 7.5 接口资源表 `sys_permission_apis`

接口资源表达后端受保护 HTTP API，是后端权限中间件的鉴权对象。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 接口资源主键。 |
| `tenant_id` | `VARCHAR(64)` | 否 | 租户自定义接口所属租户；平台内置接口为空或固定为 `platform`。 |
| `app_id` | `VARCHAR(64)` | 否 | 所属应用 ID，系统通用接口可为空。 |
| `api_code` | `VARCHAR(128)` | 是 | 接口编码，例如 `hrm.user.create_api`。 |
| `name` | `VARCHAR(128)` | 是 | 接口默认名称。 |
| `http_method` | `VARCHAR(16)` | 是 | HTTP 方法，例如 `GET`、`POST`。 |
| `path_pattern` | `VARCHAR(512)` | 是 | 后端路径模板，例如 `/api/v1/hrm/users/{id}`。 |
| `normalized_path` | `VARCHAR(512)` | 是 | 规范化路径模板，用于快速匹配。 |
| `related_menu_id` | `VARCHAR(64)` | 否 | 关联菜单 ID，用于管理端展示。 |
| `related_button_id` | `VARCHAR(64)` | 否 | 关联按钮 ID，用于管理端提示。 |
| `public_access` | `BOOLEAN` | 是 | 是否公开访问，仅允许受控系统接口使用。 |
| `auth_required` | `BOOLEAN` | 是 | 是否需要认证，公开健康检查等接口可为 false。 |
| `status` | `VARCHAR(32)` | 是 | 状态：`enabled`、`disabled`。 |
| `remark` | `VARCHAR(512)` | 否 | 备注。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + api_code` 在未删除数据中唯一。
- `http_method + normalized_path` 在未删除且启用的 API 资源中唯一；租户自定义接口如需覆盖平台接口，必须单独设计优先级，不得静默冲突。
- 建立 `http_method + normalized_path + deleted + status` 索引，支持权限中间件快速匹配。
- 建立 `tenant_id + app_id + deleted + status` 索引，支持按应用管理接口资源。
- `public_access = true` 必须由系统管理员或 migration 设置，普通租户管理员不得随意开放接口。

### 7.6 资源层级规则

推荐资源层级如下：

```text
application
  menu
    menu
      button
  api
```

约束如下：

- 应用资源是顶层归属边界，不依赖父资源字段。
- 菜单资源通过 `app_id` 归属应用，通过 `parent_id` 形成菜单树。
- 按钮资源通过 `app_id` 和 `menu_id` 归属具体应用页面。
- API 资源可以通过 `app_id` 归属应用，也可以通过 `related_menu_id` 或 `related_button_id` 关联前端操作入口。
- 删除应用前必须确认不存在未删除菜单、按钮、接口、角色授权和低代码配置引用。
- 逻辑删除任意资源时必须同步逻辑删除该资源相关授权，或保证权限查询默认过滤已删除资源。

### 7.7 API 资源匹配

API 资源路径必须使用后端路由风格的稳定模板，例如：

```text
GET /api/v1/hrm/users
POST /api/v1/hrm/users
PUT /api/v1/hrm/users/{id}
DELETE /api/v1/hrm/users/{id}
```

匹配规则如下：

- 请求进入后端后，鉴权中间件先规范化 HTTP method 和路径。
- 动态路径参数统一匹配 `{name}` 形式，不允许把真实业务 ID 写入资源表。
- 查询参数不参与 API 资源匹配；需要区分动作时，应拆分明确路径或动作字段。
- 公开接口必须显式注册为 `public_access = true`，并在代码中保留最小公开路径白名单作为启动期兜底。
- 未注册 API 资源默认拒绝，开发环境可以通过配置记录告警但不放行，生产环境必须拒绝。

## 8. 角色模型

### 8.1 角色表 `sys_roles`

角色属于租户内权限主体。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 角色主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `role_code` | `VARCHAR(64)` | 是 | 角色编码，租户内未删除数据唯一。 |
| `name` | `VARCHAR(128)` | 是 | 角色名称。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 角色名称全拼。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 角色名称简拼。 |
| `role_type` | `VARCHAR(32)` | 是 | 角色类型，例如 `system`、`custom`。 |
| `status` | `VARCHAR(32)` | 是 | 状态：`enabled`、`disabled`。 |
| `sort_no` | `BIGINT` | 是 | 排序号。 |
| `remark` | `VARCHAR(512)` | 否 | 备注。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + role_code` 在未删除数据中唯一。
- 建立 `tenant_id + deleted + status` 索引，支持账号有效角色查询。
- 建立 `tenant_id + deleted + sort_no` 索引，支持管理端分页和排序。
- 系统角色可由 migration 初始化，普通租户管理员不得物理删除系统角色。

### 8.2 角色继承关系表 `sys_role_relations`

角色直接父子关系单独存储。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `parent_role_id` | `VARCHAR(64)` | 是 | 父角色 ID。 |
| `child_role_id` | `VARCHAR(64)` | 是 | 子角色 ID。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束如下：

- `tenant_id + parent_role_id + child_role_id` 在未删除数据中唯一。
- 禁止父角色等于子角色。
- 禁止创建会形成循环的继承关系。
- 父角色和子角色必须属于同一租户、未删除且启用。
- 删除角色前必须先删除其直接继承关系、账号角色关系和授权关系。

### 8.3 角色闭包表 `sys_role_closures`

角色闭包表用于高性能查询所有祖先角色。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `ancestor_role_id` | `VARCHAR(64)` | 是 | 祖先角色 ID。 |
| `descendant_role_id` | `VARCHAR(64)` | 是 | 后代角色 ID。 |
| `depth` | `BIGINT` | 是 | 距离，`0` 表示自身。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + ancestor_role_id + descendant_role_id` 在未删除数据中唯一。
- 每个角色必须存在一条自身到自身的闭包记录，`depth = 0`。
- 建立 `tenant_id + descendant_role_id + deleted` 索引，支持查询角色所有祖先。
- 建立 `tenant_id + ancestor_role_id + deleted` 索引，支持查询角色所有后代和权限影响范围。

维护规则：

- 新增角色时，在同一事务中插入自身闭包记录。
- 新增父子关系时，在同一事务中插入父角色所有祖先到子角色所有后代的组合闭包。
- 删除父子关系时，需要重算受影响子树闭包，避免误删仍由其他父角色提供的继承路径。
- 角色继承深度应设置服务端上限，首版建议最大 10 层，避免错误配置导致维护成本失控。

## 9. 授权与账号角色关系

### 9.1 账号角色表 `sys_account_roles`

账号角色关系用于把角色授予认证账号。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `account_id` | `VARCHAR(64)` | 是 | 认证账号 ID。 |
| `role_id` | `VARCHAR(64)` | 是 | 角色 ID。 |
| `status` | `VARCHAR(32)` | 是 | 状态：`enabled`、`disabled`。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束如下：

- `tenant_id + account_id + role_id` 在未删除数据中唯一。
- 账号必须属于当前租户，且账号租户关系启用。
- 角色必须属于当前租户，且角色启用。
- 禁用账号角色关系后，该角色及其继承权限不再参与有效权限计算。

### 9.2 授权规则表 `sys_permission_grants`

授权规则是权限系统的核心表。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `subject_type` | `VARCHAR(32)` | 是 | 主体类型，首版管理端使用 `role`。 |
| `subject_id` | `VARCHAR(64)` | 是 | 主体 ID。 |
| `resource_type` | `VARCHAR(32)` | 是 | 资源类型：`application`、`menu`、`button`、`api`。 |
| `resource_id` | `VARCHAR(64)` | 是 | 资源 ID。 |
| `action` | `VARCHAR(32)` | 是 | 权限动作。 |
| `effect` | `VARCHAR(16)` | 是 | 授权效果：`allow`、`deny`。 |
| `grant_source` | `VARCHAR(32)` | 是 | 来源：`manual`、`system`、`migration`、`import`。 |
| `condition_type` | `VARCHAR(32)` | 否 | 条件类型，首版为空，后续扩展受控条件权限。 |
| `condition_value` | `TEXT` | 否 | 条件参数，首版为空。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + subject_type + subject_id + resource_type + resource_id + action` 在未删除数据中唯一。
- 建立 `tenant_id + subject_type + subject_id + deleted` 索引，支持查询角色直接权限。
- 建立 `tenant_id + resource_type + resource_id + action + deleted` 索引，支持资源授权页面反查主体。
- 建立 `tenant_id + deleted + updated_time` 索引，支持权限版本更新和审计查询。
- `subject_type = role` 时，`subject_id` 必须引用同租户有效角色。
- `resource_type + resource_id` 必须引用同租户或平台全局有效资源；不同资源类型由 service 分发到对应资源表校验。
- `action` 必须符合 `resource_type` 对应动作白名单。

## 10. 权限版本与缓存

### 10.1 权限版本表 `sys_permission_versions`

权限版本用于客户端资源刷新和服务端缓存失效。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 主键，建议使用租户 ID。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 所属租户。 |
| `version_no` | `BIGINT` | 是 | 权限版本号，每次权限相关变更递增。 |
| `changed_reason` | `VARCHAR(128)` | 否 | 最近变更原因。 |
| 基础字段 | - | 是 | 统一基础字段。 |

以下变更必须递增权限版本：

- 资源新增、修改、启用、禁用、逻辑删除。
- 角色新增、修改、启用、禁用、逻辑删除。
- 角色继承关系变更。
- 账号角色关系变更。
- 授权规则变更。

### 10.2 缓存策略

服务端推荐维护以下缓存：

| 缓存 | Key | Value | 失效条件 |
| --- | --- | --- | --- |
| API 资源匹配缓存 | `method + normalized_path` | API 资源 ID、publicAccess、状态 | API 资源版本变化。 |
| 账号有效权限缓存 | `tenantId + accountId + permissionVersion` | `resourceType + resourceId + action + effect` 集合 | 权限版本变化或账号角色变化。 |
| 菜单树缓存 | `tenantId + accountId + platform + locale + permissionVersion` | 当前账号可见菜单树 | 权限版本、资源版本或 locale 变化。 |
| 按钮权限缓存 | `tenantId + accountId + menuId + permissionVersion` | 按钮编码集合 | 权限版本变化。 |

缓存要求如下：

- 缓存不得成为权限权威来源，数据库授权规则和闭包表才是最终来源。
- 缓存 key 必须包含权限版本，避免授权变更后旧权限继续生效。
- 本地内存缓存适合单实例开发和早期部署；多实例部署必须接入集中式缓存或基于版本号的实例内自失效。
- 接口鉴权缓存命中后仍需保证账号、租户和会话已由认证中间件校验有效。
- 缓存异常时必须降级为数据库查询或拒绝访问，不得默认放行。

## 11. 鉴权链路

### 11.1 HTTP 接口鉴权

受保护接口请求链路如下：

1. traceId 中间件写入请求链路 ID。
2. locale 和 time zone 中间件写入语言上下文。
3. 租户中间件解析租户。
4. 认证中间件校验访问令牌、会话、账号和账号租户关系。
5. 权限中间件根据 `method + path` 匹配 API 资源。
6. 如果 API 资源 `public_access = true`，记录公开访问并放行。
7. 如果未匹配 API 资源，生产环境返回 `-403` 或权限资源缺失错误。
8. 查询当前账号有效权限，判断是否拥有该 API 资源的 `call` 权限。
9. 放行或返回 `-403`，并记录拒绝原因、资源类型、资源 ID、账号 ID、租户 ID 和 traceId。

权限中间件必须位于认证中间件之后，因为权限判断依赖 `RequestContext.user_id` 和 token 中的租户上下文。

### 11.2 菜单和按钮权限

当前账号前端权限初始化流程如下：

1. 客户端登录成功后调用当前账号信息接口。
2. 客户端调用权限资源接口获取当前账号应用、菜单和按钮权限。
3. 后端按当前账号有效权限过滤资源树。
4. 菜单返回树形结构，按钮返回当前菜单或全部菜单下的按钮编码集合。
5. 前端按结果渲染导航和按钮，但所有业务 API 仍由后端权限中间件兜底校验。

菜单可见规则如下：

- 拥有菜单 `view` 权限时，该菜单可见。
- 子菜单可见但父菜单没有 `view` 权限时，后端可以补齐父级容器菜单，但必须标记 `inheritedVisible = true`，避免前端断树。
- 禁用或逻辑删除的菜单不得返回。
- `visible = false` 的菜单不显示在导航中，但可作为隐藏路由由权限判断保护。

按钮可用规则如下：

- 拥有按钮 `click` 权限时，按钮可显示并可点击。
- 如果按钮绑定 API 资源，点击后端接口仍需 `api.call` 权限。
- 不建议通过按钮权限自动推导 API 权限，避免隐藏按钮与直接调用接口的安全边界混淆。

## 12. 管理端接口契约

所有接口必须使用统一 `ApiResponse<T>`，路径使用 `snake_case`，JSON 字段使用 `camelCase`。

### 12.1 资源管理接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/api/v1/permission/applications` | 新增应用资源。 |
| `PUT` | `/api/v1/permission/applications/{id}` | 修改应用资源，必须携带 `version`。 |
| `DELETE` | `/api/v1/permission/applications/{id}?version={version}` | 逻辑删除应用资源。 |
| `GET` | `/api/v1/permission/applications` | 分页查询应用资源。 |
| `POST` | `/api/v1/permission/menus` | 新增菜单资源。 |
| `PUT` | `/api/v1/permission/menus/{id}` | 修改菜单资源，必须携带 `version`。 |
| `DELETE` | `/api/v1/permission/menus/{id}?version={version}` | 逻辑删除菜单资源。 |
| `GET` | `/api/v1/permission/menu_tree` | 查询菜单资源树。 |
| `POST` | `/api/v1/permission/buttons` | 新增按钮资源。 |
| `PUT` | `/api/v1/permission/buttons/{id}` | 修改按钮资源，必须携带 `version`。 |
| `DELETE` | `/api/v1/permission/buttons/{id}?version={version}` | 逻辑删除按钮资源。 |
| `GET` | `/api/v1/permission/buttons` | 分页查询按钮资源。 |
| `POST` | `/api/v1/permission/apis` | 新增接口资源。 |
| `PUT` | `/api/v1/permission/apis/{id}` | 修改接口资源，必须携带 `version`。 |
| `DELETE` | `/api/v1/permission/apis/{id}?version={version}` | 逻辑删除接口资源。 |
| `GET` | `/api/v1/permission/apis` | 分页查询接口资源。 |
| `POST` | `/api/v1/permission/apis/import` | 批量导入或同步 API 资源。 |

资源详情接口按资源类型拆分，例如 `/api/v1/permission/menus/{id}` 查询菜单详情，避免统一详情接口返回大量类型分支字段。物理删除接口同样按资源类型提供 `{resource}/{id}/physical`，且必须满足已逻辑删除、无子资源、无授权引用和无低代码配置引用。

### 12.2 角色管理接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/api/v1/permission/roles` | 新增角色。 |
| `PUT` | `/api/v1/permission/roles/{id}` | 修改角色，必须携带 `version`。 |
| `DELETE` | `/api/v1/permission/roles/{id}?version={version}` | 逻辑删除角色。 |
| `DELETE` | `/api/v1/permission/roles/{id}/physical` | 物理删除角色。 |
| `GET` | `/api/v1/permission/roles/{id}` | 查询角色详情。 |
| `GET` | `/api/v1/permission/roles` | 分页查询角色。 |
| `GET` | `/api/v1/permission/role_tree` | 查询角色继承树。 |
| `POST` | `/api/v1/permission/roles/{id}/parents` | 设置角色父角色集合。 |
| `GET` | `/api/v1/permission/roles/{id}/inherited_permissions` | 查询角色继承后的有效权限。 |

### 12.3 授权管理接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/api/v1/permission/roles/{id}/permissions` | 查询角色直接授权。 |
| `PUT` | `/api/v1/permission/roles/{id}/permissions` | 覆盖保存角色直接授权。 |
| `GET` | `/api/v1/permission/resource_grants?resourceType={type}&resourceId={id}` | 查询资源授权主体。 |
| `PUT` | `/api/v1/permission/resource_grants` | 覆盖保存资源授权主体。 |
| `GET` | `/api/v1/permission/accounts/{id}/roles` | 查询账号角色。 |
| `PUT` | `/api/v1/permission/accounts/{id}/roles` | 覆盖保存账号角色。 |

角色授权和资源授权接口写入同一张 `sys_permission_grants`。覆盖保存必须携带当前权限版本或角色版本，避免两个管理员并发覆盖彼此授权。

### 12.4 当前账号权限接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `GET` | `/api/v1/permission/me/resources` | 查询当前账号可访问应用、菜单和按钮资源。 |
| `GET` | `/api/v1/permission/me/menus` | 查询当前账号菜单树。 |
| `GET` | `/api/v1/permission/me/buttons?menuId={id}` | 查询当前菜单下可用按钮。 |
| `GET` | `/api/v1/permission/me/permissions` | 查询当前账号有效权限摘要，用于调试和前端初始化。 |
| `GET` | `/api/v1/permission/version` | 查询当前租户权限版本。 |

当前账号接口必须只从认证上下文读取账号和租户，不允许通过 query 或 body 指定任意账号获取权限。

## 13. 数据一致性与事务

权限写入必须遵守以下事务边界：

- 创建角色时，角色表和自身闭包记录必须在同一事务提交。
- 设置角色父角色时，父子关系、闭包重算、权限版本递增和审计日志必须在同一事务提交。
- 覆盖角色授权时，授权差异写入、权限版本递增和审计日志必须在同一事务提交。
- 删除资源时，资源状态、相关授权逻辑删除、权限版本递增和审计日志必须在同一事务提交。
- 设置账号角色时，账号角色关系、权限版本递增和审计日志必须在同一事务提交。

并发控制要求如下：

- 所有修改接口必须携带 `version` 或权限版本。
- 更新和逻辑删除必须使用 `WHERE id = ? AND version = ? AND deleted = FALSE`。
- 角色继承关系变更必须对相关角色或租户权限版本加锁，避免闭包表并发重算交错。
- 批量授权必须限制单次资源数量和主体数量，超过上限使用导入任务或分批提交。

## 14. 多租户与系统资源

权限资源分为平台内置资源和租户自定义资源：

| 范围 | 说明 |
| --- | --- |
| 平台内置资源 | 后端、前端和移动端随版本发布的固定资源，例如系统菜单和内置接口。 |
| 租户自定义资源 | 租户在低代码平台中创建的应用、页面、按钮和接口扩展资源。 |

规则如下：

- 平台内置资源应通过 migration 初始化或受控同步接口维护。
- 租户自定义资源必须携带 `tenant_id`。
- 租户角色可以被授予平台内置资源和本租户资源，不得被授予其他租户资源。
- 平台超级管理员能力后续单独设计，不得通过普通租户角色跨租户授权。

## 15. 多语言与前端消费

资源名称、菜单标题、按钮文案和授权动作文案必须支持多语言。

规则如下：

- 资源表中的 `name` 是默认语言原始值。
- 资源展示给前端时，应通过业务翻译机制按当前 locale 返回本地化名称。
- 系统内置资源文案可通过系统多语言资源维护，业务低代码资源文案通过业务翻译表维护。
- 前端和移动端不得硬编码权限资源名称、按钮文案和菜单标题。
- 权限动作、资源类型、角色状态等枚举必须同步维护 `zh-CN` 和 `en-US` 文案。

## 16. 性能与容量设计

首版容量目标建议如下：

| 指标 | 建议目标 |
| --- | --- |
| 单租户角色数量 | 1,000 以内，设计上支持 10,000。 |
| 单租户资源数量 | 10,000 以内，低代码应用增长后支持 100,000。 |
| 单账号有效角色数量 | 常规 20 以内，服务端限制最大 200。 |
| 角色继承深度 | 常规 3 到 5 层，服务端限制最大 10。 |
| 单角色直接授权数量 | 常规 1,000 以内，服务端限制最大 20,000。 |
| 高频 API 鉴权耗时 | 缓存命中目标 1 毫秒内，不含业务处理。 |

性能要求如下：

- 接口鉴权不得在请求链路递归遍历角色父子关系。
- 当前账号权限查询必须批量加载资源和授权，不得对每个菜单或按钮执行单独查询。
- 菜单树构建应先批量查询可见资源，再在内存中组树。
- API 资源匹配应优先使用规范化路径缓存，避免每次请求扫描所有路径模板。
- 大批量授权保存必须进行差异化写入，只更新新增、删除或变更的授权记录。
- 权限审计日志应异步或批量写入评估，避免高频拒绝请求拖慢正常接口。

## 17. 错误码与安全响应

权限模块错误必须使用统一业务码：

| 场景 | 业务码 | 说明 |
| --- | --- | --- |
| 未登录访问受保护资源 | `-401` | 认证失败或登录过期。 |
| 已登录但无权限 | `-403` | 权限不足。 |
| 授权资源不存在 | `-404` | 资源不存在或已删除。 |
| 角色继承形成循环 | `-422` | 业务规则失败。 |
| 授权版本冲突 | `-409` | 并发修改导致版本不匹配。 |
| 权限缓存或数据库异常 | `-500` | 系统错误，不泄露内部细节。 |

安全要求如下：

- 对外响应不得暴露完整角色继承链、内部 SQL、缓存 key 或策略细节。
- 鉴权拒绝日志必须记录 `traceId`、`tenantId`、`accountId`、`resourceType`、`resourceId`、`action` 和拒绝原因。
- 未注册 API 资源在生产环境应记录高优先级告警，避免新接口绕过权限治理。

## 18. 测试与验收

权限模块至少需要以下测试：

- migration 契约测试：MySQL 和 PostgreSQL 表、索引、唯一约束和基础字段保持一致。
- 资源模型测试：资源编码唯一、资源层级合法、API method/path 约束合法。
- 角色继承测试：新增继承、删除继承、多父角色、循环检测、闭包重算。
- 授权合并测试：直接授权、继承授权、重复授权、`deny` 优先级。
- 账号有效权限测试：账号角色启用禁用、角色禁用、资源禁用、租户隔离。
- API 鉴权测试：无 token、无权限、有权限、公开接口、未注册接口。
- 菜单按钮测试：菜单树过滤、父菜单补齐、隐藏菜单、按钮权限集合。
- 缓存失效测试：资源、角色、授权和账号角色变更后权限版本递增且旧缓存失效。
- 多语言测试：资源名称、枚举文案和 fallback 结果符合 `zh-CN` 与 `en-US` 要求。
- 并发测试：两个管理员同时修改角色授权或继承关系时必须产生版本冲突或串行化结果。

## 19. 后续扩展方向

权限模块后续可在统一授权模型上扩展：

- 账号直授权：`subject_type = account`。
- 组织授权：`subject_type = org`，结合 HRM 组织树和任职关系计算有效主体。
- 岗位授权：`subject_type = post`，适合按岗位授予业务能力。
- 数据权限：新增数据范围规则表，与资源动作权限分开判断。
- 字段权限：新增字段资源类型或字段策略表，不污染 API 资源授权。
- 临时授权：增加授权有效期字段和定时回收任务。
- 外部角色映射：把 OIDC、LDAP 或企业通讯录角色映射到内部 `sys_roles`。

这些扩展必须继续复用统一授权规则、权限版本、缓存失效和审计机制，避免形成多套并行权限系统。
