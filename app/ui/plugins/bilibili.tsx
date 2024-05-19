"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

const Bilibili: React.FC = () => {
    return (
        <>
            <Collapse.Panel header="哔哩哔哩" itemKey="bilibili">
                <Form.Select
                    field="bili_qn"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            哔哩哔哩自选画质。默认原画。
                            <br />
                            刚开播如果无选择的画质，会先录制原画，
                            后续视频分段时，如果下载插件为非 stream-gears，会切换到选择的画质。
                            <br />
                            如果选择的画质不提供，会选择更低一档的画质。
                        </div>
                    }
                    label="画质等级（bili_qn）"
                    placeholder="10000（原画）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                    showClear={true}
                >
                    <Select.Option value={30000}>30000（杜比）</Select.Option>
                    <Select.Option value={20000}>20000（4k）</Select.Option>
                    <Select.Option value={10000}>10000（原画）</Select.Option>
                    <Select.Option value={401}>401（蓝光-杜比）</Select.Option>
                    <Select.Option value={400}>400（蓝光）</Select.Option>
                    <Select.Option value={250}>250（超清）</Select.Option>
                    <Select.Option value={150}>150（高清）</Select.Option>
                    <Select.Option value={80}>80（流畅）</Select.Option>
                    <Select.Option value={0}>0（最低画质）</Select.Option>
                </Form.Select>
                <Form.Switch
                    field="bilibili_danmaku"
                    extraText="录制哔哩哔哩弹幕，目前不支持视频按时长分段下的弹幕文件自动分段。仅限下载插件为非 stream-gears 时生效，默认关闭。"
                    label="录制弹幕（bilibili_danmaku）"
                />
                <Form.Select
                    field="bili_protocol"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            哔哩哔哩直播流协议。
                            <br />
                            由于B站转码为 fmp4 需要一定时间，或者某些小主播（大部分只有原画选项的主播）无fmp4流时，
                            如果开播时间小于60s，将会反复尝试获取 fmp4 流，如果没获取到就回退到 flv 流。
                            <br />
                            由于 ffmpeg 不支持多并发，且 stream-gears 尚未支持 fmp4，推荐切换为 streamlink 来录制 hls 流。
                        </div>
                    }
                    label="直播流协议（bili_protocol）"
                    placeholder="stream（flv，默认）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                    showClear={true}
                >
                    <Select.Option value="stream">
                        stream（flv，默认）
                    </Select.Option>
                    <Select.Option value="hls_ts">hls_ts</Select.Option>
                    <Select.Option value="hls_fmp4">hls_fmp4</Select.Option>
                </Form.Select>
                <Form.Input
                    field="bili_perfCDN"
                    extraText="哔哩哔哩直播优选CDN，默认无。"
                    label="优选CDN（bili_perfCDN）"
                    placeholder="cn-gotcha208, ov-gotcha05"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="bili_cdn_fallback"
                    extraText="CDN 回退（Fallback），默认为关闭。例如海外机器优选 ov05 之后，如果 ov05 流一直无法下载，将会自动回退到 ov07 进行下载。仅限相同流协议。"
                    label="CDN 回退（bili_cdn_fallback）"
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="bili_liveapi"
                    extraText="自定义哔哩哔哩直播主要 API，用于获取指定区域（大陆或海外）的直播流链接，默认使用官方 API。"
                    label="哔哩哔哩直播主要API（bili_liveapi）"
                    style={{ width: "100%" }}
                    placeholder="https://api.live.bilibili.com"
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="bili_fallback_api"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            上方的主要 API 不可用或受到区域限制时，回退使用的 API。默认使用官方 API。
                            <br />
                            海外机器玩法：哔哩哔哩直播API（bili_liveapi）设置为能获取大陆直播流的
                            API，并将哔哩哔哩直播回退API（bili_fallback_api）设置为官方
                            API，然后优选「fmp4」流并使用「streamlink」下载插件（downloader），
                            最后设置优选「cn-gotcha204,ov-gotcha05」两个节点。
                            <br />
                            这样大主播可以使用 cn204 的 fmp4 流稳定录制。
                        </div>
                    }
                    label="哔哩哔哩直播回退API（bili_fallback_api）"
                    style={{ width: "100%" }}
                    placeholder="https://api.live.bilibili.com"
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="bili_force_source"
                    extraText="哔哩哔哩强制真原画（仅限非 FLV 流，且画质等级 bili_qn >= 10000），默认为关闭，不保证可用性。"
                    label="强制获取真原画（bili_force_source）"
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="bili_normalize_cn204"
                    extraText="去除 cn-gotcha204 后面的小尾巴（-[1-4]）"
                    label="标准化 CN204（bili_normalize_cn204）"
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.TagInput
                    allowDuplicates={false}
                    addOnBlur={true}
                    separator=','
                    field="bili_replace_cn01"
                    extraText="该功能在 bili_force_source 后生效"
                    label="替换 CN01 sid (bili_replace_cn01)"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                    showClear={true}
                    placeholder="可用英文逗号分隔以批量输入 sid，失焦/Enter 保存"
                    onChange={v => console.log(v)}
                    rules={[
                        {
                            validator: (rule, value) => {
                                value = value ?? (console.log(value), []);
                                return Array.isArray(value) && value.every(item => /^cn-[a-z]{2,6}-[a-z]{2}(-[0-9]{2}){2}$/.test(item));
                            },
                            message: '例: cn-hjlheb-cu-01-01,cn-tj-ct-01-01'
                        }
                    ]}
                />
            </Collapse.Panel>
        </>
    );
};

export default Bilibili;
