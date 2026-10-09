-- Fleet 房间同样保存 ConfigPatch：升级前的 segment_time:null 是继承全局的占位值。
-- 新版本 null 用于显式关闭时长分段，先清理旧占位值，避免下发后清掉节点全局时长。
UPDATE fleet_rooms
SET override = json_remove(override, '$.segment_time')
WHERE override IS NOT NULL
  AND json_valid(override)
  AND json_type(override, '$.segment_time') = 'null';
