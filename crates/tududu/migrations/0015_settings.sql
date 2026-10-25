-- 可在后台在线修改的配置。启动期参数（数据库、密钥、监听）不在这里。
CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE settings IS '后台可改的运行时配置，改了即时生效，不需要重启';
