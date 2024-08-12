'use client'
import {Button, ButtonGroup, Layout, List, Modal, Nav, Popconfirm, Typography} from "@douyinfe/semi-ui";
import {
    IconDeleteStroked,
    IconEdit2Stroked,
    IconHelpCircle,
    IconPlusCircle, IconUserCardVideo,
    IconVideoListStroked
} from "@douyinfe/semi-icons";
import TemplateModal from "../ui/TemplateModal";
import { Table, Avatar } from '@douyinfe/semi-ui';
import { IconMore } from '@douyinfe/semi-icons';
import { ColumnProps } from "@douyinfe/semi-ui/lib/es/table/interface";
import { SortOrder } from "@douyinfe/semi-ui/lib/es/table";
import useSWR from "swr";
import { fetcher, FileList, StudioEntity } from "@/app/lib/api-streamer";
import { useState } from "react";
import Player from 'xgplayer';
import 'xgplayer/dist/index.min.css';
import FlvPlugin from "xgplayer-flv";
import FlvJsPlugin from 'xgplayer-flv.js'
import Players from "@/app/ui/Player";
import { humDate } from "@/app/lib/utils";

export default function Home() {
    const { Header, Footer, Sider, Content } = Layout;
    const { data: data, error, isLoading } = useSWR<FileList[]>("/v1/videos", fetcher);
    const { Text } = Typography;
    const [fileName , setFileName] = useState<string>();
    const columns = [
        {
            title: '标题',
            dataIndex: 'name',
            render: (text: any, record: any, index: any) => {
                return (
                    <Text strong>{text}</Text>
                );
            }
            // onFilter: (value, record) => record.name.includes(value)
        },
        {
            title: '大小',
            dataIndex: 'size',
            render: (size: number) => `${(size / 1024 / 1024).toFixed(2)} MB`,
        },
        {
            title: '更新日期',
            dataIndex: 'updateTime',
            defaultSortOrder: 'descend' as SortOrder,
            sorter: (a: any, b: any) => (a.updateTime - b.updateTime > 0 ? 1 : -1),
            render: (time: number) => humDate(time),
        },
        {
            title: '',
            dataIndex: 'operate',
            render: (text: any, record: any, index: number) => {
                return <IconUserCardVideo style={{cursor: 'pointer'}} onClick={() => showDialog(record.name)}/>;
            },
        },
    ];
    const [visible, setVisible] = useState(false);
    const showDialog = (name: string) => {
        setVisible(true);
        setFileName(name);
        // setTimeout(()=>{
        //     let player = new Player({
        //           id: 'mse',
        //           url: (process.env.NEXT_PUBLIC_API_SERVER ?? '') + '/static/' + name,
        //           height: '100%',
        //           // plugins: [FlvPlugin],
        //           plugins: [FlvJsPlugin],
        //           width: '100%',
        //         });
        // }, 500)

    };
    const handleCancel = () => {
        setVisible(false);
        console.log('Cancel button clicked');
    };
    return (<>
        <Header style={{ backgroundColor: 'var(--semi-color-bg-1)' }}>
            <Nav style={{border: 'none'}}
                header={<><div style={{
                    backgroundColor: 'rgba(var(--semi-green-4), 1)',
                    borderRadius: 'var(--semi-border-radius-large)',
                    color: 'var(--semi-color-bg-0)',
                    display: 'flex',
                    // justifyContent: 'center',
                    padding: '6px'
                }}><IconVideoListStroked size='large' /></div><h4 style={{ marginLeft: '12px' }}>历史记录</h4></>}
                mode="horizontal"
            ></Nav>
        </Header>
        <Content
            style={{
                paddingLeft: 12,
                paddingRight: 12,
                backgroundColor: 'var(--semi-color-bg-0)',
            }}
        >
            <main>
                <Table size="small" columns={columns} dataSource={data} />
            </main>
            <Modal
                visible={visible}
                onCancel={handleCancel}
                closeOnEsc={true}
                size='large'
                bodyStyle={{height: 500}}
                footer={null}
            >
                <Players url={(process.env.NEXT_PUBLIC_API_SERVER ?? '') + '/static/' + fileName}></Players>
                <div id="mse"></div>
            </Modal>
        </Content>
    </>);
}
