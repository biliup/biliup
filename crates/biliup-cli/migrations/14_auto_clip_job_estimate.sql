-- 自动切片任务入队时算的用量预估（JSON，与 `POST /v1/sessions/{id}/auto-clip` 回的 `estimate` 同形）。
-- 状态接口直接回这份，不再每次读分段、静音表和弹幕重算；这一列之前建的任务为 NULL。
ALTER TABLE auto_clip_jobs ADD COLUMN estimate TEXT;
