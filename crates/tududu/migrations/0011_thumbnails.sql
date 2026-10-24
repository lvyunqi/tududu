-- 缩略图在原图旁边另存一份，列表用缩略图，点开才加载原图
ALTER TABLE assets ADD COLUMN thumb_key TEXT;

COMMENT ON COLUMN assets.thumb_key IS '相对数据目录的缩略图路径，为空表示还没有生成';
