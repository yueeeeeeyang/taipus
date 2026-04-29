# HRM 人力资源主数据模块设计说明

## 1. 文档目标

本文档用于约定低代码平台 HRM 模块的业务边界、数据模型、接口契约、内部服务能力、查询规则、删除规则和测试验收要求。后续实现 HRM 后端模块时必须优先遵守本文档，除非业务范围、权限模型或容量目标发生变化并重新评审。

HRM 首版定位为人力资源主数据模块，核心目标如下：

- **主数据稳定**：统一维护用户、组织机构、岗位及用户组织岗位关系，为其他业务模块提供稳定的人力资源引用数据。
- **边界清晰**：首版不承接登录认证、密码凭证、菜单权限、数据权限和角色授权，角色后续由权限模块单独设计。
- **查询友好**：所有名称字段同步维护全拼和简拼，支持中文名称、拼音全拼、拼音简拼的统一模糊搜索。
- **一致写入**：所有持久化实体显式包含统一基础字段，并通过乐观锁、逻辑删除和审计字段保证写入可追踪。
- **可扩展**：租户字段、状态字段、组织类型和多组织多岗位关系在首版即明确建模，避免后续扩展破坏接口契约。

## 2. 模块边界

HRM 首版管理以下对象：

| 对象 | 说明 |
| --- | --- |
| 用户 | 企业人员主数据，核心字段为工号和姓名，不等同于登录账号。 |
| 组织机构 | 分部、部门两类组织树节点，用于表达组织层级和人员归属。 |
| 岗位 | 岗位主数据，用于表达人员承担的岗位职责或组织内任职信息。 |
| 用户组织岗位关系 | 用户、组织、岗位之间的多对多任职关系，支持主组织和主岗位标记。 |

HRM 首版明确不包含以下能力：

- 登录账号、密码、令牌、会话、单点登录和登录策略。
- 角色、权限点、菜单资源、按钮权限、数据权限和授权策略。
- 薪酬、考勤、绩效、入转调离审批等 HR 业务流程。
- 对外模块写入接口。其他业务模块首版只能通过内部 Rust Service 读取或校验 HRM 主数据。

HRM 与其他模块的关系如下：

- 其他业务模块可以引用 HRM 的用户、组织和岗位 ID，但不得绕过 HRM Service 直接解释 HRM 表结构。
- 权限模块后续可以基于 HRM 用户、组织和岗位建立角色、权限和数据范围规则。
- 鉴权模块后续可以把登录账号绑定到 HRM 用户，但 HRM 用户本身不保存密码凭证。

## 3. 数据模型

### 3.1 通用字段约定

所有 HRM 持久化表必须包含以下统一基础字段，并在 Rust 模型中平铺声明，不得通过 `BaseFields`、实体 trait、通用父类或通用 repository 隐藏基础字段。

| 数据库列名 | Rust 模型字段 | JSON DTO 字段 | 写入规则 |
| --- | --- | --- | --- |
| `version` | `version` | `version` | 新增为 `1`，修改和逻辑删除时递增，用于乐观锁。 |
| `deleted` | `deleted` | `deleted` | 新增为 `false`，逻辑删除时改为 `true`。 |
| `created_by` | `created_by` | `createdBy` | 新增时写入当前用户 ID；系统初始化数据使用 `system`。 |
| `created_time` | `created_time` | `createdTime` | 新增时写入当前 UTC 时间。 |
| `updated_by` | `updated_by` | `updatedBy` | 新增和修改时写入当前用户 ID。 |
| `updated_time` | `updated_time` | `updatedTime` | 新增和修改时写入当前 UTC 时间。 |
| `deleted_by` | `deleted_by` | `deletedBy` | 未删除为空，逻辑删除时写入当前用户 ID。 |
| `deleted_time` | `deleted_time` | `deletedTime` | 未删除为空，逻辑删除时写入当前 UTC 时间。 |

HRM 表还必须包含 `tenant_id`。即使首版运行在单租户部署中，也要通过 `tenant_id` 保留未来多租户隔离能力；单租户场景可使用配置默认租户或请求上下文中的默认值。

所有带 `name` 字段的模型必须同步包含：

