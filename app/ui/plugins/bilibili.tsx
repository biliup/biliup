"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

type Props = {
    entity: any;
};

const Bilibili: React.FC<Props> = (props) => {
    const entity = props.entity;
    return (
        <>
            <Collapse.Panel header="哔哩哔哩" itemKey="bilibili">
                <Form.InputNumber
                    field="bili_qn"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            哔哩哔哩自选画质。
                            <br />
                            刚开播可能没有除了原画之外的画质，会先录制原画，后续视频分段（仅ffmpeg
                            streamlink）时录制设置的画质，没有选中的画质则会自动选择相近的画质。
                            <br />
                            可选项：30000（杜比），20000（4K），10000（原画），401（蓝光-杜比），400（蓝光），250（超清），150（高清），80（流畅），0（B站默认，多数情况下是蓝光
                            400）
                        </div>
                    }
                    label="画质等级（bili_qn）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="bilibili_danmaku"
                    extraText="录制哔哩哔哩弹幕，目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持，默认关闭。"
                    label="录制弹幕（bilibili_danmaku）"
                />
                <Form.Switch
                    field="bili_force_cn01"
                    label="强制替换 gotcha01 （bili_force_cn01）"
                />
                <Form.Input
                    field="bili_force_cn01_domains"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            强制替换cn-gotcha01（叔叔自建）为指定的自选域名组（可多个域名，请用逗号分隔）。
                            <br />
                            完整CDN列表请参考「
                            <a
                                href="https://rec.danmuji.org/dev/bilibili-cdn/"
                                title="哔哩哔哩CND列表"
                            >
                                哔哩哔哩CND列表
                            </a>
                            」 中「B站视频云」的部分。
                            <br />
                            此功能目前会和「stream-gears」冲突导致很多分段，请优先使用「ffmpeg」录制。
                            <br />
                            如果海外机器需要使用此功能，需要在「哔哩哔哩直播API（bili_liveapi）」中指定国内的反代API来获取cn-gotcha01的节点信息。
                            <br />
                            海外机的玩法：配合一个国内的机器（例如便宜的腾讯云，阿里云等等）自建反代
                            api.live.bilibili.com 。或者使用「
                            <a
                                href="https://docs.qq.com/doc/DV2dvbXBrckNscU9x"
                                title="哔哩哔哩CND列表"
                            >
                                录播姬反代API文档
                            </a>
                            」 提供的公用反代API。
                            <br />
                            如果海外机到联通或者移动网络线路还不错，就可以参考「哔哩哔哩CND列表」选取一些联通或者移动的节点并填入下面，每次会随机返回填入的其中一个线路，并且会自动判断所填入的节点是否可用。
                        </div>
                    }
                    label="强制替换 gotcha01 域名（bili_force_cn01_domains）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />

                <Form.Input
                    field="bili_force_ov05_ip"
                    extraText="强制替换 ov-gotcha05 的下载地址为指定的自选IP"
                    label="强制替换 ov-gotcha05 IP地址（bili_force_ov05_ip）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="bili_cdn_fallback"
                    extraText="CDN 自动回退（Fallback）开关，默认为开启。例如海外机器优选 ov05 之后，如果 ov05 流一直无法下载，将会自动回退到 ov07 进行下载。"
                    label="自动回退CDN（bili_cdn_fallback）"
                    initValue={
                        entity?.hasOwnProperty("bili_cdn_fallback")
                            ? entity["bili_cdn_fallback"]
                            : true
                    }
                />

                <Form.Input
                    field="bili_liveapi"
                    extraText="自定义哔哩哔哩直播 API，用于获取指定区域（大陆或海外）的直播流链接，默认使用官方 API。"
                    label="哔哩哔哩直播API（bili_liveapi）"
                    style={{ width: "100%" }}
                    placeholder="https://api.live.bilibili.com/"
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="bili_fallback_api"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            自定义 fmp4 流获取不到时，重新获取一遍 flv 直播流的
                            API，默认不重新使用其他api重新获取一遍。
                            <br />
                            海外机器玩法：哔哩哔哩直播API（bili_liveapi）设置为能获取大陆直播流的
                            API，并将哔哩哔哩直播回退API（bili_fallback_api）设置为官方
                            API，然后优选「fmp4」流并使用「streamlink」下载插件（downloader），最后设置优选「cn-gotcha208,ov-gotcha05」两个节点。
                            <br />
                            大陆机器玩法：哔哩哔哩直播API（bili_liveapi）使用官方
                            API，并将哔哩哔哩直播回退API（bili_fallback_api）设置为能获取到海外节点
                            API，然后优选「fmp4」流并使用「streamlink」下载插件（downloader），最后设置优选「cn-gotcha208,ov-gotcha05」两个节点。这样大主播可以使用
                            cn208 的 fmp4 流稳定录制（海外机如需可以通过自建 DNS
                            优选指定线路的 cn208 节点），没有 fmp4
                            流的小主播也可以会退到 ov05 录制 flv 流。
                        </div>
                    }
                    label="哔哩哔哩直播回退API（bili_fallback_api）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="bili_force_source"
                    extraText="哔哩哔哩强制真原画（仅限 TS 与 FMP4 流的 cn-gotcha01 CDN，且 bili_qn >= 10000），默认为关闭，不保证可用性。当无法强制获取到真原画时，将会自动回退到二压原画。"
                    label="强制获取真原画（bili_force_source）"
                />
                <Form.Select
                    field="bili_protocol"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            哔哩哔哩直播流协议。
                            <br />
                            仅国内IP可以解析到 fmp4 流。海外IP只能获取到 FLV
                            流（ov05 与 ov07）和 ts 流（ov105）。
                            <br />
                            由于 fmp4
                            出现需要一定时间，或者某些小主播（大部分只有原画选项的主播）无fmp4流。
                            目前的策略是，如果开播时间小于60s，将会反复尝试获取
                            fmp4 流，如果没获取到就回退到 flv 流。
                            <br />
                            由于 ffmpeg 只能单线程下载，并且 stream-gears
                            存在录制问题，所以目前 fmp4 流只能使用 streamlink +
                            ffmpeg 混合模式录制。
                        </div>
                    }
                    label="直播流协议（bili_protocol）"
                    placeholder="stream（flv，默认）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
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
            </Collapse.Panel>
        </>
    );
};

export default Bilibili;
