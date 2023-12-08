'use client'
import React, {useEffect, useRef, useState} from "react";
import EditTemplate from "@/app/upload-manager/edit/page";
import {Button, Form, Layout, Nav} from "@douyinfe/semi-ui";
import {registerMediaQuery, responsiveMap} from "@/app/lib/utils";
import {IconPlusCircle, IconStar, IconVideoListStroked} from "@douyinfe/semi-icons";
import useSWR from "swr";
import {fetcher, put} from "@/app/lib/api-streamer";
import useSWRMutation from "swr/mutation";
import {FormApi} from "@douyinfe/semi-ui/lib/es/form";

const Dashboard: React.FC = () => {
    const {Header, Content} = Layout;
    const { data: entity, error, isLoading } = useSWR("/v1/configuration", fetcher);
    const { trigger } = useSWRMutation("/v1/configuration", put);
    const api = useRef<FormApi>();

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
                 footer={<Button onClick={()=> api.current?.submitForm()} icon={<IconPlusCircle />} theme="solid" style={{ marginRight: 10 }}>保存</Button>}
                 mode="horizontal"
            ></Nav>
        </Header>
        <Content>
            <Form  initValues={entity}
                   onSubmit={async (values) => {
                       await trigger(values)
                   }}
                   getFormApi={formApi => api.current = formApi}
                   style={{padding: '10px', marginLeft: '30px'}} labelPosition={labelPosition} labelWidth='300px'>
            <Form.InputNumber
                        field='file_size'
                        label={{text: "分段大小"} }
                        suffix={'MB'}
                    />
            <Form.InputNumber
                field='segment_time'
                label={{text: '分段时间'} }
                suffix={'分钟'}
            />
            <Form.Input
                field="filtering_threshold"
                label="filtering_threshold"
                style={{width: 400}}
            />
            <Form.Input
                field="uploader"
                label="uploader"
                style={{width: 400}}
            />
            <Form.Input
                field="lines"
                label="lines"
                style={{width: 400}}
            />
            <Form.Input
                field="threads"
                label="threads"
                style={{width: 400}}
            />
            <Form.Input
                field="delay"
                label="delay"
                style={{width: 400}}
            />
            <Form.Input
                field="event_loop_interval"
                label="event_loop_interval"
                style={{width: 400}}
            />
            <Form.Input
                field="pool1_size"
                label="pool1_size"
                style={{width: 400}}
            />
            <Form.Input
                field="pool2_size"
                label="pool2_size"
                style={{width: 400}}
            />
            <Form.Switch
                field="use_live_cover"
                label="use_live_cover"
            />
            <Form.Input
                field="douyucdn"
                label="douyucdn"
                style={{width: 400}}
            />
            <Form.Input
                field="douyu_danmaku"
                label="douyu_danmaku"
                style={{width: 400}}
            />
            <Form.Input
                field="douyu_rate"
                label="douyu_rate"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_before_date"
                label="youtube_before_date"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_enable_download_live"
                label="youtube_enable_download_live"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_enable_download_playback"
                label="youtube_enable_download_playback"
                style={{width: 400}}
            />
            <Form.Input
                field="twitch_danmaku"
                label="twitch_danmaku"
                style={{width: 400}}
            />
            <Form.Input
                field="twitch_disable_ads"
                label="twitch_disable_ads"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_after_date"
                label="youtube_after_date"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_max_videosize"
                label="youtube_max_videosize"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_max_resolution"
                label="youtube_max_resolution"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_prefer_acodec"
                label="youtube_prefer_acodec"
                style={{width: 400}}
            />
            <Form.Input
                field="youtube_prefer_vcodec"
                label="youtube_prefer_vcodec"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_qn"
                label="bili_qn"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_force_cn01_domains"
                label="bili_force_cn01_domains"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_force_cn01"
                label="bili_force_cn01"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_force_ov05_ip"
                label="bili_force_ov05_ip"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_cdn_fallback"
                label="bili_cdn_fallback"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_fallback_api"
                label="bili_fallback_api"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_liveapi"
                label="bili_liveapi"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_force_source"
                label="bili_force_source"
                style={{width: 400}}
            />
            <Form.Input
                field="bili_protocol"
                label="bili_protocol"
                style={{width: 400}}
            />
            <Form.Input
                field="bilibili_danmaku"
                label="bilibili_danmaku"
                style={{width: 400}}
            />

            <Form.Input
                field="douyin_quality"
                label="douyin_quality"
                style={{width: 400}}
            />
            <Form.Input
                field="douyin_danmaku"
                label="douyin_danmaku"
                style={{width: 400}}
            />
            <Form.Input
                field="huya_max_ratio"
                label="huya_max_ratio"
                style={{width: 400}}
            />
            <Form.Input
                field="huya_danmaku"
                label="huya_danmaku"
                style={{width: 400}}
            />
            <Form.Input
                field="huyacdn"
                label="huyacdn"
                style={{width: 400}}
            />
        </Form>
        </Content>
    </>
}

export default Dashboard;