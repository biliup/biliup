import { Form, Modal, Notification, Typography, Collapse, ArrayField, Button, Collapsible } from "@douyinfe/semi-ui";
import { IconPlusCircle, IconMinusCircle } from '@douyinfe/semi-icons';
import { FormApi } from "@douyinfe/semi-ui/lib/es/form";
import React, { CSSProperties, useRef } from "react";
import { useState } from "react";
import { fetcher, LiveStreamerEntity, sendRequest, StudioEntity } from "../lib/api-streamer";
import useSWR from "swr";
import useSWRMutation from 'swr/mutation';

type TemplateModalProps = {
    visible?: boolean
    entity?: LiveStreamerEntity
    children?: React.ReactNode
    onOk: (e: any) => Promise<void>
}


const TemplateModal: React.FC<TemplateModalProps> = ({ children, entity, onOk }) => {
    const { Paragraph, Title, Text } = Typography;
    let message = '该项为必填项';
    const [isOpen, setOpen] = useState(false);
    const maskStyle = isOpen
        ? {}
        : {
            WebkitMaskImage:
                'linear-gradient(to bottom, black 0%, rgba(0, 0, 0, 1) 60%, rgba(0, 0, 0, 0.2) 80%, transparent 100%)',
        };

    const collapsed = (<div className="semi-form-field-extra">
        流程无报错结束时触发，将按自定义顺序执行操作。默认<Text type="danger">删除</Text>视频文件,若要保留文件请设置为 mv。示例：
        <br />
        <code>rm</code> 删除文件，为默认操作
        <br />
        <code>mv = backup/</code> 移动文件到backup目录下
        <br />
        <code>run = echo hello!</code> 使用当前运行用户在 shell 执行任意命令，<Text type="danger">注意风险。</Text>视频文件路径作为标准输入传入
        {/* TODO: 在这里塞插件仓库 */}
    </div>);
    const toggle = () => {
        setOpen(!isOpen);
    };
    const linkStyle: CSSProperties = {
        position: 'absolute',
        left: 0,
        right: 0,
        textAlign: 'center',
        bottom: isOpen ? 0 : -10,
        fontWeight: 700,
        cursor: 'pointer',
    };
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
            })
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

                    <Form.Select showClear field="upload_id" label={{ text: '投稿模板', optional: true }} style={{ width: 176 }} optionList={list} />

                    <ArrayField
                        field='postprocessor'
                        initValue={entity === undefined ? [{ cmd: 'rm' }] : undefined}
                    >
                        {({ add, arrayFields }) => (
                            <>
                                <Form.Slot label={{ text: "后处理" }} labelPosition="left">
                                    <Button icon={<IconPlusCircle />} onClick={add} theme="light">添加行</Button>
                                </Form.Slot>

                                {arrayFields.map(({ field, key, remove }, i) => (
                                    <div key={key} style={{ display: "flex" }}>
                                        <Form.Select
                                            field={`${field}.cmd`}
                                            label="操作"
                                            rules={[
                                                { required: true, message },
                                            ]}
                                            noLabel>
                                            <Form.Select.Option value="run">run（运行）</Form.Select.Option>
                                            <Form.Select.Option value="mv">mv（移动到）</Form.Select.Option>
                                            <Form.Select.Option value="rm">rm（删除文件）</Form.Select.Option>
                                        </Form.Select>
                                        {api.current?.getValue(field)?.cmd !== 'rm' ? (
                                            <Form.Input
                                                field={`${field}.value`}
                                                label='='
                                                labelPosition="inset"
                                                rules={[
                                                    { required: true, message },
                                                ]}
                                                style={{ width: 300, marginRight: 16 }}></Form.Input>
                                        ) : null}
                                        <Button
                                            type="danger"
                                            theme="borderless"
                                            icon={<IconMinusCircle />}
                                            onClick={remove}
                                            disabled={arrayFields.length <= 1}
                                            style={{ margin: 12 }}
                                        />
                                    </div>
                                ))}
                                <div style={{ position: 'relative' }}>
                                    <Collapsible isOpen={isOpen} collapseHeight={60} style={{ ...maskStyle }}>
                                        {collapsed}
                                    </Collapsible>
                                    <a onClick={toggle} style={{ ...linkStyle }}>
                                        + Show {isOpen ? 'Less' : 'More'}
                                    </a>
                                </div>
                            </>)}
                    </ArrayField>

                    <Form.Input field='format' label='视频格式' placeholder='flv' style={{ width: 176 }}
                                helpText='视频保存格式。不支持stream-gears下载器和Youtube平台。' />

                    <Collapse keepDOM>
                        <Collapse.Panel header="更多设置" itemKey="processors">
                            <Form.Input
                                field='filename_prefix'
                                label={{ text: "文件名模板", optional: true }}
                                // initValue='./video/%Y-%m-%d/%H_%M_%S{title}'
                                placeholder='{streamer}%Y-%m-%dT%H_%M_%S'
                            />

                            <Form.Input
                                field='time_range'
                                extraText={
                                    <div style={{ fontSize: "14px" }}>
                                        格式：&apos;01:00:00-02:00:00&apos;（时:分:秒-时:分:秒）
                                    </div>
                                }
                                        label="录制时间范围"
                                        placeholder="01:00:00-02:00:00"
                                        style={{ width: 176 }}
                            />

                            <ArrayField field='preprocessor'>
                                {({ add, arrayFields }) => (
                                    <Form.Section text="下载前处理">
                                        <div className="semi-form-field-extra">
                                            下载直播前触发，将按自定义顺序执行自定义操作，仅支持shell指令
                                        </div>
                                        <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                            添加行
                                        </Button>
                                        {arrayFields.map(({ field, key, remove }, i) => (
                                            <div key={key} style={{ width: 1000, display: "flex" }}>
                                                <Form.Input
                                                    field={`${field}[run]`}
                                                    label={`run = `}
                                                    labelPosition='inset'
                                                    rules={[
                                                        { required: true, message },
                                                    ]}
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
                                    </Form.Section>
                                )}
                            </ArrayField>

                            <ArrayField field='segment_processor'>
                                {({ add, arrayFields }) => (
                                    <Form.Section text="分段时后处理" >
                                        <div className="semi-form-field-extra">
                                            分段时触发，将按自定义顺序执行自定义操作，仅支持shell指令
                                        </div>
                                        <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                            添加行
                                        </Button>
                                        {arrayFields.map(({ field, key, remove }, i) => (
                                            <div key={key} style={{ width: 1000, display: "flex" }}>
                                                <Form.Input
                                                    field={`${field}[run]`}
                                                    label={`run = `}
                                                    labelPosition='inset'
                                                    rules={[
                                                        { required: true, message },
                                                    ]}
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
                                    </Form.Section>
                                )}
                            </ArrayField>

                            <ArrayField field='downloaded_processor'>
                                {({ add, arrayFields }) => (
                                    <Form.Section text="下载后处理" >
                                        <div className="semi-form-field-extra">
                                            准备上传直播时触发，将按自定义顺序执行自定义操作，仅支持shell指令，如果对上传的视频进行修改，需要保证和filename_prefix命名规则一致，会自动检测上传
                                        </div>
                                        <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                            添加行
                                        </Button>
                                        {arrayFields.map(({ field, key, remove }, i) => (
                                            <div key={key} style={{ width: 1000, display: "flex" }}>
                                                <Form.Input
                                                    field={`${field}[run]`}
                                                    label={`run = `}
                                                    labelPosition='inset'
                                                    rules={[
                                                        { required: true, message },
                                                    ]}
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
                                    </Form.Section>
                                )}
                            </ArrayField>

                            <ArrayField field='opt_args'>
                                {({ add, arrayFields }) => (
                                    <Form.Section text="ffmpeg参数">
                                        <div className="semi-form-field-extra">
                                            如：&quot;-ss&quot;、&quot;00:00:16&quot;，每个参数需单独一行
                                        </div>
                                        <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                            添加行
                                        </Button>
                                        {arrayFields.map(({ field, key, remove }, i) => (
                                            <div key={key} style={{ width: 1000, display: "flex" }}>
                                                <Form.Input
                                                    field={field}
                                                    label={`参数${i + 1}`}
                                                    labelPosition="left"
                                                    rules={[
                                                        { required: true, message },
                                                    ]}
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
                                    </Form.Section>
                                )}
                            </ArrayField>
                        </Collapse.Panel>
                    </Collapse>
                </Form>
            </Modal>
        </>
    );
}

export default TemplateModal;
