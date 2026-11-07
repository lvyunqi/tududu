-- 预览图：点开看的那一版，比原图小很多，比缩略图清楚
ALTER TABLE assets ADD COLUMN preview_key TEXT;

COMMENT ON COLUMN assets.preview_key IS '相对数据目录的预览图路径，为空表示还没生成';
