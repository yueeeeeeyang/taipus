//! 租户模块。
//!
//! 首版租户模块只负责租户主数据和共享表隔离上下文，不承接账号、认证、套餐计费或分库分 schema。

pub mod dto;
pub mod handler;
pub mod model;
pub mod repository;
pub mod route;
pub mod service;