| 数据库列名 | JSON DTO 字段 | 说明 |
| --- | --- | --- |
| `name_full_pinyin` | `nameFullPinyin` | 名称全拼，由后端根据 `name` 自动生成。 |
| `name_simple_pinyin` | `nameSimplePinyin` | 名称简拼，由后端根据 `name` 自动生成。 |

拼音字段生成规则如下：

- 新增和修改名称时，service 层必须调用 `utils::pinyin::to_pinyin_text` 生成全拼和简拼。
- 前端传入的拼音字段一律不可信，不得作为最终写入值。
- 多音字首版使用现有拼音库默认读音；如后续需要人工纠音，应新增业务别名字段或纠音配置，不得在通用拼音工具中硬编码领域读音。

### 3.2 用户表 `hrm_users`

用户表保存人员主数据，不保存密码和登录凭证。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 用户主键，由后端统一 ID 工具生成。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 租户标识。 |
| `employee_no` | `VARCHAR(64)` | 是 | 工号，租户内未删除数据唯一。 |
| `name` | `VARCHAR(128)` | 是 | 用户姓名。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 姓名全拼。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 姓名简拼。 |
| `mobile` | `VARCHAR(32)` | 否 | 手机号，首版不作为登录账号。 |
| `email` | `VARCHAR(255)` | 否 | 邮箱，首版不作为登录账号。 |
| `sort_no` | `BIGINT` | 是 | 用户排序号，数值越小越靠前。 |
| `status` | `VARCHAR(32)` | 是 | 用户状态，建议枚举为 `enabled`、`disabled`。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + employee_no` 在未删除数据中唯一。
- 建立 `tenant_id + deleted + status` 索引，支持候选用户查询。
- 建立 `tenant_id + deleted + sort_no` 索引，支持用户列表按排序号展示。
- 建立 `tenant_id + deleted + updated_time` 索引，支持分页稳定排序。
- 建立姓名和拼音相关查询索引；MySQL 可先使用普通索引配合前缀匹配，复杂模糊搜索后续再评估全文索引或搜索引擎。

### 3.3 组织机构表 `hrm_orgs`

组织机构表保存组织树节点。首版组织类型暂定为分部和部门两种，并在 service 层强制校验父子层级规则。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 组织主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 租户标识。 |
| `parent_id` | `VARCHAR(64)` | 否 | 父组织 ID，根节点为空。 |
| `org_code` | `VARCHAR(64)` | 是 | 组织编码，租户内未删除数据唯一。 |
| `name` | `VARCHAR(128)` | 是 | 组织名称。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 组织名称全拼。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 组织名称简拼。 |
| `org_type` | `VARCHAR(32)` | 是 | 组织类型，首版枚举为 `branch`（分部）和 `department`（部门）。 |
| `sort_no` | `BIGINT` | 是 | 同级排序号，数值越小越靠前。 |
| `status` | `VARCHAR(32)` | 是 | 组织状态，建议枚举为 `enabled`、`disabled`。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + org_code` 在未删除数据中唯一。
- 建立 `tenant_id + parent_id + deleted + sort_no` 索引，支持组织树查询。
- 建立 `tenant_id + deleted + status` 索引，支持有效组织候选查询。
- 建立 `tenant_id + deleted + sort_no` 索引，支持组织普通列表按排序号展示。
- 组织树查询默认只返回未删除节点；是否展示禁用节点由接口参数控制，默认展示未删除的全部节点，避免历史组织结构断裂。

组织层级规则如下：

- 根节点只能是分部，禁止创建无父级部门。
- 分部下可以创建下级分部，也可以创建部门。
- 部门下只能创建下级部门，禁止创建分部。
- 创建组织时如果传入 `parentId`，必须校验父组织存在、未删除、启用，并校验父组织类型允许当前组织类型。
- 修改组织的 `parentId` 或 `orgType` 时，必须重新校验父子层级规则，并检查不会形成循环上下级。
- 循环检查必须基于同租户、未删除组织的祖先链完成；目标父组织不得等于当前组织，也不得是当前组织的任意后代。
- 修改 `orgType` 时还必须校验当前组织的未删除直接子组织；如果当前组织改为部门，则其下不得存在分部子节点，避免形成“部门下有分部”的非法结构。

