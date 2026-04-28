# 多语言支持方案设计说明

## 1. 文档目标

本文档用于约定低代码平台的多语言支持方案，重点解决系统文本和业务数据文本的统一管理、后端接入、前端消费、语言协商、缓存失效和测试验收问题。后续实现多语言模块时，必须优先遵守本文档，除非产品范围、部署架构或性能容量目标发生变化并重新评审。

多语言模块的核心目标如下：

- **后端统一管理**：后端作为系统多语言资源和业务数据多语言资源的唯一正式来源，前端和移动端只消费后端提供的资源、语言上下文和翻译结果。
- **便捷接入**：后端业务模块通过统一 `I18nService`、`LocaleResolver`、错误消息 key 和业务翻译模型接入多语言能力，避免各模块重复解析语言和拼装文案。
- **边界清晰**：系统多语言和业务数据多语言必须分别建模、分别存储、分别缓存，避免把用户录入内容混入系统文案资源。
- **兼容前端**：前端和移动端可以继续使用成熟 i18n 客户端库，但正式翻译资源必须来自后端分发接口。
- **可观测与可维护**：缺失翻译、fallback 命中、资源版本、请求语言和最终语言必须可记录、可排查、可测试。

## 2. 术语与分类

多语言信息分为两类：系统多语言和业务数据多语言。两类文本的来源、生命周期、权限边界和缓存策略不同，必须分开处理。

| 分类 | 定义 | 示例 | 管理方式 |
| --- | --- | --- | --- |
| 系统多语言 | 由平台代码、产品配置或发布流程维护的固定文本。 | 后端错误信息、校验提示、操作提示、菜单、按钮、表单标签、前端固定显示文本。 | 使用系统资源文件和后端资源分发接口统一管理。 |
| 业务数据多语言 | 由用户录入、业务流程产生或低代码配置产生的业务内容。 | 表单名称、字段显示名、流程节点名称、业务字典值、用户维护的描述文本。 | 使用数据库业务翻译表统一存储，并按业务权限读写。 |

系统多语言强调发布一致性和资源版本管理。业务数据多语言强调数据归属、权限控制、乐观锁、审计和查询性能。

## 3. 技术选型

后端系统多语言建议首版使用 `rust-i18n`。该库适合 Rust 服务端按 key 和 locale 渲染短文本，可以把错误信息、校验提示和操作提示集中为资源文件，降低业务模块接入成本。

前端和移动端建议使用 `i18next` 与 `react-i18next`。前端运行时从后端系统资源接口拉取资源，并加载到 i18next resource store 中。前端可以保留启动兜底文案，但不得把本地文件作为正式翻译源。

复杂复数、性别、富文本占位或 ICU 风格消息在首版不强制引入。若后续系统文本复杂度明显上升，可评估 `fluent-bundle` 或其他 Fluent 生态能力，但必须保持后端统一管理和资源分发接口不变。

## 4. 语言标识与协商

语言标识统一使用 BCP 47 风格，例如 `zh-CN`、`en-US`。默认语言必须通过配置文件指定，例如 `I18N_DEFAULT_LOCALE=zh-CN` 或等价配置项，不允许在业务代码或接口逻辑中写死默认语言。首批建议支持 `zh-CN` 和 `en-US`。

请求语言选择优先级固定如下：

1. 显式请求参数，例如 `locale=zh-CN`。
2. 请求头 `X-Locale`。
3. 当前登录用户的语言偏好。
4. 请求头 `Accept-Language`。
5. 配置文件指定的系统默认语言。

语言协商规则如下：

- `LocaleResolver` 负责解析、规范化、校验和 fallback，不允许 handler 或 service 自行解析请求头。
- 不支持的 locale 必须 fallback 到配置文件指定的默认语言，并记录缺失或降级事件。
- 响应头必须回写最终采用的语言，例如 `Content-Language: zh-CN`。
- `RequestContext` 后续需要增加 `locale` 和 `requestedLocale`，分别表示最终语言和调用方原始期望语言。
- 缓存 key 必须包含最终 locale，避免不同语言响应互相污染。

## 5. 系统多语言方案

系统多语言覆盖后端和前端固定文本，包括：

- 后端错误信息、参数校验信息、操作成功或失败提示。
- 前端和移动端固定 UI 文本，例如菜单、按钮、表单标签、空状态、确认弹窗。
- 平台内置配置文本，例如系统字典的内置显示名、默认页面标题和默认流程提示。

系统资源按 `rust-i18n` 支持的 `_version: 1` 资源文件维护，每种语言一个独立 yml，资源 key 使用 namespace 前缀区分系统域：

```text
backend/
  locales/
    zh-CN.yml
    en-US.yml
```

namespace 约定如下：

