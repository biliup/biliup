-- 历史 ConfigPatch 会把未设置的 segment_time 写成 null，旧语义下这些值继承全局。
-- 新版本允许显式 null 按主播关闭时长分段，并省略未设置的 segment_time。
-- 升级前的 null 只能是旧语义的占位值，删除它们以继续继承全局。
UPDATE livestreamers
SET override = json_remove(override, '$.segment_time')
WHERE override IS NOT NULL
  AND json_valid(override)
  AND json_type(override, '$.segment_time') = 'null';
