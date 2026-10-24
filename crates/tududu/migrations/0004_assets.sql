-- 图片资产。图片本身落在本地磁盘，这里只存元信息与相对路径
CREATE TABLE assets (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    object_key    TEXT NOT NULL,
    mime          TEXT NOT NULL,
    byte_size     BIGINT NOT NULL,
    sha256        TEXT NOT NULL,
    source        TEXT NOT NULL DEFAULT 'upload',
    status        SMALLINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMENT ON TABLE assets IS '用户图片资产，文件存在本地数据目录';
COMMENT ON COLUMN assets.object_key IS '相对数据目录的路径，例如 u/TU8K2M9QRS/2026/10/xxx.png';
COMMENT ON COLUMN assets.status IS '1 正常 / 2 回收站 / 3 已删除';
COMMENT ON COLUMN assets.source IS 'upload 上传 / job 生成';

CREATE INDEX idx_assets_owner_created ON assets (owner_user_id, created_at DESC);
CREATE INDEX idx_assets_owner_sha ON assets (owner_user_id, sha256);
