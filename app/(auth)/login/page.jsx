'use client'

import React, {useState, useRef} from 'react';
import {Nav, Avatar, Form, Checkbox, Button, Toast} from '@douyinfe/semi-ui';
import { IconSemiLogo, IconFeishuLogo, IconHelpCircle, IconBell } from '@douyinfe/semi-icons';
import styles from './index.module.scss';
import {API_BASE} from "../../lib/api-streamer";
import useSWR from "swr";

const fetcher = (url) => fetch(API_BASE + url).then(res => {
    if (!res.ok) {
        throw new Error('not found');
    }
    return res;
});

const Component = () => {
    const [username, setUsername] = useState('biliup');
    const [password, setPassword] = useState('');
    const [remember, setRemember] = useState(false);
    const [loading, setLoading] = useState(false);
    const formApi = useRef(null);

    // 使用 SWR 检查用户是否存在
    const { data, error, isLoading } = useSWR('/v1/users/biliup', fetcher, {
        revalidateOnFocus: false,
        revalidateOnReconnect: false,
        shouldRetryOnError: false
    });

    // 判断是否为注册模式
    const isRegisterMode = error?.message === 'not found';

    // 处理表单提交
    const handleSubmit = async () => {
        if (!username || !password) {
            Toast.error('请填写用户名和密码');
            return;
        }

        // 提交前执行表单规则校验（注册模式下包含确认密码一致性），
        // 校验失败时 Semi 会在对应字段下方展示错误信息
        try {
            await formApi.current?.validate();
        } catch {
            return;
        }

        setLoading(true);

        try {
            const endpoint = isRegisterMode ? '/v1/users/register' : '/v1/users/login';
            const response = await fetch(API_BASE + endpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    username,
                    password,
                    ...(remember && { remember })
                }),
            });

            if (response.ok) {
                Toast.success(isRegisterMode ? '注册成功' : '登录成功');
                const urlParams = new URLSearchParams(window.location.search);
                const nextPath = urlParams.get('next');
                if (nextPath) {
                    try {
                        // URLSearchParams has already decoded the value. Only
                        // allow a same-origin destination so a crafted login URL
                        // cannot become an open redirect after authentication.
                        const destination = new URL(nextPath, window.location.origin);
                        if (destination.origin === window.location.origin) {
                            window.location.assign(
                                `${destination.pathname}${destination.search}${destination.hash}`
                            );
                            return;
                        }
                    } catch {
                        // Fall through to the safe default route.
                    }
                }
                window.location.assign('/');
            } else {
                const result = await response.json();
                Toast.error(result.message || (isRegisterMode ? '注册失败' : '登录失败'));
            }
        } catch (err) {
            Toast.error('网络错误，请稍后重试');
            console.error('Submit error:', err);
        } finally {
            setLoading(false);
        }
    };

    // 加载中状态
    if (isLoading) {
        return (
            <div className={styles.frame}>
                <div className={styles.main}>
                    <div className={styles.login}>
                        <div style={{ textAlign: 'center', padding: '50px' }}>
                            加载中...
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className={styles.frame}>
            <div className={styles.main}>
                <div className={styles.login}>
                    <div className={styles.component66}>
                        <img
                            src="/logo.png"
                            className={styles.logo}
                            alt="logo"
                        />
                        <div className={styles.header}>
                            <p className={styles.title}>
                                {isRegisterMode ? '欢迎注册' : '欢迎回来'}
                            </p>
                            <p className={styles.text3}>
                                <span className={styles.text}>
                                    {isRegisterMode ? '注册' : '登录'}
                                </span>
                                <span className={styles.text2}>&nbsp;biliup&nbsp;</span>
                                <span className={styles.text}>账户</span>
                            </p>
                        </div>
                    </div>
                    <div className={styles.form}>
                        <Form className={styles.inputs} getFormApi={(api) => (formApi.current = api)}>
                            <Form.Input
                                label={{ text: "用户名" }}
                                field="username"
                                fieldStyle={{ padding: 0 }}
                                style={{ width: 440 }}
                                className={styles.formField}
                                value={username}
                                initValue='biliup'
                                disabled
                            />
                            <Form.Input
                                label={{ text: "密码" }}
                                field="password"
                                type="password"
                                placeholder={isRegisterMode ? "设置密码" : "输入密码"}
                                fieldStyle={{ padding: 0 }}
                                style={{ width: 440 }}
                                className={styles.formField}
                                value={password}
                                onChange={setPassword}
                            />
                            {isRegisterMode && (
                                <Form.Input
                                    label={{ text: "确认密码" }}
                                    field="confirmPassword"
                                    type="password"
                                    placeholder="再次输入密码"
                                    fieldStyle={{ padding: 0 }}
                                    style={{ width: 440 }}
                                    className={styles.formField}
                                    rules={[
                                        { required: true, message: '请再次输入密码' },
                                        {
                                            validator: (rule, value) => value === password,
                                            message: '两次密码输入不一致'
                                        }
                                    ]}
                                />
                            )}
                        </Form>
                        {!isRegisterMode && (
                            <Checkbox
                                type="default"
                                className={styles.checkbox}
                                checked={remember}
                                onChange={(e) => setRemember(e.target.checked)}
                            >
                                记住我
                            </Checkbox>
                        )}
                        <Button
                            theme="solid"
                            block
                            loading={loading}
                            onClick={handleSubmit}
                        >
                            {isRegisterMode ? '注册' : '登录'}
                        </Button>
                        {isRegisterMode && (
                            <div style={{ marginTop: '16px', textAlign: 'center', color: '#666' }}>
                                <span style={{ fontSize: '14px' }}>
                                    注册即表示同意
                                    <a href="/terms" style={{ color: '#1890ff' }}>用户协议</a>
                                    和
                                    <a href="/privacy" style={{ color: '#1890ff' }}>隐私政策</a>
                                </span>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}

export default Component;
