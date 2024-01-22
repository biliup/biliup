'use client'
import {
    Button,
    ButtonGroup,
    Layout,
    List,
    Nav,
    Popconfirm,
    Notification,
} from "@douyinfe/semi-ui";
import {
    IconCloudStroked,
    IconPlusCircle,
    IconUserListStroked
} from "@douyinfe/semi-icons";
import { useState } from "react";
import Link from "next/link";
import { Card } from '@douyinfe/semi-ui';
import { IconEdit2Stroked, IconSendStroked, IconDeleteStroked } from '@douyinfe/semi-icons';
import {fetcher, requestDelete, StudioEntity} from "../lib/api-streamer";
import useSWR from "swr";
import {useRouter} from "next/navigation";
import UserList from "../ui/UserList";
import useSWRMutation from "swr/mutation";
import {useBiliUsers} from "../lib/use-streamers";

export default function Union() {
    const { Meta } = Card;
    const { Header, Content } = Layout;
    const [visible, setVisible] = useState(false);
    const router = useRouter();
    const {trigger: deleteUpload} = useSWRMutation('/v1/upload/streamers', requestDelete);
    const { data: templates, error, isLoading } = useSWR<StudioEntity[]>("/v1/upload/streamers", fetcher);
    const {biliUsers} = useBiliUsers();
    const handleAddLinkClick = (event: React.MouseEvent) => {
        if (biliUsers.length === 0) {
            event.preventDefault(); // 阻止Link的默认跳转事件
            change(); // 运行change函数
            Notification.info({
                title: '用户列表为空',
                position: 'top',
                content: '请先在右侧点击新增用户',
                duration: 3,
            })
        }
    };

    const change = () => {
        setVisible(!visible);
    };
    const onConfirm = async (id: number) => {
        await deleteUpload(id);
    }
    return (<>
        <UserList visible={visible} onCancel={change}></UserList>
        <Header style={{ backgroundColor: 'var(--semi-color-bg-1)' }}>
            <Nav
                header={<><div style={{
                    backgroundColor: 'rgba(var(--semi-violet-4), 1)',
                    borderRadius: 'var(--semi-border-radius-large)',
                    color: 'var(--semi-color-bg-0)',
                    display: 'flex',
                    // justifyContent: 'center',
                    padding: '6px'
                }}><IconCloudStroked size='large' /></div><h4 style={{ marginLeft: '12px' }}>投稿管理</h4></>}
                mode="horizontal"
                footer={<>
                    <Button
                        onClick={change}
                        // theme="borderless"
                        type="tertiary"
                        icon={<IconUserListStroked />}
                        style={{
                            // color: 'var(--semi-color-text-2)',
                            borderRadius: 'var(--semi-border-radius-circle)',
                            marginRight: '12px',
                        }} />
                    <Link href='/upload-manager/add' onClick={handleAddLinkClick}>
                        <Button icon={<IconPlusCircle />} theme="solid" style={{ marginRight: 10 }}>新建</Button>
                    </Link>

                </>}
            ></Nav>
        </Header>
        <Content
            style={{
                padding: '24px',
                backgroundColor: 'var(--semi-color-bg-0)'
            }}
        >
            <List grid={{
                gutter: 12,
                xs: 24,
                sm: 24,
                md: 12,
                lg: 8,
                xl: 6,
                xxl: 4,
            }}
                dataSource={templates}
                renderItem={item => <List.Item>
                    <Card
                        shadows='hover'
                        style={{
                            maxWidth: 360,
                            margin: '8px 2px',
                            flexGrow: 1
                        }}
                        bodyStyle={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between'
                        }}
                    >
                        <Meta
                            title={item.template_name}
                        />
                        <ButtonGroup theme='borderless'>
                            <Button icon={<IconSendStroked />}></Button>
                            <Button icon={<IconEdit2Stroked />} onClick={()=> {
                                router.push(`/upload-manager/edit?id=${item.id}`);
                            }}></Button>
                            <Popconfirm
                                title="确定是否要删除？"
                                content="此操作将不可逆"
                                margin={50}
                                onConfirm={async () => await onConfirm(item.id)}
                                // onCancel={onCancel}
                            >
                                <Button theme='borderless' icon={<IconDeleteStroked />}></Button>
                            </Popconfirm>
                        </ButtonGroup>
                    </Card>
                </List.Item>}
            />
        </Content>
    </>);
}

