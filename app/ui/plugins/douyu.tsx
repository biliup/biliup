"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

const Douyu: React.FC = () => {
    return (
        <>
            <Collapse.Panel header="斗鱼" itemKey="douyu">
                <Form.InputNumber
                    field="douyu_rate"
                    extraText="刚开播可能没有除了原画之外的画质 会先录制原画 后续视频分段(仅ffmpeg streamlink)时录制设置的画质
0 原画,4 蓝光4m,3 超清,2 高清"
                    label="画质等级（douyu_rate）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="douyu_danmaku"
                    extraText="录制斗鱼弹幕，默认关闭【目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持】"
                    label="录制弹幕（douyu_danmaku）"
                />
                <Form.Input
                    field="douyucdn"
                    extraText="如遇到斗鱼录制卡顿可以尝试切换线路。可选以下线路
tctc-h5（备用线路4）, tct-h5（备用线路5）, ali-h5（备用线路6）, hw-h5（备用线路7）, hs-h5（备用线路13）"
                    label="访问线路（douyucdn）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
            </Collapse.Panel>
        </>
    );
};

export default Douyu;
