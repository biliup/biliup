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
                    showClear={true}
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
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.douyin_cookie"
                    extraText={
                        <div className="semi-form-field-extra">
                            如需要录制抖音 www.douyin.com/user/
                            类型链接，或遭到风控，请在此填入 Cookie。
                            <br />
                            需要__ac_nonce、__ac_signature、sessionid的值，请不要将所有
                            Cookie 填入。
                        </div>
                    }
                    placeholder="__ac_nonce=none;__ac_signature=none;sessionid=none;"
                    label="抖音 Cookie（douyin_cookie）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Select
                    field="douyin_protocol"
                    extraText="hls 仅供测试，请谨慎切换。"
                    label="直播流协议（douyin_protocol）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                    showClear={true}
                >
                    <Select.Option value="flv">flv（默认）</Select.Option>
                    <Select.Option value="hls">hls</Select.Option>
                </Form.Select>
            </Collapse.Panel>
        </>
    );
};

export default Douyin;