| namespace | 说明 |
| --- | --- |
| `backend` | 后端错误、校验、日志可见提示和接口提示。 |
| `common` | 前端和移动端通用按钮、状态、操作提示。 |
| `menu` | 菜单、导航和模块标题。 |
| `form` | 表单内置组件、校验提示和默认字段标题。 |

系统文本 key 必须稳定，不得直接以中文文本作为 key。推荐命名示例：

```text
error.param_invalid
error.resource_not_found
common.confirm
menu.system_settings
form.validation.required
```

后端错误处理需要从固定 message 文本演进为 `messageKey + messageArgs`。`ErrorCode` 仍保持数字业务码稳定，`AppError` 负责携带 `messageKey`、安全默认消息和参数，最终由 `I18nService` 按请求 locale 渲染响应 `message`。

当前后端配置项如下：

| 配置项 | 必填 | 说明 |
| --- | --- | --- |
| `I18N_DEFAULT_LOCALE` | 是 | 配置默认语言，业务代码不得写死默认 locale。 |
| `I18N_SUPPORTED_LOCALES` | 否 | 逗号分隔的支持语言列表；未配置时只启用默认语言。 |
| `I18N_SYSTEM_RESOURCE_VERSION` | 否 | 系统资源版本；未配置时使用当前后端内置版本。 |

## 6. 业务数据多语言方案

业务数据多语言覆盖用户录入和业务产生的数据，包括：

- 用户配置的页面名称、表单名称、字段显示名、字段说明。
- 业务字典、分类、标签、流程节点、审批动作名称。
- 业务运行时生成并需要展示给用户的可翻译文本。

业务翻译建议使用统一数据库表存储，不与系统资源文件混放。核心定位维度如下：

| 字段 | 说明 |
| --- | --- |
| `resource_type` | 业务资源类型，例如 `form_definition`、`field_definition`、`dictionary_item`。 |
| `resource_id` | 业务资源主键。 |
| `field_name` | 被翻译字段，例如 `name`、`description`、`display_name`。 |
| `locale` | 翻译语言，例如 `zh-CN`、`en-US`。 |
| `text_value` | 翻译后的文本值。 |
| `version` | 乐观锁版本号。 |
| `deleted` | 逻辑删除标记。 |

业务翻译表必须包含统一基础字段：`created_by`、`created_time`、`updated_by`、`updated_time`、`deleted_by`、`deleted_time`。MySQL 和 PostgreSQL migration 必须保持同版本脚本，并对未删除数据建立唯一约束：`resource_type + resource_id + field_name + locale`。

业务数据读取规则如下：

- 普通查询默认按当前 locale 返回最佳匹配文本。
- 如果目标 locale 缺失，按 fallback 链路返回配置默认语言文本，并标记 `translationMissing = true`。
- 管理端详情可以返回 `translations`，展示所有语言版本，方便维护翻译。
- 写入业务翻译必须校验 locale、资源存在性、字段白名单、权限边界和乐观锁版本。
- 逻辑删除业务资源时，必须同步逻辑删除对应业务翻译，或通过查询层保证翻译不再被读取。

## 7. 后端接入方式

后端多语言模块建议提供以下核心类型：

| 类型 | 职责 |
| --- | --- |
| `Locale` | 表示规范化后的语言标识，负责校验支持范围。 |
| `LocaleResolver` | 从请求参数、`X-Locale`、用户偏好和 `Accept-Language` 中解析最终语言。 |
| `I18nService` | 渲染系统文本、加载系统资源、查询业务翻译和执行 fallback。 |
| `SystemMessageKey` | 系统文本 key 的稳定枚举或常量集合。 |
| `BusinessTranslation` | 业务翻译持久化模型。 |
| `LocalizedText<T>` | 表示带翻译状态的数据包装，例如当前值、目标语言、来源语言和是否缺失。 |

后端接入规则如下：

- handler 不直接读取 `Accept-Language` 或 `X-Locale`，统一从 `RequestContext` 获取 locale。
- service 层负责决定哪些业务字段需要多语言，并调用 `I18nService` 或 repository 查询翻译。
- repository 层负责按资源维度批量查询业务翻译，避免列表接口出现 N+1 查询。
- `AppError` 后续必须携带 `messageKey`，由统一错误响应转换层按 locale 渲染。
- 后端日志和审计记录必须包含最终 locale，便于定位语言协商和翻译缺失问题。

## 8. 接口契约

### 8.1 系统资源接口

系统资源接口用于前端和移动端拉取固定文本资源。

```text
GET /api/v1/i18n/system_resources?locale=zh-CN&platform=frontend&namespaces=common,menu
```

标准响应示例：

```json
{
  "code": 200,
  "message": "ok",
  "data": {
    "locale": "zh-CN",
    "fallbackLocales": ["zh-CN"],
    "version": "202604280001",
    "resources": {
      "common": {
        "confirm": "确认",
        "cancel": "取消"
      },
      "menu": {
        "systemSettings": "系统设置"
      }
    }
  },
  "traceId": "trace-id",
  "timestamp": "2026-04-28T00:00:00Z"
}
```

