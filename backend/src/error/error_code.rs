//! 稳定数字业务码定义。
//!
//! 业务 API 使用 HTTP 200 承载标准响应，调用方必须读取 `code` 判断业务状态。
//! 已开放的错误码不得随意变更语义，避免破坏前端、移动端或外部调用方兼容性。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    /// 成功业务码，业务 API 仍通过 HTTP 200 承载响应。
    Success = 200,
    /// 参数错误，例如必填字段缺失、格式错误、分页越界。
    ParamInvalid = -400,
    /// 未认证，例如缺少令牌、令牌无效或登录过期。
    Unauthorized = -401,
    /// 登录账号或密码错误。
    ///
    /// 该错误码只用于用户名密码登录失败；访问令牌缺失、过期、篡改等仍使用 `Unauthorized`，
    /// 便于前端区分“登录表单输入错误”和“当前会话不可用”。
    AuthBadCredentials = -1001,
    /// 已认证但无操作权限。
    Forbidden = -403,
    /// 资源不存在或已被逻辑删除。
    ResourceNotFound = -404,
    /// 并发冲突，例如乐观锁版本不匹配或重复提交。
    Conflict = -409,
    /// 业务规则失败，例如状态流转非法。
    BusinessError = -422,
    /// 请求过于频繁，预留给后续限流能力。
    RateLimited = -429,
    /// 系统错误，例如数据库不可用、未预期异常或外部依赖失败。
    SystemError = -500,
}

impl ErrorCode {
    /// 将枚举转换为对外稳定数字业务码。
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// 返回默认安全提示语。
    ///
    /// 这些消息可以直接进入接口响应，不包含任何内部实现细节。
    pub fn default_message(self) -> &'static str {
        match self {
            Self::Success => "ok",
            Self::ParamInvalid => "请求参数不合法",
            Self::Unauthorized => "未认证或登录已过期",
            Self::AuthBadCredentials => "账号或密码错误",
            Self::Forbidden => "无权限执行该操作",
            Self::ResourceNotFound => "资源不存在",
            Self::Conflict => "数据已被修改，请刷新后重试",
            Self::BusinessError => "业务规则校验失败",
            Self::RateLimited => "请求过于频繁",
            Self::SystemError => "系统错误，请稍后重试",
        }
    }

    /// 返回系统多语言资源 key。
    ///
    /// 错误码和 message key 分开维护，保证数字业务码稳定的同时允许响应消息按 locale 渲染。
    pub fn message_key(self) -> &'static str {
        match self {
            Self::Success => "error.success",
            Self::ParamInvalid => "error.param_invalid",
            Self::Unauthorized => "error.unauthorized",
            Self::AuthBadCredentials => "error.auth_bad_credentials",
            Self::Forbidden => "error.forbidden",
            Self::ResourceNotFound => "error.resource_not_found",
            Self::Conflict => "error.conflict",
            Self::BusinessError => "error.business_error",
            Self::RateLimited => "error.rate_limited",
            Self::SystemError => "error.system_error",
        }
    }

    /// 判断业务码是否表示成功。
    ///
    /// 当前约定成功码为正数，错误码为负数。
    pub fn is_success(self) -> bool {
        self.as_i32() > 0
    }
}
