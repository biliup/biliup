import React, {useEffect, useRef, useState} from "react";
import {fetcher, LiveStreamerEntity, proxy, requestDelete, sendRequest, StudioEntity, User} from "../lib/api-streamer";
import useSWR from "swr";
import {useRouter} from "next/router";
import {Button, Form, List, Modal, Notification, SideSheet, Toast, Typography} from "@douyinfe/semi-ui";
import AvatarCard from "./AvatarCard";
import {IconPlusCircle} from "@douyinfe/semi-icons";
import {FormApi} from "@douyinfe/semi-ui/lib/es/form";
import useSWRMutation from "swr/mutation";
import {useBiliUsers} from "../lib/use-streamers";

type UserListProps = {
    onCancel?: ((e: (React.MouseEvent<Element, MouseEvent> | React.KeyboardEvent<Element>)) => void)
    visible?: boolean
    children?: React.ReactNode
}
const UserList: React.FC<UserListProps> = ({onCancel, visible}) => {
    const {trigger} = useSWRMutation('/v1/users', sendRequest<User>);
    const {trigger: deleteUser} = useSWRMutation('/v1/users', requestDelete);
    const {biliUsers: list} = useBiliUsers();
    const [modalVisible, setVisible] = useState(false);
    const [confirmLoading, setConfirmLoading] = useState(false);
    const showDialog = () => {
        setVisible(true);
    };
    const handleOk = async () => {
        let values = await api.current?.validate();
        setConfirmLoading(true);
        try {
            const ret = await fetcher(`/bili/space/myinfo?user=${values?.value}`, undefined);
            if (ret.code) {
                console.log(ret);
                throw new Error(ret.message);
            }
            console.log(ret);
            await trigger({
                id: 0,
                name: values?.value,
                value: values?.value,
                platform: "bilibili-cookies"
            });
            setVisible(false);
            Toast.success('创建成功');
        } catch (e: any) {
            return Notification.error({
                title: '创建失败',
                content: <Typography.Paragraph style={{maxWidth: 450}}>{e.message}</Typography.Paragraph>,
                // theme: 'light',
                // duration: 0,
                style: {width: 'min-content'}
            });
        } finally {
            setConfirmLoading(false);
        }
    };
    const handleCancel = () => {
        setVisible(false);
        console.log('Cancel button clicked');
    };
    const handleAfterClose = () => {
        console.log('After Close callback executed');
    };
    const updateList = async (id: number) => {
        try {
            await deleteUser(id);
            Toast.success('删除成功');
        } catch (e: any) {
            Notification.error({
                title: '删除失败',
                content: <Typography.Paragraph style={{maxWidth: 450}}>{e.message}</Typography.Paragraph>,
                // theme: 'light',
                // duration: 0,
                style: {width: 'min-content'}
            });
        }
    };
    const api = useRef<FormApi>();
    return (<SideSheet
        title={<Typography.Title heading={4}>用户管理</Typography.Title>}
        visible={visible}
        footer={
            <div style={{display: 'flex', justifyContent: 'flex-end'}}>
                <Button onClick={showDialog} icon={<IconPlusCircle size="large"/>}
                        style={{marginRight: 4, backgroundColor: 'rgba(var(--semi-indigo-0), 1)'}}>
                    新增
                </Button>
            </div>}
        headerStyle={{borderBottom: '1px solid var(--semi-color-border)'}}
        bodyStyle={{borderBottom: '1px solid var(--semi-color-border)'}}
        onCancel={onCancel}>
        <List
            className='component-list-demo-booklist'
            dataSource={list}
            split={false}
            size='small'
            style={{flexBasis: '100%', flexShrink: 0}}
            renderItem={item =>
                <AvatarCard url={item.face} abbr={item.name} label={item.name} value={item.value}
                            onRemove={async () => await updateList(item.id)}/>
                // <div style={{ margin: 4 }} className='list-item'>
                //     <Button type='danger' theme='borderless' icon={<IconMinusCircle />} onClick={() => updateList(item)} style={{ marginRight: 4 }} />
                //     {item}
                // </div>
            }
        />

        <Modal
            title="新建"
            visible={modalVisible}
            onOk={handleOk}
            afterClose={handleAfterClose} //>=1.16.0
            onCancel={handleCancel}
            closeOnEsc={true}
            confirmLoading={confirmLoading}
            bodyStyle={{overflow: 'auto', maxHeight: 'calc(100vh - 320px)', paddingLeft: 10, paddingRight: 10}}
        >
            <Form getFormApi={formApi => api.current = formApi}>
                <Form.Input
                    field='value'
                    label="Cookie路径"
                    trigger='blur'
                    placeholder='cookies.json'
                    rules={[
                        {required: true},
                    ]}
                />
            </Form>
        </Modal>
    </SideSheet>);
}

export default UserList;
