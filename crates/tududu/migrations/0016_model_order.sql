-- 模型的后台配置：排序、备注
ALTER TABLE models ADD COLUMN sort_order INT NOT NULL DEFAULT 100;
ALTER TABLE models ADD COLUMN remark TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN models.sort_order IS '用户端展示顺序，小的在前';

CREATE INDEX idx_models_order ON models (sort_order, code);
