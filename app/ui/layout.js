import {Button, Layout as SeLayout} from '@douyinfe/semi-ui';
import {Nav} from '@douyinfe/semi-ui';
import {
    IconMoon,
    IconSun,
    IconCloudStroked,
    IconSemiLogo,
    IconUserCardVideo,
    IconDoubleChevronLeft,
    IconDoubleChevronRight, IconVideoListStroked
} from '@douyinfe/semi-icons';
import Link from "next/link";
import {useCallback, useMemo, useState} from "react";
import styles from '../styles/Home.module.css';
import { useRouter } from 'next/router';

export default function Layout({children}) {
    const {Header, Footer, Sider, Content} = SeLayout;
    const router = useRouter();

    const renderWrapper = useCallback(({ itemElement, isSubNav, isInSubNav, props }) => {
        const routerMap = {
            home: "/",
            history: "/history",
            dashboard: "/dashboard",
            streamers: "/streamers",
            "upload-manager": "/upload-manager",
        };
        if(!routerMap[props.itemKey]) {
            return itemElement;
        }
        return (
            <Link
                style={{ textDecoration: "none", fontWeight: "600 !important" }}
                href={routerMap[props.itemKey]}
            >
                {itemElement}
            </Link>
        );
        // return itemElement;
    }, []);
    let initOpenKeys = [];
    console.log("slice(1)", router.pathname.slice(1));
    if (router.pathname.slice(1) === 'streamers' || router.pathname.slice(1) ===  'history') {
        initOpenKeys = ['manager'];
    }
    const [openKeys, setOpenKeys] = useState(initOpenKeys);
    const [selectedKeys, setSelectedKeys] = useState([router.pathname.slice(1)]);
    const [isCollapsed, setIsCollapsed] = useState(false);
    const items = useMemo(() =>[
            {itemKey: 'manager', text: '录播管理',
                items: [{itemKey: 'streamers', text: '直播管理'}, {itemKey: 'history', text: '历史记录'}],
                icon: <div style={{
                    backgroundColor: '#5ac262ff',
                    borderRadius: 'var(--semi-border-radius-medium)',
                    color: 'var(--semi-color-bg-0)',
                    display: 'flex',
                    // justifyContent: 'center',
                    padding: '4px'
                }}><IconVideoListStroked size='small'/></div>},
            {itemKey: 'upload-manager', text: '投稿管理', icon: <div style={{
                    backgroundColor: '#885bd2ff',
                    borderRadius: 'var(--semi-border-radius-medium)',
                    color: 'var(--semi-color-bg-0)',
                    display: 'flex',
                    padding: '4px'
                }}><IconCloudStroked size='small' /></div>},
            // {itemKey: 'Job', text: '投稿管理', icon: <IconStar/>},
            // {
            //     text: '任务平台',
            //     icon: <IconSetting/>,
            //     itemKey: 'Job',
            //     items: [{itemKey: 'About', text: '任务管理'}, {itemKey: 'Dashboard', text: '用户任务查询'}],
            // },
        ].map(value => {
            value.text = <div style={{
                color: selectedKeys.some(key => value.itemKey === key) || (selectedKeys.some(key=> openKeys.some(o=> isSub(key, o))) && openKeys.some(key => value.itemKey === key)) ? 'var(--semi-color-text-0)' : 'var(--semi-color-text-2)',
                fontWeight: 600
            }}>{value.text}</div>
            return value;
        }), [openKeys, selectedKeys]);
    // const [navStyle, setNavStyle] = useState({ height: '100%' });
    const onSelect = data => {
        setSelectedKeys([...data.selectedKeys]);
    };
    const onOpenChange = data => {
        console.log('trigger onOpenChange: ', data);
        setOpenKeys([...data.openKeys]);
    };
    let navStyle = isCollapsed ? { height: '100%', overflow: 'visible' } : { height: '100%' };
    const onCollapseChange = useCallback( () => {
        setIsCollapsed(!isCollapsed);
    }, [isCollapsed]);
    const [mode, setMode] = useState('light');

    return (
        <>
            <SeLayout className="components-layout-demo semi-light-scrollbar">
                <Sider>
                    <Nav
                        style={navStyle}
                        // toggleIconPosition={'left'}
                        // defaultOpenKeys={['job']}
                        openKeys={openKeys}
                        selectedKeys={selectedKeys}
                        isCollapsed={isCollapsed}
                        // bodyStyle={{height: '100%'}}
                        renderWrapper={renderWrapper}
                        items={items}
                        // onCollapseChange={onCollapseChange}
                        onOpenChange={onOpenChange}
                        onSelect={onSelect}
                        // header={{
                        //     logo: <IconSemiLogo style={{height: '36px', fontSize: 36}}/>,
                        //     text: 'BILIUP'
                        // }}
                        // footer={{
                        //     collapseButton: true,
                        // }}
                    >
                        <Nav.Header
                            logo={<IconSemiLogo style={{height: '36px', fontSize: 36}}/>}
                            style={{justifyContent: 'flex-start'}}
                            text='BILIUP'
                        >
                            <div style={{flexGrow: 1, display: 'flex', flexDirection: 'row-reverse', alignSelf: 'flex-end'}}>
                                <Button
                                    onClick={onCollapseChange} type="tertiary"
                                    className={styles.shadow} theme='borderless'
                                    icon={isCollapsed
                                        ? <IconDoubleChevronRight /> :  <IconDoubleChevronLeft />}
                                />
                            </div>
                        </Nav.Header>
                        {footer(mode, setMode)}
                    </Nav>
                </Sider>
                <SeLayout style={{height: '100vh'}}>
                    {children}
                </SeLayout>
            </SeLayout>
        </>
    )
}

function footer(mode, setMode) {
    const switchMode = () => {
        const body = document.body;
        if (body.hasAttribute('theme-mode')) {
            body.removeAttribute('theme-mode');
            setMode('light');
        } else {
            body.setAttribute('theme-mode', 'dark');
            setMode('dark');
        }
    };
    return (
    <Nav.Footer collapseButton={true} >
        <Button
            onClick={switchMode}
            theme="borderless"
            icon=
                {mode === 'light'
                    ? <IconMoon size="large"/>
                    : <IconSun size="large" />
            }
            style={{
                color: 'var(--semi-color-text-2)',
                // marginRight: '12px',
            }}/>
    </Nav.Footer>);
}

function isSub(key1, key2) {
    const routerMap = {
        manager: ["streamers", "history"],
    };
    return routerMap[key2]?.includes(key1);
}
