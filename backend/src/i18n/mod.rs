//! 多语言模块入口。
//!
//! 本模块负责语言协商、系统资源分发、系统消息渲染和业务翻译模型定义。业务模块只应依赖这里
//! 暴露的稳定类型，避免自行解析请求头或直接读取多语言资源文件。

pub mod business_translation;
pub mod business_translation_registry;
pub mod business_translation_repository;
pub mod handler;
pub mod locale;
pub mod route;
pub mod service;
pub mod system_resource;
pub mod time_zone;
