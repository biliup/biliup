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
    const auth = localStorage.getItem('auth');
    fetch('/api/basic', {
      headers: {
        'Authorization': `Basic ${auth}` // 使用测试凭据
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
      background: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
      backgroundSize: 'cover',
      ['--semi-color-text-0' as any]: 'rgba(0, 0, 0, 1)', 
      ['--semi-color-text-1' as any]: 'rgba(0, 0, 0, 0.3)',
      ['--semi-color-text-2' as any]: 'rgba(0, 0, 0, 0.3)',
    }}>
      <Card 
        shadows='always'
        style={{ 
          width: 400,
          borderRadius: 16,
          padding: 24,
          backdropFilter: 'blur(10px)',
          backgroundColor: 'rgba(255, 255, 255, 0.85)',
          boxShadow: '0 8px 32px rgba(0, 0, 0, 0.1)',
          border: '1px solid rgba(255, 255, 255, 0.18)',
        }}
      >
        <div style={{ textAlign: 'center', marginBottom: 30 }}>
          <Typography.Title heading={2} style={{ color: '#333' }}>Biliup 登录</Typography.Title>
        </div>
        <Form onSubmit={handleSubmit}>
          {({ formState, values, formApi }) => (
            <>
              <Form.Input
                field="username"
                label="用户名"
                prefix={<IconUser style={{ color: '#667eea' }} />}
                placeholder="请输入用户名"
                rules={[{ required: true, message: '请输入用户名' }]}
                style={{ marginBottom: 20 }}
              />
              <Form.Input
                field="password"
                label="密码"
                mode="password"
                prefix={<IconLock style={{ color: '#667eea' }} />}
                placeholder="请输入密码"
                rules={[{ required: true, message: '请输入密码' }]}
                style={{ marginBottom: 30 }}
              />
              <Button 
                loading={loading}
                type="primary" 
                htmlType="submit" 
                block
                style={{
                  backgroundColor: '#667eea',
                  borderColor: '#667eea',
                  borderRadius: 8,
                  padding: '12px 0',
                  fontSize: 16,
                  fontWeight: 500,
                  color: 'white',
                }}
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