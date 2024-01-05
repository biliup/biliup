import { Form, Modal, Notification, Typography, Collapse, ArrayField, Button } from "@douyinfe/semi-ui";
import { IconPlusCircle, IconMinusCircle } from '@douyinfe/semi-icons';
import { FormApi } from "@douyinfe/semi-ui/lib/es/form";
import React, { useRef } from "react";
import { useState } from "react";
import {fetcher, LiveStreamerEntity, sendRequest, StudioEntity} from "../lib/api-streamer";
import useSWR from "swr";
import useSWRMutation from 'swr/mutation';

type TemplateModalProps = {
    visible?: boolean
    entity?: LiveStreamerEntity
    children?: React.ReactNode
    onOk: (e: any) => Promise<void>
}


const TemplateModal: React.FC<TemplateModalProps> = ({ children, entity , onOk}) => {
    let message = '该项为必填项';
    const api = useRef<FormApi>();
    const { data: templates, error, isLoading } = useSWR<StudioEntity[]>("/v1/upload/streamers", fetcher);

    const [visible, setVisible] = useState(false);
    const showDialog = () => {
        setVisible(true);
    };
    const handleOk = async () => {
        let values = await api.current?.validate();
        await onOk(values);
        setVisible(false);
    };
    const handleCancel = () => {
        setVisible(false);
    };
    const childrenWithProps = React.Children.map(children, child => {
        if (React.isValidElement<any>(child)) {
            return React.cloneElement(child, {
                onClick: () => {
                    showDialog();
                    child.props.onClick?.();
                }
            })//这次我们通过React.cloneElement添加属性
        }
    })
    const list = templates?.map((template) => {
        return {
            value: template.id, label: template.template_name
        }
    })
    return (
        <>
            {childrenWithProps}
            <Modal
                title="录播管理"
                visible={visible}
                onOk={handleOk}
                style={{ width: 600 }}
                onCancel={handleCancel}
                bodyStyle={{ overflow: 'auto', maxHeight: 'calc(100vh - 320px)', paddingLeft: 10, paddingRight: 10 }}
            >
                <Form initValues={entity} getFormApi={formApi => api.current = formApi}>

                    <Form.Input
                        field='remark'
                        label="录播备注"
                        trigger='blur'
                        rules={[
                            { required: true, message },
                        ]}
                    />

                    <Form.Input
                        field='url'
                        label="直播链接"
                        trigger='blur'
                        rules={[
                            { required: true, message },
                        ]}
                    />
                    <Form.Input
                        field='filename_prefix'
                        label={{text: "文件名模板", optional: true} }
                        // initValue='./video/%Y-%m-%d/%H_%M_%S{title}'
                        placeholder='{streamer}%Y-%m-%d %H_%M_%S{title}'
                    />
                    <Form.Select showClear field="upload_id" label={{ text: '投稿模板', optional: true }} style={{ width: 176 }} optionList={list} />
                    <Collapse keepDOM>
                    <Collapse.Panel header="更多设置" itemKey="processors">
                    <Form.Input field='format' label='视频格式' placeholder='flv' style={{ width: 176 }}
                    helpText='视频保存格式。如需使用mp4格式，必须切换downloader为ffmpeg或者streamlink，youtube不支持。' />
                    <Form.Section text="preprocessor">
                    <ArrayField field='preprocessor'>
                        {({ add, arrayFields }) => (
                            <React.Fragment>
                                <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                    Add new line
                                </Button>
                                {arrayFields.map(({ field, key, remove }, i) => (
                                    <div key={key} style={{ width: 1000, display: "flex" }}>
                                        <Form.Input
                                            field={`${field}[run]`}
                                            label={`run = `}
                                            labelPosition='inset'
                                            style={{ width: 400, marginRight: 16 }}></Form.Input>
                                        <Button
                                            type="danger"
                                            theme="borderless"
                                            icon={<IconMinusCircle />}
                                            onClick={remove}
                                            style={{ margin: 12 }}
                                        />
                                    </div>
                                ))}
                            </React.Fragment>
                        )}
                    </ArrayField>
                    </Form.Section>
                    <Form.Section text="downloaded_processor">
                    <ArrayField field='downloaded_processor'>
                        {({ add, arrayFields }) => (
                            <React.Fragment>
                                <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                    Add new line
                                </Button>
                                {arrayFields.map(({ field, key, remove }, i) => (
                                    <div key={key} style={{ width: 1000, display: "flex" }}>
                                        <Form.Input
                                            field={`${field}[run]`}
                                            label={`run = `}
                                            labelPosition='inset'
                                            style={{ width: 400, marginRight: 16 }}></Form.Input>
                                        <Button
                                            type="danger"
                                            theme="borderless"
                                            icon={<IconMinusCircle />}
                                            onClick={remove}
                                            style={{ margin: 12 }}
                                        />
                                    </div>
                                ))}
                            </React.Fragment>
                        )}
                    </ArrayField>
                    </Form.Section>
                    <Form.Section text="postprocessor">
                    <ArrayField field='postprocessor'>
                        {({ add, arrayFields }) => (
                            <React.Fragment>
                                <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                    Add new line
                                </Button>
                                {arrayFields.map(({ field, key, remove }, i) => (
                                    <div key={key} style={{ width: 1000, display: "flex" }}>
                                        <Form.Select
                                            field={`${field}.cmd`}
                                            label="操作"
                                            rules={[
                                                { required: true, message },
                                            ]}
                                            noLabel>
                                            <Form.Select.Option value="run">run</Form.Select.Option>
                                            <Form.Select.Option value="mv">mv</Form.Select.Option>
                                            <Form.Select.Option value="rm">rm</Form.Select.Option>
                                        </Form.Select>
                                        {api.current?.getValue(field)?.cmd !== 'rm' ? (
                                            <Form.Input
                                            field={`${field}[${api.current?.getValue(field)?.cmd}]`}
                                            label='='
                                            labelPosition="inset"
                                            style={{ width: 350, marginRight: 16 }}></Form.Input>
                                        ) : null}
                                        <Button
                                            type="danger"
                                            theme="borderless"
                                            icon={<IconMinusCircle />}
                                            onClick={remove}
                                            style={{ margin: 12 }}
                                        />
                                    </div>
                                ))}
                            </React.Fragment>
                        )}
                    </ArrayField>
                    </Form.Section>
                    <Form.Section text="ffmpeg参数">
                    <ArrayField field='opt_args'>
                        {({ add, arrayFields }) => (
                            <React.Fragment>
                                <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                    Add new line
                                </Button>
                                {arrayFields.map(({ field, key, remove }, i) => (
                                    <div key={key} style={{ width: 1000, display: "flex" }}>
                                        <Form.Input
                                            field={field}
                                            label={`参数${i+1}`}
                                            labelPosition="left"
                                        ></Form.Input>
                                        <Button
                                            type="danger"
                                            theme="borderless"
                                            icon={<IconMinusCircle />}
                                            onClick={remove}
                                            style={{ margin: 12 }}
                                        />
                                    </div>
                                ))}
                            </React.Fragment>
                        )}
                    </ArrayField>
                    </Form.Section>
                    </Collapse.Panel>
                    </Collapse>
                </Form>
            </Modal>
        </>
    );
}

export default TemplateModal;
