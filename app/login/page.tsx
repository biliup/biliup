"use client";

import { Form, Button, Card, Toast, Typography } from "@douyinfe/semi-ui";
import { IconUser, IconLock } from "@douyinfe/semi-icons";
import { useAuth } from '../lib/auth-context';
import { useRouter } from "next/navigation";
import { useState, useEffect } from "react";

export default function LoginPage() {
  const { login } = useAuth();
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  
  useEffect(() => {
    // 检查是否需要认证
    fetch('/api/basic', {
      headers: {
        'Authorization': 'Basic dGVzdDp0ZXN0' // 使用测试凭据
      }
    }).then(response => {
      if (response.status !== 401) {
        // 如果不需要认证，直接跳转到主页
        router.push('/');
      }
    }).catch(() => {
      // 如果发生错误，默认需要认证，保持在登录页面
    });
  }, [router]);
  
  const handleSubmit = async (values: { username: string; password: string }) => {
    setLoading(true);
    try {
      const success = await login(values.username, values.password);
      
      if (success) {
        Toast.success('登录成功');
        router.push('/');
      } else {
        Toast.error('用户名或密码错误');
      }
    } catch (error) {
      Toast.error('登录失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      height: '100vh',
      backgroundColor: 'var(--semi-color-bg-0)',
    }}>
      <Card 
        shadows='hover'
        style={{ width: 400 }}
      >
        <div style={{ textAlign: 'center', marginBottom: 20 }}>
          <Typography.Title heading={3}>Biliup 登录</Typography.Title>
        </div>
        <Form onSubmit={handleSubmit}>
          {({ formState, values, formApi }) => (
            <>
              <Form.Input
                field="username"
                label="用户名"
                prefix={<IconUser />}
                placeholder="请输入用户名"
                rules={[{ required: true, message: '请输入用户名' }]}
              />
              <Form.Input
                field="password"
                label="密码"
                mode="password"
                prefix={<IconLock />}
                placeholder="请输入密码"
                rules={[{ required: true, message: '请输入密码' }]}
              />
              <Button 
                loading={loading}
                type="primary" 
                htmlType="submit" 
                block
              >
                登录
              </Button>
            </>
          )}
        </Form>
      </Card>
    </div>
  );
}