### 3.4 岗位表 `hrm_posts`

岗位表保存岗位主数据。岗位本身不强制绑定单个组织，用户任职时通过关系表指定组织和岗位。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 岗位主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 租户标识。 |
| `post_code` | `VARCHAR(64)` | 是 | 岗位编码，租户内未删除数据唯一。 |
| `name` | `VARCHAR(128)` | 是 | 岗位名称。 |
| `name_full_pinyin` | `VARCHAR(256)` | 是 | 岗位名称全拼。 |
| `name_simple_pinyin` | `VARCHAR(128)` | 是 | 岗位名称简拼。 |
| `sort_no` | `BIGINT` | 是 | 排序号，数值越小越靠前。 |
| `status` | `VARCHAR(32)` | 是 | 岗位状态，建议枚举为 `enabled`、`disabled`。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + post_code` 在未删除数据中唯一。
- 建立 `tenant_id + deleted + status` 索引，支持有效岗位候选查询。
- 建立 `tenant_id + deleted + sort_no` 索引，支持岗位列表按排序号展示。
- 建立 `tenant_id + deleted + updated_time` 索引，支持分页稳定排序。

### 3.5 用户组织岗位关系表 `hrm_user_org_posts`

关系表表达用户在某组织下担任某岗位。用户可以拥有多个组织和多个岗位，并可标记一个主组织和一个主岗位关系。

| 字段 | 类型建议 | 必填 | 说明 |
| --- | --- | --- | --- |
| `id` | `VARCHAR(64)` | 是 | 关系主键。 |
| `tenant_id` | `VARCHAR(64)` | 是 | 租户标识。 |
| `user_id` | `VARCHAR(64)` | 是 | 用户 ID。 |
| `org_id` | `VARCHAR(64)` | 是 | 组织 ID。 |
| `post_id` | `VARCHAR(64)` | 是 | 岗位 ID。 |
| `primary_org` | `BOOLEAN` | 是 | 是否为该用户主组织关系。 |
| `primary_post` | `BOOLEAN` | 是 | 是否为该用户主岗位关系。 |
| `sort_no` | `BIGINT` | 是 | 用户任职关系排序号。 |
| 基础字段 | - | 是 | 统一基础字段。 |

约束与索引要求：

- `tenant_id + user_id + org_id + post_id` 在未删除数据中唯一，避免重复任职关系。
- 每个用户在未删除关系中最多只能有一个 `primary_org = true` 的关系，并必须通过数据库唯一约束兜底防止并发写入破坏主组织唯一性。
- 每个用户在未删除关系中最多只能有一个 `primary_post = true` 的关系，并必须通过数据库唯一约束兜底防止并发写入破坏主岗位唯一性。
- 建立 `tenant_id + user_id + deleted` 索引，支持查询用户任职信息。
- 建立 `tenant_id + org_id + deleted` 索引，支持查询组织下用户。
- 建立 `tenant_id + post_id + deleted` 索引，支持查询岗位下用户。

写入关系时必须显式校验：

- 用户、组织和岗位存在，且 `deleted = false`。
- 新增关系时用户、组织和岗位都必须为 `enabled`。
- 禁用已有用户、组织或岗位不强制删除历史关系，但后续不得再基于禁用对象新增关系。
- 设置新的主组织或主岗位时，必须在同一事务中取消该用户其他有效关系的对应主标记，避免并发请求产生多个主关系。

## 4. 管理端 HTTP 接口

HRM 管理端接口只服务 HRM 主数据维护。所有接口必须返回统一 `ApiResponse<T>`，路径使用 `snake_case`，JSON 字段使用 `camelCase`。

### 4.1 用户接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/api/v1/hrm/users` | 新增用户。 |
| `PUT` | `/api/v1/hrm/users/{id}` | 修改用户，必须携带 `version`。 |
| `DELETE` | `/api/v1/hrm/users/{id}?version={version}` | 逻辑删除用户，必须通过 query 参数携带 `version`。 |
| `DELETE` | `/api/v1/hrm/users/{id}/physical` | 物理删除用户。 |
| `GET` | `/api/v1/hrm/users/{id}` | 根据 ID 查询用户详情。 |
| `GET` | `/api/v1/hrm/users` | 分页查询用户。 |

