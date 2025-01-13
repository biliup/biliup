'use client'
import {
    Layout,
    Nav,
    Button,
    Tag,
    Typography,
    Popconfirm,
    Notification,
    Card
} from '@douyinfe/semi-ui';
import {
    IconHelpCircle,
    IconPlusCircle,
    IconVideoListStroked,
    IconEdit2Stroked, IconDeleteStroked
} from '@douyinfe/semi-icons';
import { List, ButtonGroup } from '@douyinfe/semi-ui';
import React, { useState } from "react";
import useStreamers from '../lib/use-streamers';
import TemplateModal from '../ui/TemplateModal';
import {LiveStreamerEntity, put, requestDelete, sendRequest} from "../lib/api-streamer";
import useSWRMutation from "swr/mutation";

export default function Home() {
    const { Header, Content } = Layout;
    const { Text } = Typography;
    const { streamers, isLoading } = useStreamers();
    const { trigger: deleteStreamers} = useSWRMutation('/v1/streamers', requestDelete);
    const { trigger: updateStreamers} = useSWRMutation('/v1/streamers', put);
    const { trigger } = useSWRMutation('/v1/streamers', sendRequest)

    const onConfirm = async (id: number) => {
        await deleteStreamers(id);
    };
    const handleEntityPostprocessor = (values: any) => {
        if (values?.postprocessor) {
            values.postprocessor = values.postprocessor.map((element: { [key: string]: string } | string) => {
                if (element === "rm") {
                    return { cmd: "rm" };
                } else if (typeof element === "object" && !element.cmd) {
                    const [key, value] = Object.entries(element)[0];
                    return { cmd: key, value: value };
                }
                return element
            });
            // console.log(values.postprocessor);
        }
        return values;
    };
    const data: LiveStreamerEntity[] | undefined = streamers?.map((live) => {
        let status;
        switch (live.status) {
            case 'Working': status = <Tag color='red'>直播中</Tag>; break;
            case 'Idle': status = <Tag color='green'>空闲</Tag>; break;
            case 'Pending': status = <Tag color='grey'>未知</Tag>; break;
            case 'Inspecting': status = <Tag color='indigo'>检测中</Tag>; break;
            case 'OutOfSchedule': status = <Tag color='green'>非录播时间</Tag>; break;
        }
        return {...handleEntityPostprocessor(live), status};
    });

    const handleOk = async (values: any) => {
        if (values?.postprocessor) {
            values.postprocessor = values.postprocessor.map(
                ({ cmd, value }: { cmd: string; value: string; }) => (cmd === "rm" ? "rm" : {[cmd]: value})
            );
        }
        try {
            const res = await trigger(values);
        } catch (e: any) {
            Notification.error({
                title: '创建失败',
                content: <Typography.Paragraph style={{ maxWidth: 450 }}>{e.message}</Typography.Paragraph>,
                style: { width: 'min-content' }
            });
            throw e;
        }
    };

    const handleUpdate = async (values: any) => {
        // console.log(values);
        delete values.status;
        if (values?.postprocessor) {
            values.postprocessor = values.postprocessor.map(
                ({ cmd, value }: { cmd: string; value: string; }) => (cmd === "rm" ? "rm" : {[cmd]: value})
            );
        }
        try {
            const res = await updateStreamers(values);
        } catch (e: any) {
            Notification.error({
                title: '更新失败',
                content: <Typography.Paragraph style={{ maxWidth: 450 }}>{e.message}</Typography.Paragraph>,
                style: { width: 'min-content' }
            });
            throw e;
        }
    };
    return (<>
        <Header style={{
            backgroundColor: 'var(--semi-color-bg-1)', position: 'sticky', top: 0, zIndex: 1
        }}>
            <nav
                style={{
                    display:'flex',
                    paddingLeft:'25px',
                    paddingRight:'25px',
                    alignItems:'center',
                    justifyContent:'space-between',
                    flexWrap:'wrap',
                    boxShadow:'0 1px 2px 0 rgb(0 0 0 / 0.05)'         
                }}
            >
                <div style={{
                    display:'flex',
                    gap:10,
                    justifyContent:'center',
                    alignItems:'center',
                    flexWrap:'wrap'
                }}>
                    <IconVideoListStroked size='large' style={{
                        backgroundColor: 'rgba(var(--semi-green-4), 1)',
                        borderRadius: 'var(--semi-border-radius-large)',
                        color: 'var(--semi-color-bg-0)',
                        padding:'6px'
                    }}/>
                    <h4>录播管理</h4>
                </div>
                <div
                    style={{
                        display:'flex',
                        flexWrap:'wrap',
                        alignItems:'center',
                        justifyContent:'center',
                        gap:6
                    }}
                >
                    <Button
                        theme="borderless"
                        icon={<IconHelpCircle size="large" />}
                        style={{
                            color: 'var(--semi-color-text-2)',
                        }} onClick={() => window.location.href = '/static/ds_update.log'} />
                    <TemplateModal onOk={handleOk}>
                        <Button icon={<IconPlusCircle />} theme="solid" style={{ marginRight: 10 }}>新建</Button>
                    </TemplateModal>
                </div>
            </nav>
        </Header>
        <Content
            style={{
                padding: '24px',
                backgroundColor: 'var(--semi-color-bg-0)',
            }}
        >
            <main>
                <List
                    grid={{
                        gutter: 12,
                        xs: 24,
                        sm: 24,
                        md: 12,
                        lg: 8,
                        xl: 6,
                        xxl: 4,
                    }}
                    dataSource={data}
                    renderItem={item => (
                        <List.Item>
                            <Card
                            shadows='hover'
                            style={{
                                // maxWidth: 360,
                                margin: '9px 0px',
                                width: '100%'
                                // flexGrow: 1,
                            }}
                            bodyStyle={{
                                // display: 'flex',
                                // alignItems: 'center',
                                // justifyContent: 'space-between'
                            }}>
                                <div style={{ position: 'absolute', right: 20, top: 9}}>{item.status}</div>
                                <div style={{ display: 'flex', justifyContent: 'space-between' }}>
                                    <h3 style={{ color: 'var(--semi-color-text-0)', fontWeight: 500, maxWidth: '60%', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{item.remark}</h3>
                                </div>
                                <Text style={{width: '101%'}} ellipsis={{ showTooltip: true }} type="tertiary">{item.url}</Text>
                                <div style={{ margin: '0', display: 'flex', padding: "0 0 32px 0px", justifyContent: 'flex-end' }}>
                                    <ButtonGroup theme='borderless' style={{ position: 'absolute', right: 20, bottom: 15}}>
                                        <TemplateModal onOk={handleUpdate} entity={item}>
                                            <Button theme='borderless' icon={<IconEdit2Stroked />}></Button>
                                        </TemplateModal>
                                        <span className="semi-button-group-line semi-button-group-line-borderless semi-button-group-line-primary"></span>
                                        <Popconfirm
                                            title="确定是否要删除？"
                                            content="此操作将不可逆"
                                            onConfirm={async () => await onConfirm(item.id)}
                                            // onCancel={onCancel}
                                        >
                                            <Button theme='borderless' icon={<IconDeleteStroked />}></Button>
                                        </Popconfirm>
                                    </ButtonGroup>
                                </div>
                            </Card>

                        </List.Item>
                    )}
                />
            </main>
        </Content>
    </>);
}
