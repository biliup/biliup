"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

type Props = {
    entity: any;
};

const Twitch: React.FC<Props> = (props) => {
    const entity = props.entity;
    return (
        <>
            <Collapse.Panel header="Twitch" itemKey="twitch">
                <Form.Switch
                    field="twitch_danmaku"
                    extraText="录制Twitch弹幕，默认关闭【只有下载器为FFMPEG时才有效】"
                    label="录制弹幕（twitch_danmaku）"
                />
                <Form.Switch
                    initValue={
                        entity?.hasOwnProperty("twitch_disable_ads")
                            ? entity["twitch_disable_ads"]
                            : true
                    }
                    field="twitch_disable_ads"
                    extraText="去除Twitch广告功能，默认开启【只有下载器为FFMPEG时才有效】
这个功能会导致Twitch录播分段，因为遇到广告就自动断开了，这就是去广告。若需要录播完整一整段可以关闭这个，但是关了之后就会有紫色屏幕的CommercialTime
还有一个不想视频分段的办法是去花钱开一个Turbo会员，能不看广告，然后下面的user里把twitch的cookie填上，也能不看广告，自然就不会分段了"
                    label="去除广告（twitch_disable_ads）"
                />
            </Collapse.Panel>
        </>
    );
};

export default Twitch;
