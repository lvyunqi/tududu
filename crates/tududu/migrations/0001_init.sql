-- 用户主表
CREATE TABLE users (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    short_code       TEXT NOT NULL UNIQUE,
    nickname         TEXT NOT NULL,
    avatar_path      TEXT,
    phone            TEXT UNIQUE,
    email            TEXT UNIQUE,
    password_hash    TEXT,
    status           SMALLINT NOT NULL DEFAULT 1,
    register_channel TEXT NOT NULL DEFAULT 'app',
    register_ip      TEXT,
    risk_flag        SMALLINT NOT NULL DEFAULT 0,
    last_login_at    TIMESTAMPTZ,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE users IS '用户账号';
COMMENT ON COLUMN users.short_code IS '对外展示的短编号，避免暴露内部 UUID';
COMMENT ON COLUMN users.avatar_path IS '头像在本地数据目录下的相对路径';
COMMENT ON COLUMN users.status IS '1 正常 / 2 禁言 / 3 封禁 / 4 已注销';
COMMENT ON COLUMN users.risk_flag IS '0 正常 / 1 观察 / 2 高风险';

CREATE INDEX idx_users_status_created ON users (status, created_at DESC);

-- 第三方身份绑定
CREATE TABLE user_identities (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    provider   TEXT NOT NULL,
    open_id    TEXT NOT NULL,
    union_id   TEXT,
    credential JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, open_id)
);

COMMENT ON TABLE user_identities IS '用户的多平台身份绑定';
COMMENT ON COLUMN user_identities.provider IS 'phone / email / wechat 等';

CREATE INDEX idx_user_identities_user ON user_identities (user_id);

-- 登录会话，一条记录对应一台设备上的一次长期登录
CREATE TABLE user_sessions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    device_id    TEXT NOT NULL,
    device_name  TEXT,
    platform     TEXT,
    app_version  TEXT,
    refresh_hash TEXT NOT NULL,
    ip           TEXT,
    user_agent   TEXT,
    expires_at   TIMESTAMPTZ NOT NULL,
    revoked_at   TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE user_sessions IS '设备登录会话，用于刷新令牌与踢下线';
COMMENT ON COLUMN user_sessions.refresh_hash IS '刷新令牌的摘要，不存明文';

CREATE INDEX idx_user_sessions_user ON user_sessions (user_id, created_at DESC);
CREATE INDEX idx_user_sessions_active ON user_sessions (expires_at) WHERE revoked_at IS NULL;
