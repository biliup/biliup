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

                    <ArrayField field='preprocessor'>
                        {({ add, arrayFields }) => (
                            <Form.Section text="下载前处理">
                                <div className="semi-form-field-extra">
                                    开始下载直播时触发，将按自定义顺序执行自定义操作，仅支持shell指令
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

                    <ArrayField field='postprocessor'>
                        {({ add, arrayFields }) => (
                            <Form.Section text="上传完成后处理">
                                <div className="semi-form-field-extra" >
                                    上传完成后触发，将按自定义顺序执行自定义操作，当postprocessor不存在时，默认执行删除文件操作，示例：
                                    <br />
                                    <code>run = echo hello!</code> 执行任意命令，等同于在shell中运行,视频文件路径作为标准输入传入
                                    <br />
                                    <code>mv = backup/</code> 移动文件到backup目录下
                                    <br />
                                    <code>run = python3 path/to/mail.py</code> 执行一个 Python 脚本，可以用来发送邮件等。<a href="https://biliup.github.io/biliup/Guide.html#%E4%B8%8A%E4%BC%A0%E5%AE%8C%E6%88%90%E5%90%8E%E5%8F%91%E9%80%81%E9%82%AE%E4%BB%B6%E9%80%9A%E7%9F%A5" target="_blank">自动发信通知脚本示例</a>
                                    <br />
                                    <code>run = sh ./run.sh</code> 执行一个shell脚本，用途多样，主要调用系统内的cli工具。<a href="https://gist.github.com/UVJkiNTQ/ae4282e8f9fe4e45b3144b57605b4178" target="_blank">自动上传网盘脚本示例</a>
                                    <br />
                                    <code>rm</code>  删除文件，为默认操作
                                </div>
                                <Button icon={<IconPlusCircle />} onClick={add} theme="light">
                                    添加行
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
                                            label={`参数${i+1}`}
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
