-- 修正 app_metadata 的逻辑删除唯一约束。
-- PostgreSQL 使用 partial unique index 只约束未删除数据，允许同一 key 保留多条历史删除记录。
DROP INDEX uk_app_metadata_key_deleted;

CREATE UNIQUE INDEX uk_app_metadata_active_key
    ON app_metadata (metadata_key)
    WHERE deleted = FALSE;
