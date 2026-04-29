//! HRM 人力资源主数据模块。
//!
//! 首版只维护用户、组织、岗位和用户组织岗位关系，不实现登录、角色或权限授权能力。

pub mod dto;
pub mod handler;
pub mod model;
pub mod repository;
pub mod route;
pub mod service;
