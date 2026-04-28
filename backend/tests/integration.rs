//! 后端基础底座集成测试入口。
//!
//! 通过显式路径拆分测试文件，保持响应契约和健康检查契约的测试边界清晰。

#[path = "integration/api_response_test.rs"]
mod api_response_test;

#[path = "integration/health_check_test.rs"]
mod health_check_test;

#[path = "integration/migration_test.rs"]
mod migration_test;
