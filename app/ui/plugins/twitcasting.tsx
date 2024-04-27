"use client";
import React from "react";
import {Form, Collapse} from "@douyinfe/semi-ui";


const TwitCasting: React.FC = () => {
    return (
        <>
            <Collapse.Panel header="TwitCasting" itemKey="twitcasting">
                <Form.Switch
                    field="twitcasting_danmaku"
                    extraText="录制TwitCasting弹幕，默认关闭"
                    label="录制弹幕（twitcasting_danmaku）"
                />
                <Form.Input
                    field="twitcasting_password"
                    label="TwitCasting直播间密码（twitcasting_password）"
                    style={{width: "100%"}}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
            </Collapse.Panel>
        </>
    );
};

export default TwitCasting;
