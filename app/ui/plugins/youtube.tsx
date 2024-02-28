"use client";
import React from "react";
import { Form, Select, Collapse } from "@douyinfe/semi-ui";

type Props = {
    entity: any;
};

const YouTube: React.FC<Props> = (props) => {
    const entity = props.entity;
    return (
        <>
            <Collapse.Panel header="YouTube" itemKey="youtube">
                <Form.Switch
                    initValue={
                        entity?.hasOwnProperty("youtube_enable_download_live")
                            ? entity["youtube_enable_download_live"]
                            : true
                    }
                    field="youtube_enable_download_live"
                    extraText="### 是否下载直播 默认开启
关闭后将忽略直播下载（可以下载回放） 避免网络被风控(有些网络只能下载回放无法下载直播)的时候还会尝试下载直播
大量下载时极易风控 如对实时性要求不高推荐关闭
一个人同时开启多个直播只能录制最新录制的那个
如果正在录制直播将无法下载回放
例如录制https://www.youtube.com/@NeneAmanoCh/streams，关闭后将忽略正在直播
"
                    label="下载直播（youtube_enable_download_live）"
                />
                <Form.Switch
                    initValue={
                        entity?.hasOwnProperty(
                            "youtube_enable_download_playback"
                        )
                            ? entity["youtube_enable_download_playback"]
                            : true
                    }
                    field="youtube_enable_download_playback"
                    extraText="是否下载直播回放 默认开启
关闭后将忽略直播下载回放(不会影响正常的视频下载) 只想录制直播的可以开启
如果正在下载回放将无法录制直播
例如录制https://www.youtube.com/@NeneAmanoCh/streams，关闭后将忽略直播回放
"
                    label="下载回放（youtube_enable_download_playback）"
                />
                <Form.Input
                    field="youtube_after_date"
                    extraText="仅下载该日期之后的视频,默认不限制"
                    label="下载起始日期（youtube_after_date）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="youtube_before_date"
                    extraText="仅下载该日期之前的视频（可与上面的youtube_after_date配合使用，构成指定下载范围区间）默认不限制"
                    label="下载截止日期（youtube_before_date）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="youtube_max_videosize"
                    extraText="限制单个视频的最大大小
默认不限制
直播无此功能
注意：此参数优先级高于分辨率设置，并且不包括音频部分的大小，仅仅只是视频部分的大小。
此功能在一部分视频上无法使用 推荐使用画质限制不开启此功能"
                    label="视频大小上限（youtube_max_videosize）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.InputNumber
                    field="youtube_max_resolution"
                    extraText="设置偏好的YouTube下载最高纵向分辨率
默认不限制
可以用此限制画质
例如设置为1080最高只会下载1080P画质"
                    label="视频分辨率上限（youtube_max_resolution）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="youtube_prefer_vcodec"
                    label="偏好视频封装格式（youtube_prefer_vcodec）"
                    style={{ width: "100%" }}
                    fieldStyle={{
                        alignSelf: "stretch",
                        padding: 0,
                    }}
                />
                <Form.Input
                    field="youtube_prefer_acodec"
                    extraText="设置偏好的YouTube下载封装格式
默认不限制
请务必记得安装ffmpeg
如无特殊需求不建议筛选封装格式 特别是录制直播时 多数直播mp4都是不可用的
bilibili支持 mp4 mkv webm 无需筛选也能上传
支持同时添加多个编码，自动优选指定编码格式里最好的画质/音质版本。
视频：其中avc编码最高可以下载到1080p的内容，vp9最高可以下载到4k以及很少部分8k内容，av01画质不是所有视频都有，但是大部分8k视频的8k画质只有av01编码。
音频：其中opus编码最高48KHz采样，mp4a（AAC）最高44.1KHz采样，理论上来说opus音质会更好一些。
如需指定封装格式，请按以下推荐设置。mp4：avc+mp4a;av01+mp4a. mkv:vp9+mp4a,avc+opus. webm:av01+opus;vp9+opus."
                    label="偏好音频封装格式（youtube_prefer_acodec）"
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
export default YouTube;
