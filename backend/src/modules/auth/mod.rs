//! 认证模块。
//!
//! 首版只负责账号、账号租户关系、JWT 双令牌、刷新令牌会话和认证审计；角色、权限点、
//! 菜单权限和数据权限由后续权限模块单独实现。

pub mod dto;
pub mod handler;
pub mod model;
pub mod password;
pub mod repository;
pub mod route;
pub mod service;
pub mod token;
