"use client";
import React from "react";
import { Form, Collapse } from "@douyinfe/semi-ui";

const Huya: React.FC = () => {
    return (
        <>
            <Collapse.Panel header="虎牙" itemKey="huya">
                <Form.InputNumber
                    field="huya_max_ratio"
                    extraText="虎牙自选录制码率
 可以避免录制如20M的码率，每小时8G左右大小，上传及转码耗时过长。
 20000（蓝光20M）, 10000（蓝光10M）, 8000（蓝光8M）, 2000（超清）, 500（流畅）
 设置为10000则录制小于等于蓝光10M的画质"
                    label="画质等级（huya_max_ratio）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Switch
                    field="huya_danmaku"
                    extraText="录制虎牙弹幕，默认关闭【目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持】"
                    label="录制弹幕（huya_danmaku）"
                />
                <Form.Input
                    field="huyacdn"
                    extraText="如遇到虎牙录制卡顿可以尝试切换线路。可选以下线路
 AL（阿里云 - 直播线路3）, TX（腾讯云 - 直播线路5）, HW（华为云 - 直播线路6）, WS（网宿）, HS（火山引擎 - 直播线路14）, AL13（阿里云）, HW16（华为云）, HY(星域云 - 直播线路66)"
                    label="访问线路（huyacdn）"
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

export default Huya;
