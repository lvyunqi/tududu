-- 模型渠道商：官方直连或各类中转服务
CREATE TABLE providers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,
    protocol        TEXT NOT NULL DEFAULT 'openai_images',
    base_url        TEXT NOT NULL,
    auth_type       TEXT NOT NULL DEFAULT 'bearer',
    headers         JSONB NOT NULL DEFAULT '{}'::jsonb,
    timeout_ms      INT NOT NULL DEFAULT 180000,
    proxy_url       TEXT,
    max_concurrency INT NOT NULL DEFAULT 8,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE providers IS '模型渠道商';
COMMENT ON COLUMN providers.protocol IS 'openai_images 等，决定请求与响应的解析方式';

-- 渠道接入点，一个 provider 下可以挂多条密钥
CREATE TABLE channels (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id     UUID NOT NULL REFERENCES providers (id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    api_key_cipher  TEXT NOT NULL,
    api_key_hint    TEXT NOT NULL,
    weight          INT NOT NULL DEFAULT 100,
    priority        SMALLINT NOT NULL DEFAULT 1,
    max_concurrency INT NOT NULL DEFAULT 4,
    health          SMALLINT NOT NULL DEFAULT 1,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    last_ok_at      TIMESTAMPTZ,
    last_fail_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN channels.api_key_cipher IS 'AES-256-GCM 密文，nonce 拼在密文前面';
COMMENT ON COLUMN channels.api_key_hint IS '只用于界面展示的掩码，例如 sk-…a1b2';
COMMENT ON COLUMN channels.health IS '1 正常 / 2 降级 / 3 熔断 / 4 停用';

CREATE INDEX idx_channels_provider ON channels (provider_id);

-- 从上游拉回来的模型清单
CREATE TABLE models (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id   UUID NOT NULL REFERENCES providers (id) ON DELETE CASCADE,
    code          TEXT NOT NULL,
    display_name  TEXT NOT NULL,
    capabilities  JSONB NOT NULL DEFAULT '{}'::jsonb,
    enabled       BOOLEAN NOT NULL DEFAULT TRUE,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider_id, code)
);

COMMENT ON TABLE models IS '渠道商侧可用的模型，由在线拉取或手工添加';
