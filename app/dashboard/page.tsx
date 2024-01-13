'use client'
import React, {useEffect, useRef, useState} from "react";
import EditTemplate from "@/app/upload-manager/edit/page";
import {Button, Form, Layout, Nav, Collapse, Avatar} from "@douyinfe/semi-ui";
import {registerMediaQuery, responsiveMap} from "@/app/lib/utils";
import {IconPlusCircle, IconStar, IconVideoListStroked} from "@douyinfe/semi-icons";
import useSWR from "swr";
import {fetcher, put} from "@/app/lib/api-streamer";
import useSWRMutation from "swr/mutation";
import {FormApi} from "@douyinfe/semi-ui/lib/es/form";
import {useBiliUsers} from "../lib/use-streamers";

const Dashboard: React.FC = () => {
    const {Header, Content} = Layout;
    const { data: entity, error, isLoading } = useSWR("/v1/configuration", fetcher);
    const { trigger } = useSWRMutation("/v1/configuration", put);
    const formRef = useRef<FormApi>();
    const [formKey, setFormKey] = useState(0); // 初始化一个key

    // 触发表单重新挂载
    const remountForm = () => {
        setFormKey(prevKey => prevKey + 1); // 更新key的值
    };

    const [labelPosition, setLabelPosition] = useState<'top' | 'left' | 'inset'>('inset');
    useEffect(() => {
        const unRegister = registerMediaQuery(responsiveMap.lg, {
            match: () => {
                setLabelPosition('left');
            },
            unmatch: () => {
                setLabelPosition('top');
            },
        })
        return () => unRegister();
    }, []);

    useEffect(() => {
        remountForm();
    }, [entity]);

    const {biliUsers} = useBiliUsers();
    const list = biliUsers?.map((item) => {
        return {
            value: item.value, label: <>
                <Avatar size="extra-small" src={item.face} />
                <span style={{ marginLeft: 8 }}>
                    {item.name}
                </span></>
        }
    })
// const handleSelectChange = (value) => {
//         let text = value === 'male' ? 'Hi male' : 'Hi female!';
//         formRef.current?.setValue('Note', text);
//     };

    return <>
        <Header style={{backgroundColor: 'var(--semi-color-bg-1)'}}>
            <Nav style={{border: 'none'}}
                 header={<>
                     <div style={{
                         backgroundColor: '#6b6c75ff',
                         borderRadius: 'var(--semi-border-radius-large)',
                         color: 'var(--semi-color-bg-0)',
                         display: 'flex',
                         // justifyContent: 'center',
                         padding: '6px'
                     }}><IconStar size='large'/></div>
                     <h4 style={{marginLeft: '12px'}}>空间配置</h4></>}
                 footer={<Button onClick={()=> formRef.current?.submitForm()} icon={<IconPlusCircle />} theme="solid" style={{ marginRight: 10 }}>保存</Button>}
                 mode="horizontal"
            ></Nav>
        </Header>
        <Content>
            <Form  key = {formKey}
                   initValues={entity}
                   onSubmit={async (values) => {
                        await trigger(values)
                   }}
                   getFormApi={formApi => formRef.current = formApi}
                   style={{padding: '10px', marginLeft: '30px'}} labelPosition={labelPosition} labelWidth='300px'>
            <Form.Section text="全局录播与上传设置">
            <Form.Select
                field="downloader"
                extraText={
                <div style={{ fontSize: "14px" }}>
                    选择全局默认的下载插件, 可选:
                    <br />
                    1. streamlink（streamlink配合ffmpeg混合下载模式，适合用于下载hls_fmp4与hls_ts流，因为streamlink支持多线程拉取, 使用该模式下载flv流时，将会仅使用ffmpeg。请手动安装streamlink以及ffmpeg）
                    <br />
                    2. ffmpeg（纯ffmpeg下载。请手动安装ffmpeg）
                    <br />
                    3. stream-gears（默认）
                </div>}
                style={{ width: 250 }}>
                <Form.Select.Option value="streamlink">streamlink</Form.Select.Option>
                <Form.Select.Option value="ffmpeg">ffmpeg</Form.Select.Option>
                <Form.Select.Option value="stream-gears">stream-gears</Form.Select.Option>
            </Form.Select>
            <Form.InputNumber
                        field='file_size'
                        extraText='录像单文件大小限制，单位MB，超过此大小分段下载'
                        label={{text: "分段大小"} }
                        suffix={'MB'}
                        style={{width: 250}}
                    />
            <Form.Input
                field='segment_time'
                extraText="录像单文件时间限制，格式'00:00:00'（时分秒），超过此大小分段下载，如需使用大小分段请注释此字段"
                label={{text: '分段时间'} }
                style={{width: 250}}
            />
            <Form.InputNumber
                field="filtering_threshold"
                extraText="小于此大小的视频文件将会被过滤删除，单位MB"
                label="filtering_threshold"
                suffix={'MB'}
                style={{width: 250}}
            />
            <Form.Select
                field="submit_api"
                extraText="b站提交接口，默认自动选择，可选web，client"
                style={{ width: 250 }}>
                <Form.Select.Option value="web">web</Form.Select.Option>
                <Form.Select.Option value="client">client</Form.Select.Option>
            </Form.Select>
            <Form.Select
                field="uploader"
                extraText="选择全局默认上传插件，Noop为不上传，但会执行后处理,可选bili_web，biliup-rs(默认值)"
                style={{ width: 250 }}>
                <Form.Select.Option value="bili_web">bili_web</Form.Select.Option>
                <Form.Select.Option value="biliup-rs">biliup-rs</Form.Select.Option>
                <Form.Select.Option value="Noop">Noop</Form.Select.Option>
            </Form.Select>
            <Form.Select
                field="lines"
                extraText="b站上传线路选择，默认为自动模式，目前可手动切换为bda2, kodo, ws, qn"
                style={{ width: 250 }}>
                <Form.Select.Option value="bda2">bda2</Form.Select.Option>
                <Form.Select.Option value="kodo">kodo</Form.Select.Option>
                <Form.Select.Option value="ws">ws</Form.Select.Option>
                <Form.Select.Option value="qn">qn</Form.Select.Option>
            </Form.Select>
            <Form.InputNumber
                field="threads"
                extraText="单文件并发上传数，未达到带宽上限时增大此值可提高上传速度"
                label="threads"
                style={{width: 250}}
            />
            <Form.InputNumber
                field="delay"
                extraText="检测到主播下播后延迟再次检测，单位：秒，避免特殊情况提早启动上传导致漏录
当delay不存在时，默认延迟时间为0秒，没有快速上传的需求推荐设置5分钟(300秒)或按需设置。若设置的延迟时间超过60秒，则会启用分段检测机制，每隔60秒进行一次开播状态的检测。"
                label="delay"
                suffix='s'
                style={{width: 250}}
            />
            <Form.InputNumber
                field="event_loop_interval"
                extraText='平台检测间隔时间，单位：秒。比如虎牙所有主播检测完后会等待30秒 再去从新检测'
                label="event_loop_interval"
                suffix='s'
                style={{width: 250}}
            />
            <Form.InputNumber
                field="pool1_size"
                extraText='线程池1大小，负责下载事件。每个下载都会占用1。应该设置为比主播数量要多一点的数。'
                label="pool1_size"
                style={{width: 250}}
            />
            <Form.InputNumber
                field="pool2_size"
                extraText='线程池2大小，负责上传事件。每个上传都会占用1。
 应该设置为比主播数量要多一点的数，如果开启uploading_record需要设置的更多。'
                label="pool2_size"
                style={{width: 250}}
            />
            <Form.Switch
                field="use_live_cover"
                extraText='使用直播间封面作为投稿封面。此封面优先级低于单个主播指定的自定义封面。（目前支持bilibili,twitch,youtube。直播封面将会保存于cover文件夹下，上传后自动删除）'
                label="use_live_cover"
            />
            </Form.Section>
            <Form.Section text="各平台录播设置">
            <Collapse keepDOM>
            <Collapse.Panel header="斗鱼" itemKey="douyu">
            <Form.Input
                field="douyucdn"
                extraText='如遇到斗鱼录制卡顿可以尝试切换线路。可选以下线路
tctc-h5（备用线路4）, tct-h5（备用线路5）, ali-h5（备用线路6）, hw-h5（备用线路7）, hs-h5（备用线路13）'
                label="douyucdn"
                style={{width: 400}}
            />
            <Form.Switch
                field="douyu_danmaku"
                extraText='录制斗鱼弹幕，默认关闭【目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持】'
                label="douyu_danmaku"
            />
            <Form.InputNumber
                field="douyu_rate"
                extraText='刚开播可能没有除了原画之外的画质 会先录制原画 后续视频分段(仅ffmpeg streamlink)时录制设置的画质
0 原画,4 蓝光4m,3 超清,2 高清'
                label="douyu_rate"
            />
            </Collapse.Panel>
            <Collapse.Panel header="YouTube" itemKey="youtube">
            <Form.Input
                field="youtube_before_date"
                extraText='仅下载该日期之前的视频（可与上面的youtube_after_date配合使用，构成指定下载范围区间）
 默认不限制'
                label="youtube_before_date"
                style={{width: 400}}
            />
            <Form.Switch
                field="youtube_enable_download_live"
                extraText='### 是否下载直播 默认开启
关闭后将忽略直播下载（可以下载回放） 避免网络被风控(有些网络只能下载回放无法下载直播)的时候还会尝试下载直播
大量下载时极易风控 如对实时性要求不高推荐关闭
一个人同时开启多个直播只能录制最新录制的那个
如果正在录制直播将无法下载回放
例如录制https://www.youtube.com/@NeneAmanoCh/streams，关闭后将忽略正在直播
'
                label="youtube_enable_download_live"
            />
            <Form.Switch
                field="youtube_enable_download_playback"
                extraText='是否下载直播回放 默认开启
关闭后将忽略直播下载回放(不会影响正常的视频下载) 只想录制直播的可以开启
如果正在下载回放将无法录制直播
例如录制https://www.youtube.com/@NeneAmanoCh/streams，关闭后将忽略直播回放
'
                label="youtube_enable_download_playback"
            />
            <Form.Input
                field="youtube_after_date"
                extraText='仅下载该日期之后的视频,默认不限制'
                label="youtube_after_date"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_max_videosize"
                extraText='限制单个视频的最大大小
默认不限制
直播无此功能
注意：此参数优先级高于分辨率设置，并且不包括音频部分的大小，仅仅只是视频部分的大小。
此功能在一部分视频上无法使用 推荐使用画质限制不开启此功能'
                label="youtube_max_videosize"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_max_resolution"
                extraText='设置偏好的YouTube下载最高纵向分辨率
默认不限制
可以用此限制画质
例如设置为1080最高只会下载1080P画质'
                label="youtube_max_resolution"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_prefer_acodec"
                extraText='设置偏好的YouTube下载封装格式
默认不限制
请务必记得安装ffmpeg
如无特殊需求不建议筛选封装格式 特别是录制直播时 多数直播mp4都是不可用的
bilibili支持 mp4 mkv webm 无需筛选也能上传
支持同时添加多个编码，自动优选指定编码格式里最好的画质/音质版本。
视频：其中avc编码最高可以下载到1080p的内容，vp9最高可以下载到4k以及很少部分8k内容，av01画质不是所有视频都有，但是大部分8k视频的8k画质只有av01编码。
音频：其中opus编码最高48KHz采样，mp4a（AAC）最高44.1KHz采样，理论上来说opus音质会更好一些。
如需指定封装格式，请按以下推荐设置。mp4：avc+mp4a;av01+mp4a. mkv:vp9+mp4a,avc+opus. webm:av01+opus;vp9+opus.'
                label="youtube_prefer_acodec"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_prefer_vcodec"
                label="youtube_prefer_vcodec"
                style={{width: 400}}
            />
            </Collapse.Panel>
            <Collapse.Panel header="Twitch" itemKey="twitch">
            <Form.Switch
                field="twitch_danmaku"
                extraText='录制Twitch弹幕，默认关闭【只有下载器为FFMPEG时才有效】'
                label="twitch_danmaku"
            />
            <Form.Switch
                field="twitch_disable_ads"
                extraText='去除Twitch广告功能，默认开启【只有下载器为FFMPEG时才有效】
这个功能会导致Twitch录播分段，因为遇到广告就自动断开了，这就是去广告。若需要录播完整一整段可以关闭这个，但是关了之后就会有紫色屏幕的CommercialTime
还有一个不想视频分段的办法是去花钱开一个Turbo会员，能不看广告，然后下面的user里把twitch的cookie填上，也能不看广告，自然就不会分段了'
                label="twitch_disable_ads"
            />
            </Collapse.Panel>
            <Collapse.Panel header="Bilibili" itemKey="bilibili">
            <Form.InputNumber
                field="bili_qn"
                extraText='哔哩哔哩自选画质
刚开播可能没有除了原画之外的画质 会先录制原画 后续视频分段(仅ffmpeg streamlink)时录制设置的画质
30000 杜比,20000 4K,10000 原画,401 蓝光(杜比),400 蓝光,250 超清,150 高清,80 流畅,0 B站默认(多数情况下是蓝光 400)
没有选中的画质则会自动选择相近的画质'
                label="bili_qn"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_force_cn01_domains"
                extraText='强制替换cn-gotcha01（叔叔自建）为指定的自选域名组（可多个域名，请用逗号分隔）
完整CDN列表请参考 https://rec.danmuji.org/dev/bilibili-cdn/ 中"B站视频云"的部分
此功能目前会和stream-gears冲突导致很多分段，请考虑使用ffmpeg录制
如果海外机器需要使用此功能，需要在bili_liveapi中指定国内的反代API来获取cn-gotcha01的节点信息。
海外机的玩法：配合一个国内的机器（例如便宜的腾讯云，阿里云等等）自建反代api.live.bilibili.com。或者使用https://docs.qq.com/doc/DV2dvbXBrckNscU9x 此处提供的公用反代API。
如果海外机到联通或者移动网络线路还不错，就可以参考***完整CDN列表***选取一些联通或者移动的节点并填入下面
每次会随机返回填入的其中一个线路，并且会自动判断所填入的节点是否可用'
                label="bili_force_cn01_domains"
                style={{width: 400}}
            />
            <Form.Switch
                field="bili_force_cn01"
                label="bili_force_cn01"
            />
            <Form.Input
                field="bili_force_ov05_ip"
                extraText='强制替换ov-gotcha05的下载地址为指定的自选IP'
                label="bili_force_ov05_ip"
                style={{width: 400}}
            />
            <Form.Switch
                field="bili_cdn_fallback"
                extraText='CDN自动Fallback开关，默认为开启，例如海外机器优选ov05之后，如果ov05流一直无法下载，将会自动fallback到ov07进行下载。'
                label="bili_cdn_fallback"
            />
            <Form.Input
                field="bili_fallback_api"
                extraText='自定义fmp4流获取不到时，重新获取一遍flv直播流的api，默认不重新使用其他api重新获取一遍。
海外机器玩法：bili_liveapi设置为能获取大陆直播流的API，并将bili_fallback_api设置为官方API，然后优选fmp4流并使用streamlink下载器，最后设置优选cn-gotcha208,ov-gotcha05两个节点。
大陆机器玩法：bili_liveapi取消注释保持默认使用官方API，并将bili_fallback_api设置为能获取到海外节点API，然后优选fmp4流并使用streamlink下载器，最后设置优选cn-gotcha208,ov-gotcha05两个节点。
这样大主播可以使用cn208的fmp4流稳定录制（海外机如需可以通过自建dns优选指定线路的cn208节点），没有fmp4流的小主播也可以会退到ov05录制flv流。'
                label="bili_fallback_api"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_liveapi"
                extraText='自定义哔哩哔哩直播API，用于获取指定区域（大陆或者海外）的直播流链接，默认使用官方API。'
                label="bili_liveapi"
                style={{width: 400}}
            />
            <Form.Switch
                field="bili_force_source"
                extraText='哔哩哔哩强制真原画（仅限TS与FMP4流的 cn-gotcha01 CDN，且 bili_qn >= 10000），默认为关闭
不保证可用性。当无法强制获取到真原画时，将会自动回退到二压原画。'
                label="bili_force_source"
            />
            <Form.Input
                field="bili_protocol"
                extraText='哔哩哔哩直播流协议.可选 stream（flv流）,hls_ts(ts流）与hls_fmp4（fmp4流），默认为stream
 仅国内IP可以解析到fmp4流。海外IP只能获取到flv流（ov05与ov07）和ts流（ov105）
 由于fmp4出现需要一定时间，或者某些小主播（大部分只有原画选项的主播）无fmp4流。
 目前的策略是，如果开播时间小于60s，将会反复尝试获取fmp4流，如果没获取到就回退到flv流。
 由于ffmpeg只能单线程下载，并且stream-gears录制有问题，所以目前fmp4流只能使用streamlink+ffmpeg混合模式。
'
                label="bili_protocol"
                style={{width: 400}}
            />
            <Form.Switch
                field="bilibili_danmaku"
                extraText='录制BILIBILI弹幕，目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持，默认关闭'
                label="bilibili_danmaku"
            />
            </Collapse.Panel>
            <Collapse.Panel header="抖音" itemKey="douyin">
            <Form.Input
                field="douyin_quality"
                extraText='抖音自选画质
 刚开播可能没有除了原画之外的画质 会先录制原画 后续视频分段(仅ffmpeg streamlink)时录制设置的画质
 origin 原画,uhd 蓝光,hd 超清,sd 高清,ld 标清,md 流畅
 没有选中的画质则会自动选择相近的画质优先低清晰度'
                label="douyin_quality"
                style={{width: 400}}
            />
            <Form.Switch
                field="douyin_danmaku"
                extraText='录制抖音弹幕，默认关闭【目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持】'
                label="douyin_danmaku"
            />
            </Collapse.Panel>
            <Collapse.Panel header="虎牙" itemKey="huya">
            <Form.Input
                field="huya_max_ratio"
                extraText='虎牙自选录制码率
 可以避免录制如20M的码率，每小时8G左右大小，上传及转码耗时过长。
 20000（蓝光20M）, 10000（蓝光10M）, 8000（蓝光8M）, 2000（超清）, 500（流畅）
 设置为10000则录制小于等于蓝光10M的画质'
                label="huya_max_ratio"
                style={{width: 400}}
            />
            <Form.Switch
                field="huya_danmaku"
                extraText='录制虎牙弹幕，默认关闭【目前暂时不支持视频按时长分段下的弹幕文件自动分段，只有使用ffmpeg（包括streamlink混合模式）作为下载器才支持】'
                label="huya_danmaku"
            />
            <Form.Input
                field="huyacdn"
                extraText='如遇到虎牙录制卡顿可以尝试切换线路。可选以下线路
 AL（阿里云 - 直播线路3）, TX（腾讯云 - 直播线路5）, HW（华为云 - 直播线路6）, WS（网宿）, HS（火山引擎 - 直播线路14）, AL13（阿里云）, HW16（华为云）, HY(星域云 - 直播线路66)'
                label="huyacdn"
                style={{width: 400}}
            />
            </Collapse.Panel>
            <Collapse.Panel header="用户cookie" itemKey="user">
            <Form.Input
                field="user.bili_cookie"
                extraText='请至少填入bilibili cookie之一。推荐使用 biliup-rs(https://github.com/biliup/biliup-rs) 来获取。'
                label="bili_cookie"
                style={{width: 400}}
            />
            <Form.Select field="user.bili_cookie_file" label={{ text: 'bili_cookie_file' }} style={{ width: 176 }} optionList={list} 
                extraText='和上一个配置项同时存在时，优先使用文件。只支持 biliup-rs 生成的文件。'
            />
            <Form.Input
                field="user.douyin_cookie"
                extraText='如需要录制抖音www.douyin.com/user/类型链接或被风控,
                请在此填入cookie需要__ac_nonce、__ac_signature、sessionid的值请不要将所有cookie填入'
                label="douyin_cookie"
                style={{width: 400}}
            />
            <Form.Input
                field="user.twitch_cookie"
                extraText={
                    <div className="semi-form-field-extra">
                    如录制Twitch时遇见视频流中广告过多的情况，可尝试在此填入cookie，可以大幅减少视频流中的twitch广告（经测试需要在该Cookie所属账号开了TwitchTurbo会员才有用）
                    <br />
                    该cookie有过期风险，cookie过期后会在日志输出警告请及时更换cookie，cookie失效的情况下后续录制将忽略cookie（我个人用了四个月都没过期）
                    <br />
                    twitch_cookie获取方式：在浏览器中打开Twitch.tv，F12调出控制台，在控制台中执行：
                    <br />
                    <code>{`document.cookie.split("; ").find(item={'>'}item.startsWith("auth-token="))?.split("=")[1]`}</code>
                    <br />
                    twitch_cookie需要在downloader= &quot;ffmpeg&quot;时候才会生效
                    </div>
                }
                label="twitch_cookie"
                style={{width: 400}}
            />
            <Form.Input
                field="user.youtube_cookie"
                extraText='使用Cookies登陆YouTube帐号，可用于下载会限，私享等未登录账号无法访问的内容。请使用 Netscape 格式的 Cookies 文本路径。
                可以使用Chrome插件Get cookies.txt来生成txt文件。'
                label="youtube_cookie"
                style={{width: 400}}
            />
            <Form.Input
                field="user.niconico-email"
                extraText='与您的Niconico账户相关的电子邮件或电话号码'
                label="niconico-email"
                style={{width: 400}}
            />
            <Form.Input
                field="user.niconico-password"
                extraText='您的Niconico账户的密码'
                label="niconico-password"
                style={{width: 400}}
            />
            <Form.Input
                field="user.niconico-user-session"
                extraText='用户会话令牌的值。可作为提供密码的替代方法。'
                label="niconico-user-session"
                style={{width: 400}}
            />
            <Form.Input
                field="user.niconico-purge-credentials"
                extraText='清除缓存的 Niconico 凭证，以启动一个新的会话并重新认证。'
                label="niconico-purge-credentials"
                style={{width: 400}}
            />
            <Form.Input
                field="user.afreecatv_username"
                extraText='AfreecaTV 用户名'
                label="afreecatv_username"
                style={{width: 400}}
            />
            <Form.Input
                field="user.afreecatv_password"
                extraText='AfreecaTV 密码'
                label="afreecatv_password"
                style={{width: 400}}
            />
            </Collapse.Panel>
            </Collapse>
            </Form.Section>
        </Form>
        </Content>
    </>
}

export default Dashboard;