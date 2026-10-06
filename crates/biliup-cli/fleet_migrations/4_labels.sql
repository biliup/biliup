-- Fleet F4：节点标签约束。节点自己的标签在 F1 的 fleet_nodes.labels 里（JSON 字符串数组）。
-- required_labels 是房间要求的标签（JSON 字符串数组）：分配到的节点必须带齐这些标签，空数组表示不限。
-- 标签变了不自动迁移已分配的房间，只在房间列表里标出「标签不满足」。
ALTER TABLE fleet_rooms ADD COLUMN required_labels TEXT NOT NULL DEFAULT '[]';
