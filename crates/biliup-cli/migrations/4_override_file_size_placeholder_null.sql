-- 历史版本把主播覆写（ConfigPatch）整份序列化落库，未设置的字段也写成 null。
-- file_size 是唯一把 null 解释为“显式清除”的字段，于是任何保存过覆写的主播都会
-- 把全局 file_size 清掉：边录边传退回约 2 GiB 默认分段，stream-gears 不再按大小分段。
--
-- 新版本序列化时会略过未设置的 file_size，占位 null 不再产生。这里清理既有数据：
-- 旧数据里的 file_size: null 无法区分占位与显式清除，统一视为未设置（跟随全局）；
-- 确实要按主播关闭大小分段的用户，在覆写里重新清空该字段即可。
UPDATE livestreamers
SET override = json_remove(override, '$.file_size')
WHERE override IS NOT NULL
  AND json_valid(override)
  AND json_type(override, '$.file_size') = 'null';