新增用户请求至少包含 `employeeNo`、`name`、`sortNo`、`status`，可选包含 `mobile`、`email`。修改用户时禁止修改 `id`、`tenantId`、基础创建字段和删除字段；如修改 `name`，必须重新生成拼音字段。

用户物理删除必须满足：

- 用户已经逻辑删除。
- 用户不存在任何未删除的 `hrm_user_org_posts` 引用。
- 后续如权限模块引用用户，必须通过权限模块提供的引用检查扩展点确认无有效引用。

### 4.2 组织接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/api/v1/hrm/orgs` | 新增组织。 |
| `PUT` | `/api/v1/hrm/orgs/{id}` | 修改组织，必须携带 `version`。 |
| `DELETE` | `/api/v1/hrm/orgs/{id}?version={version}` | 逻辑删除组织，必须通过 query 参数携带 `version`。 |
| `DELETE` | `/api/v1/hrm/orgs/{id}/physical` | 物理删除组织。 |
| `GET` | `/api/v1/hrm/orgs/{id}` | 根据 ID 查询组织详情。 |
| `GET` | `/api/v1/hrm/orgs` | 分页查询组织。 |
| `GET` | `/api/v1/hrm/org_tree` | 查询组织树。 |

新增组织请求至少包含 `orgCode`、`name`、`orgType`、`status`、`sortNo`，可选包含 `parentId`。如果存在 `parentId`，必须校验父组织存在、未删除且为 `enabled`，并按“分部可包含分部和部门、部门只能包含部门”的规则校验父子类型合法性。

修改组织时如调整 `parentId` 或 `orgType`，必须在同一事务内完成父组织存在性、父子类型、子组织类型和循环上下级检查。循环检查失败、父子类型不合法、把组织移动到自身后代下，或把仍存在分部子节点的组织改为部门时，必须返回业务规则失败，禁止依赖数据库外键或递归查询异常表达业务错误。

组织物理删除必须满足：

- 组织已经逻辑删除。
- 组织不存在未删除子组织。
- 组织不存在未删除的 `hrm_user_org_posts` 引用。

组织逻辑删除默认不级联删除子组织和任职关系。若请求删除仍有未删除子组织或有效任职关系的组织，service 必须返回业务规则失败，提示调用方先处理子组织或关系数据。

