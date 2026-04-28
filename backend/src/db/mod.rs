//! 数据库基础设施模块。
//!
//! 该模块封装 SQLx 连接池和事务边界，业务代码不得直接散落数据库方言判断。

pub mod entity;
pub mod executor;
pub mod repository;
pub mod transaction;