接口规则如下：

- `platform` 用于区分 `frontend`、`mobile`、`backend` 等消费端。
- `namespaces` 为空时返回该 platform 的必要基础资源，不默认返回所有资源。
- `version` 用于客户端缓存和灰度发布，资源更新后必须变化。
- 未支持的 locale 必须返回 fallback 后的资源，并在 `fallbackLocales` 中体现。

### 8.2 业务数据查询接口

普通业务查询默认返回当前 locale 的最佳匹配字段：

```json
{
  "id": "form_001",
  "name": "客户登记",
  "nameLocale": "zh-CN",
  "translationMissing": false
}
```

管理端详情可以返回所有语言版本：

```json
{
  "id": "form_001",
  "name": "客户登记",
  "translations": {
    "name": {
      "zh-CN": "客户登记",
      "en-US": "Customer Registration"
    }
  }
}
```

业务数据接口规则如下：

- 普通查询不应默认返回所有翻译，避免响应体过大。
- 管理端或编辑接口可以显式返回 `translations`。
- 批量列表必须批量加载翻译，禁止每条记录单独查询翻译。
- 写入接口必须使用字段白名单，禁止调用方翻译任意数据库字段。

## 9. 前端和移动端接入

前端和移动端的正式翻译资源来自后端系统资源接口。推荐接入方式如下：

- 应用启动时读取用户语言偏好或浏览器语言，并请求系统资源接口。
- 将接口返回的 `resources` 注入 `i18next/react-i18next`。
- 语言切换时重新请求后端资源，并刷新当前页面依赖的业务数据。
- 前端本地只保留启动失败、网络不可用等极少量兜底文案，不维护完整正式翻译源。
- 前端提交业务数据翻译时，必须按后端字段白名单和 locale 规则提交，不自行发明翻译结构。

前端缓存必须绑定 `locale + platform + namespaces + version`。当后端资源版本变化时，客户端必须丢弃旧资源并重新加载。

## 10. 缓存、权限与审计

系统多语言资源可以按 locale、platform、namespace 和 version 做内存缓存。资源文件变更后，必须通过应用重启、版本号更新或后续管理端发布流程触发缓存失效。

业务数据多语言缓存必须谨慎使用。列表查询可以使用请求级缓存或短时缓存，但写入、逻辑删除和发布操作必须能及时失效相关翻译。

权限与审计规则如下：

- 系统多语言资源维护属于平台配置权限，不能开放给普通业务用户。
- 业务数据翻译维护必须遵守原业务资源的读写权限。
- 翻译写入、修改、删除必须记录操作者、资源定位、locale、字段名、旧值摘要、新值摘要和 `traceId`。
- 审计日志不得记录敏感字段的明文翻译值，敏感字段默认不允许进入多语言翻译表。

## 11. 测试与验收要求

后续实现多语言模块时，必须至少覆盖以下场景：

- `X-Locale`、用户偏好、`Accept-Language` 和默认语言优先级正确。
- 不支持的 locale 能 fallback 到配置默认语言，并回写正确 `Content-Language`。
- 后端错误信息能按 locale 输出，数字 `code` 保持不变。
- 系统资源接口按 platform、namespace、locale 和 version 返回稳定结构。
- 前端固定文本资源来自后端接口，不依赖独立正式翻译源。
- 业务数据缺失目标语言时按 fallback 返回，并标记翻译缺失。
- 业务翻译写入校验 locale、字段白名单、权限和乐观锁。
- 列表接口批量加载翻译，不出现 N+1 查询。
- MySQL 与 PostgreSQL migration 对业务翻译表唯一约束保持兼容。
- 多语言资源缓存能按 version 或写入事件失效。

## 12. 后续实现顺序建议

建议按以下顺序落地多语言模块：

1. 增加 locale 配置、`Locale`、`LocaleResolver` 和 `RequestContext` locale 字段。
2. 接入 `rust-i18n`，先完成后端错误信息和校验信息多语言。
3. 实现系统资源分发接口，供前端和移动端接入 `i18next/react-i18next`。
4. 设计并创建业务翻译表 migration，保持 MySQL 与 PostgreSQL 同版本兼容。
5. 在一个低代码配置模块中验证业务数据多语言读写、fallback 和批量查询。
6. 补齐缓存、审计、权限和翻译缺失监控。

## 13. 参考库

后续实现优先评估以下成熟库：

- 后端系统多语言：`rust-i18n`，参考 https://docs.rs/rust-i18n。
- 前端和移动端资源消费：`i18next` 与 `react-i18next`，参考 https://www.i18next.com/。
- 复杂消息格式候选：`fluent-bundle`，参考 https://docs.rs/fluent-bundle。
