-- 广场分类
CREATE TABLE prompt_categories (
    id      BIGSERIAL PRIMARY KEY,
    name    TEXT NOT NULL,
    slug    TEXT NOT NULL UNIQUE,
    sort    INT NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT TRUE
);

INSERT INTO prompt_categories (name, slug, sort) VALUES
    ('人像', 'portrait', 10),
    ('风景', 'landscape', 20),
    ('电商', 'commerce', 30),
    ('修复', 'restore', 40),
    ('二次元', 'anime', 50),
    ('其他', 'other', 90);

-- 广场作品
CREATE TABLE prompts (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    short_code        TEXT NOT NULL UNIQUE,
    author_user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    title             TEXT NOT NULL,
    prompt_text       TEXT NOT NULL,
    model             TEXT NOT NULL,
    category_id       BIGINT REFERENCES prompt_categories (id) ON DELETE SET NULL,
    cover_asset_id    UUID REFERENCES assets (id) ON DELETE SET NULL,
    source_session_id UUID REFERENCES sessions (id) ON DELETE SET NULL,
    status            SMALLINT NOT NULL DEFAULT 2,
    like_count        INT NOT NULL DEFAULT 0,
    view_count        INT NOT NULL DEFAULT 0,
    published_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON COLUMN prompts.status IS '2 已发布 / 3 已下架';

CREATE INDEX idx_prompts_feed ON prompts (status, published_at DESC);
CREATE INDEX idx_prompts_category ON prompts (category_id, published_at DESC);
CREATE INDEX idx_prompts_author ON prompts (author_user_id, created_at DESC);

-- 点赞
CREATE TABLE prompt_likes (
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    prompt_id  UUID NOT NULL REFERENCES prompts (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, prompt_id)
);
