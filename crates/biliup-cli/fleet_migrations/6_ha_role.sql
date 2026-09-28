-- Fleet H2：一主一备的上传主机可以切换。primary_node_id / standby_node_id 仍是两台机器本身
-- （控制面的「本机」节点、被指定进配对的普通节点），leader_node_id 是此刻负责上传的一台：
-- 为空表示「本机」节点（H1 的默认），等于 standby_node_id 表示那台普通节点。换备机时清空。
ALTER TABLE ha_pair ADD COLUMN leader_node_id INTEGER REFERENCES fleet_nodes (id);
