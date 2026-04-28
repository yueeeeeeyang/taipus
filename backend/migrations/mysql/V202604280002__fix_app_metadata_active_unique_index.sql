-- 修正 app_metadata 的逻辑删除唯一约束。
-- 旧索引 (metadata_key, deleted) 会让同一个 key 的多条已删除记录互相冲突，因此改为只约束未删除数据。
ALTER TABLE app_metadata
    DROP INDEX uk_app_metadata_key_deleted,
    ADD COLUMN active_metadata_key VARCHAR(128)
        GENERATED ALWAYS AS (CASE WHEN deleted = FALSE THEN metadata_key ELSE NULL END) STORED,
    ADD UNIQUE KEY uk_app_metadata_active_key (active_metadata_key);
