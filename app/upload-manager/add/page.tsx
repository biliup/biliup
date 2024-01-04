'use client'
import React, {useEffect, useRef, useState} from 'react';
import { Form, Button, Toast, Typography, Notification } from '@douyinfe/semi-ui';
import {FormApi} from '@douyinfe/semi-ui/lib/es/form';
import {IconChevronDown, IconChevronUp, IconPlusCircle} from '@douyinfe/semi-icons';
import {registerMediaQuery, responsiveMap } from '../../lib/utils'
import { sendRequest, StudioEntity } from '../../lib/api-streamer';
import useSWRMutation from 'swr/mutation';
import { useRouter } from 'next/navigation';
import TemplateFields from "../../ui/TemplateFields";

export default function Add() {
    const { Paragraph } = Typography;
    const { trigger } = useSWRMutation('/v1/upload/streamers', sendRequest);
    const router = useRouter();
    const api = useRef<FormApi>();
    const [labelPosition, setLabelPosition] = useState<'top' | 'left' | 'inset'>('inset');
    useEffect(()=> {
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

    return (
        <>
            <div style={{display: 'flex', flexDirection: 'row-reverse', paddingRight: 12}}>
                <Button onClick={()=>{api.current?.submitForm()}} type='primary' icon={<IconPlusCircle size='large'/>} theme='solid' style={{ marginTop: 12, marginRight: 4 }}>创建模板</Button>
            </div>
            <main style={{
                backgroundColor: 'var(--semi-color-bg-0)',
                display: 'flex',
                justifyContent: 'space-around',
            }}>
                <Form autoScrollToError onSubmit={async (values)=>{
                    try {
                        const studioEntity: StudioEntity = {
                            template_name: values.template_name,
                            user_cookie: values.user_cookie,
                            copyright: values.copyright,
                            id: values.id,
                            source: values.source ?? '',
                            tid: values.tid[1],
                            cover: values.cover ?? '',
                            title: values.title ?? '',
                            desc: values.desc ?? '',
                            dynamic: values.dynamic ?? '',
                            tags: values.tag ?? '',
                            interactive: values.interactive ?? 0,
                            dolby: values.dolby ?? 0,
                            lossless_music: values.lossless_music ?? 0,
                            up_selection_reply: values.up_selection_reply ?? false,
                            up_close_reply: values.up_close_reply ?? false,
                            up_close_danmu: values.up_close_danmu ?? false,
                            open_elec: values.open_elec,
                            no_reprint: values.no_reprint,
                            mission_id: values.mission_id,
                            dtime: values.dtime,
                            format: values.format,
                            credits: values.credits,
                            preprocessor: values.preprocessor,
                            downloaded_processor: values.downloaded_processor,
                            postprocessor: values.postprocessor,
                            opt_args: values.opt_args,
                        }

                        const result = await trigger(studioEntity);

                        Toast.success('创建成功');
                        router.push('/upload-manager');
                    }catch (e: any) {
                        // error handling
                        Notification.error({
                            title: '创建失败',
                            content: <Paragraph style={{maxWidth: 450}}>{e.message}</Paragraph>,
                            // theme: 'light',
                            // duration: 0,
                            style: {width: 'min-content'}
                        });
                        throw e
                    }

                }} component={TemplateFields} getFormApi={formApi => api.current = formApi} labelWidth='180px' labelPosition={labelPosition}/>
            </main>
        </>
    );
}
