"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

const Douyin: React.FC = () => {
    return (
        <>
            <Collapse.Panel header="抖音" itemKey="douyin">
                <Form.Select
                    field="douyin_quality"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            抖音自选画质，没有选中的画质则会自动选择相近的画质优先低清晰度。
                            <br />
                            刚开播可能没有除了原画之外的画质，会先录制原画。当使用
                            ffmpeg 或 streamlink
                            时，后续视频分段将会录制设置的画质。
                        </div>
                    }
                    label="画质等级（douyin_quality）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                >
                    <Select.Option value="origin">原画（origin）</Select.Option>
                    <Select.Option value="uhd">蓝光（uhd）</Select.Option>
                    <Select.Option value="hd">超清（hd）</Select.Option>
                    <Select.Option value="sd">高清（sd）</Select.Option>
                    <Select.Option value="ld">标清（ld）</Select.Option>
                    <Select.Option value="md">流畅（md）</Select.Option>
                </Form.Select>
                <Form.Switch
                    field="douyin_danmaku"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            录制抖音弹幕，默认关闭。
                            <br />
                            暂不支持视频按时长分段下的弹幕文件自动分段；仅在使用ffmpeg（包括streamlink混合模式）时得到支持。
                        </div>
                    }
                    label="录制弹幕（douyin_danmaku）"
                />
            </Collapse.Panel>
        </>
    );
};

export default Douyin;
