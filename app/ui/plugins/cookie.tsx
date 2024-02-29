"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

type Props = {
    entity: any;
    list: any;
};

const Cookie: React.FC<Props> = (props) => {
    const entity = props.entity;
    const list = props.list;
    return (
        <>
            <Collapse.Panel header="用户 Cookie" itemKey="user">
                <Form.Input
                    field="user.bili_cookie"
                    extraText={
                        <div className="semi-form-field-extra">
                            请至少填入bilibili cookie之一。推荐使用「
                            <a
                                href="https://github.com/biliup/biliup-rs"
                                title="「biliup-rs」 Github 项目主页"
                                target="_blank"
                            >
                                biliup-rs
                            </a>
                            」来获取。
                            <br />
                        </div>
                    }
                    label="哔哩哔哩 Cookie 文本（bili_cookie）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Select
                    field="user.bili_cookie_file"
                    label="哔哩哔哩 Cookie 文件（bili_cookie_file）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                    optionList={list}
                    extraText="只支持「biliup-rs」生成的文件。当与上一个配置项同时存在时，将优先使用文件。"
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
                    label="抖音 Cookie（douyin_cookie）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.twitch_cookie"
                    extraText={
                        <div className="semi-form-field-extra">
                            【仅限Turbo会员】如录制Twitch时遇见视频流中广告过多的情况，可尝试在此填入cookie，可以大幅减少视频流中的twitch广告
                            <br />
                            该 Cookie
                            存在过期风险，Cookie过期后会在日志输出警告，请注意及时更换。
                            <br />
                            当 Cookie 失效，录制时将忽略
                            Cookie。（经作者个人测试，可保持未过期状态四个月以上）
                            <br />
                            twitch_cookie 获取方式：在浏览器中打开 twitch.tv ，
                            F12 调出控制台，在控制台中执行如下代码：
                            <br />
                            <code
                                style={{ color: "blue" }}
                            >{`document.cookie.split("; ").find(item={'>'}item.startsWith("auth-token="))?.split("=")[1]`}</code>
                            <br />
                            twitch_cookie&nbsp;需要在&nbsp;downloader=
                            &quot;ffmpeg&quot;&nbsp;时候才会生效。
                        </div>
                    }
                    label="Twitch Cookie（twitch_cookie）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.youtube_cookie"
                    extraText={
                        <div style={{ fontSize: "14px" }}>
                            使用Cookies登陆YouTube帐号，可用于下载会限、私享等未登录账号无法访问的内容。
                            <br />
                            <style></style>可以使用 Chrome 插件「Get
                            cookies.txt」来生成txt文件，请使用 Netscape 格式的
                            Cookies 文本路径。
                        </div>
                    }
                    label="TouTube Cookie（youtube_cookie）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.niconico-email"
                    extraText="与您的 Niconico 账户相关联的电子邮件或电话号码。"
                    label="ニコニコ動画 用户名（niconico-email）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.niconico-password"
                    mode="password"
                    extraText="您的 Niconico 账户的密码。"
                    label="ニコニコ動画 密码（niconico-password）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.niconico-user-session"
                    extraText="用户会话令牌的值，可作为提供密码的替代方法。"
                    label="ニコニコ動画 用户会话令牌（niconico-user-session）"
                    mode="password"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.niconico-purge-credentials"
                    extraText="清除缓存的 Niconico 凭证，以启动一个新的会话并重新认证。"
                    label="ニコニコ動画 清除凭证缓存（niconico-purge-credentials）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.afreecatv_username"
                    extraText="您的 AfreecaTV 用户名。"
                    label="AfreecaTV 用户名（afreecatv_username）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="user.afreecatv_password"
                    extraText="您的 AfreecaTV 密码。"
                    label="AfreecaTV 密码（afreecatv_password）"
                    mode="password"
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

export default Cookie;
