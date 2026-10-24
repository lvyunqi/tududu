-- 刷新令牌按摘要查找，必须唯一且走索引
CREATE UNIQUE INDEX idx_user_sessions_refresh ON user_sessions (refresh_hash);
