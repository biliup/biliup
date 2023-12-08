import { Form, Modal, Notification, Typography } from "@douyinfe/semi-ui";
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
                title="新建"
                visible={visible}
                onOk={handleOk}
                // style={{ width: 600 }}
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
                </Form>
            </Modal>
        </>
    );
}

export default TemplateModal;