### 4.3 岗位接口

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/api/v1/hrm/posts` | 新增岗位。 |
| `PUT` | `/api/v1/hrm/posts/{id}` | 修改岗位，必须携带 `version`。 |
| `DELETE` | `/api/v1/hrm/posts/{id}?version={version}` | 逻辑删除岗位，必须通过 query 参数携带 `version`。 |
| `DELETE` | `/api/v1/hrm/posts/{id}/physical` | 物理删除岗位。 |
| `GET` | `/api/v1/hrm/posts/{id}` | 根据 ID 查询岗位详情。 |
| `GET` | `/api/v1/hrm/posts` | 分页查询岗位。 |

新增岗位请求至少包含 `postCode`、`name`、`status`、`sortNo`。岗位物理删除必须满足岗位已经逻辑删除，且不存在未删除的 `hrm_user_org_posts` 引用。

### 4.4 用户组织岗位关系接口

关系数据属于 HRM 主数据维护范围，但不作为其他模块写入口开放。

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| `POST` | `/api/v1/hrm/user_org_posts` | 新增用户任职关系。 |
| `PUT` | `/api/v1/hrm/user_org_posts/{id}` | 修改任职关系，必须携带 `version`。 |
| `DELETE` | `/api/v1/hrm/user_org_posts/{id}?version={version}` | 逻辑删除任职关系，必须通过 query 参数携带 `version`。 |
| `DELETE` | `/api/v1/hrm/user_org_posts/{id}/physical` | 物理删除任职关系。 |
| `GET` | `/api/v1/hrm/user_org_posts/{id}` | 根据 ID 查询任职关系。 |
| `GET` | `/api/v1/hrm/user_org_posts` | 分页查询任职关系。 |

关系物理删除必须满足关系已经逻辑删除。首版无需 HRM 内部引用检查，但必须预留跨模块引用检查扩展点；后续若其他模块引用关系 ID，物理删除必须先通过扩展点确认无有效引用，禁止绕过扩展点强删。

## 5. 分页查询与搜索规则

所有分页查询必须复用现有 `PageQuery`，进入 repository 前调用 `validate_and_normalize`，并遵守服务端 `pageSize <= 100`。

分页查询规则如下：

- 默认查询只返回 `deleted = false` 的数据。
- 所有字段都应支持条件查询；枚举、布尔、数字、ID、时间字段采用精确匹配或范围匹配。
- 字符串字段支持模糊匹配，必须使用参数绑定，禁止直接拼接用户输入。
- 名称搜索必须同时匹配 `name`、`name_full_pinyin`、`name_simple_pinyin`。
- 排序字段必须使用白名单映射，用户、组织和岗位列表默认按 `sort_no ASC, updated_time DESC, id ASC` 排序；组织树默认按 `sort_no ASC, created_time ASC` 排序。
- 查询条件中的 `tenantId` 默认取 `RequestContext.tenant_id` 或系统默认租户，普通调用方不得跨租户查询。

建议分页查询 DTO 按实体显式定义，避免引入通用动态查询对象。示例字段包括：

| 查询对象 | 字段示例 |
| --- | --- |
| 用户分页查询 | `employeeNo`、`name`、`mobile`、`email`、`sortNo`、`status`、`createdTimeStart`、`createdTimeEnd`、`updatedTimeStart`、`updatedTimeEnd`。 |
| 组织分页查询 | `parentId`、`orgCode`、`name`、`orgType`、`status`、`createdTimeStart`、`createdTimeEnd`。 |
| 岗位分页查询 | `postCode`、`name`、`status`、`createdTimeStart`、`createdTimeEnd`。 |
| 任职关系分页查询 | `userId`、`orgId`、`postId`、`primaryOrg`、`primaryPost`、`createdTimeStart`、`createdTimeEnd`。 |

## 6. 内部 Rust Service 能力

HRM 首版提供给其他后端模块使用的能力只通过内部 Rust Service 暴露，不提供跨模块写入 HTTP 接口。内部 Service 必须返回明确的业务结果，不得让调用方依赖数据库错误或空集合推断业务状态。

建议服务能力如下：

| 能力 | 说明 |
| --- | --- |
| `get_users_by_ids` | 按 ID 批量查询未删除用户，保留输入 ID 与结果缺失的可识别关系。 |
| `get_orgs_by_ids` | 按 ID 批量查询未删除组织。 |
| `get_posts_by_ids` | 按 ID 批量查询未删除岗位。 |
| `ensure_user_active` | 校验用户存在、未删除且为 `enabled`。 |
| `ensure_org_active` | 校验组织存在、未删除且为 `enabled`。 |
| `ensure_post_active` | 校验岗位存在、未删除且为 `enabled`。 |
| `get_org_tree` | 查询组织树，支持是否包含禁用节点。 |
| `get_user_assignments` | 查询用户组织岗位聚合信息，包含主组织和主岗位标记。 |
| `list_org_users` | 查询组织下用户，支持分页和是否包含下级组织的扩展参数。 |
| `list_candidate_users` | 查询可被其他模块选择的有效用户。 |
| `list_candidate_orgs` | 查询可被其他模块选择的有效组织。 |
| `list_candidate_posts` | 查询可被其他模块选择的有效岗位。 |

内部 Service 规则如下：

- 只提供读能力和存在性校验，写入统一走 HRM 管理 Service。
- 批量查询必须在 repository 层使用 `IN` 查询或等价批量查询，避免列表接口出现 N+1。
- 内部批量查询单次最多允许 500 个 ID，超过上限必须返回参数错误 `-400`，不得静默截断或自动忽略超出部分。
- 校验方法必须区分资源不存在、已逻辑删除和已禁用；对外可统一映射为业务规则失败，但日志中要保留具体原因。
- 返回 DTO 不得暴露数据库方言细节或 repository 内部结构。

## 7. 后端实现约束

后端模块建议放置在 `backend/src/modules/hrm/`，并遵守既有业务模块分层：

```text
backend/src/modules/hrm/
  mod.rs
  route.rs
  handler.rs
  service.rs
  repository.rs
  model.rs
  dto.rs
```

各层职责如下：

- `route.rs` 只声明 HTTP 方法、路径和 handler 绑定。
- `handler.rs` 只负责参数提取、显式校验、调用 service 和返回 `ApiResponse<T>`。
- `service.rs` 负责业务规则、事务编排、权限检查入口、拼音生成、状态校验和引用检查。
- `repository.rs` 负责 SQLx 查询和数据持久化，默认过滤 `deleted = false`。
- `model.rs` 定义数据库实体模型，所有基础字段必须平铺。
- `dto.rs` 定义请求和响应结构，接口字段使用 `camelCase`。

写入约定如下：

- 新增 SQL 必须写入 `version = 1`、`deleted = false`、创建字段、更新字段和拼音字段。
- 更新 SQL 必须包含 `WHERE id = ? AND tenant_id = ? AND version = ? AND deleted = false`，并显式递增 `version`。
- 逻辑删除 SQL 必须维护 `deleted`、`deleted_by`、`deleted_time` 和 `version`。
- 物理删除只能在 service 完成已删除状态和引用检查后执行。
- 唯一业务键冲突必须转换为并发冲突或业务规则错误，不得把数据库原始错误暴露给接口调用方。
- 组织创建和修改必须在 service 层校验组织类型、父组织类型、直接子组织类型和祖先链，避免形成非法父子层级或循环上下级。
- 涉及主组织、主岗位切换的写入必须使用事务，避免并发请求产生多个主关系。

数据库迁移约定如下：

- MySQL 与 PostgreSQL migration 必须同版本新增，例如 `V4__create_hrm_tables.sql`。
- 表结构、默认值、索引和唯一约束必须在 migration 中完整表达。
- MySQL 中逻辑删除唯一约束可使用生成列承载未删除业务键并建立唯一索引。
- PostgreSQL 中逻辑删除唯一约束优先使用 `WHERE deleted = FALSE` 的 partial unique index。
- 任职关系的主组织唯一性和主岗位唯一性必须通过数据库唯一约束兜底：MySQL 使用生成列表达未删除且主标记为 `true` 的唯一键，PostgreSQL 使用 `WHERE deleted = FALSE AND primary_org = TRUE`、`WHERE deleted = FALSE AND primary_post = TRUE` 的 partial unique index。
- 所有重要查询路径必须同步建立索引，不得等接口性能问题出现后再补索引。

## 8. 错误处理与权限边界

HRM 接口错误必须使用统一业务码：

| 业务码 | 场景 |
| --- | --- |
| `-400` | 必填字段缺失、字段格式错误、分页参数越界、状态枚举非法。 |
| `-403` | 当前用户无 HRM 管理权限或跨租户访问。 |
| `-404` | 查询、更新或删除的资源不存在，或已被逻辑删除。 |
| `-409` | 乐观锁版本不匹配、租户内编码冲突、重复任职关系。 |
| `-422` | 禁用对象被新增引用、删除存在引用的数据、组织删除仍有子节点。 |
| `-500` | 未预期系统错误或数据库依赖异常。 |

权限边界如下：

- HRM 管理端写接口必须预留 service 层权限检查入口。
- 首版鉴权未完整落地时，可先基于 `RequestContext` 预留检查函数，但不得把权限判断散落在 handler 或 repository。
- 跨租户访问必须禁止；tenant 解析规则必须集中在 service 或上下文转换层。
- 审计日志后续应记录 HRM 写操作的操作者、租户、对象类型、对象 ID、操作类型、结果和 `traceId`。

## 9. 性能与容量要求

HRM 是基础主数据模块，后续会被多个业务模块频繁读取，必须前置考虑性能容量。

性能约束如下：

- 分页接口必须限制最大 `pageSize`，禁止一次性返回全部用户。
- 内部批量查询必须限制最大 ID 数量，首版单次最多 500 个 ID，超过上限返回参数错误 `-400`，调用方需要自行分批。
- 组织树查询应控制返回字段，避免把用户列表嵌入组织树导致响应体过大。
- 用户任职聚合查询必须批量加载组织和岗位，禁止按用户逐条查询组织岗位。
- 高频候选查询可以后续增加缓存，但首版必须保证 SQL 索引覆盖主要过滤条件。

容量预期应在实现时至少按以下规模验证：

| 数据 | 首版验证规模 |
| --- | --- |
| 用户 | 10 万级。 |
| 组织 | 1 万级。 |
| 岗位 | 1 万级。 |
| 用户组织岗位关系 | 30 万级。 |

## 10. 测试与验收要求

### 10.1 Migration 契约测试

- MySQL 和 PostgreSQL HRM migration 同版本存在，文件名和业务意图一致。
- `hrm_users`、`hrm_orgs`、`hrm_posts`、`hrm_user_org_posts` 表存在。
- 所有表包含统一基础字段和 `tenant_id`。
- 所有带名称的表包含 `name_full_pinyin` 和 `name_simple_pinyin`。
- 未删除数据唯一约束符合租户内唯一要求。
- 逻辑删除后允许同租户复用 `employee_no`、`org_code`、`post_code`。
- 任职关系主组织和主岗位唯一约束能在数据库层阻止同一用户出现多个有效主关系。
- 主要分页、树查询、候选查询和关系查询索引存在。

### 10.2 Repository 测试

- 新增用户、组织、岗位时正确写入基础字段、拼音字段、排序号和默认版本。
- 修改名称时重新生成拼音字段，并校验 `version` 后递增版本。
- 创建组织时只允许 `branch` 和 `department` 两种 `orgType`，并正确校验父子组织类型。
- 修改组织父级时能阻止把组织移动到自身或自身后代下。
- 修改组织类型时能阻止把仍存在分部子节点的组织改为部门。
- 逻辑删除后默认查询不可见。
- 物理删除只允许已逻辑删除且无有效引用的数据。
- 分页查询支持精确条件、字符串模糊匹配、名称全拼和简拼匹配。
- 任职关系写入能防止重复关系和多个主组织、主岗位。

### 10.3 Service 测试

- 新增关系时，禁用用户、禁用组织或禁用岗位都会返回业务规则失败。
- 用户支持多个组织和多个岗位，并能正确标记主组织和主岗位。
- 分部下允许创建分部和部门，部门下只允许创建部门。
- 根节点只能创建分部，创建无父级部门必须返回业务规则失败。
- 创建或调整组织父级时能够检测并拒绝循环上下级。
- 内部批量查询超过 500 个 ID 时返回参数错误 `-400`。
- 删除存在子组织或有效任职关系的组织会返回业务规则失败。
- 批量查询和用户任职聚合查询不会产生 N+1 查询。
- 内部校验方法能区分不存在、已删除和已禁用数据。

### 10.4 API 测试

- CRUD 接口统一返回 `ApiResponse<T>`，HTTP 标准业务响应状态码保持 `200`。
- 响应字段使用 `camelCase`，路径使用 `snake_case`。
- 分页响应使用 `PageResult<T>`，并正确返回 `records`、`pageNo`、`pageSize`、`total`、`totalPages`、`hasNext`。
- 并发修改版本不匹配时返回 `-409`。
- 删除存在引用的数据返回 `-422`。
- 非法分页参数返回 `-400`。

## 11. 首版默认假设

- HRM 首版是主数据模块，不实现角色；原需求中的角色调整到后续权限模块处理。
- HRM 表显式包含 `tenant_id`，唯一约束按租户内未删除数据生效。
- 用户与组织、岗位是多对多关系，关系表支持一个主组织和一个主岗位关系。
- 物理删除不会级联强删有效引用数据。
- 组织类型首版只有分部和部门，必须强制校验父子层级合法性。
- 组织根节点只能是分部。
- 内部批量查询单次最多 500 个 ID，超过上限返回 `-400`。
- 禁用数据保留历史关系，但不能作为新增关系候选。
