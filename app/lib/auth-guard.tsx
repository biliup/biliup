"use client";

import { useAuth } from './auth-context';
import { useRouter } from 'next/navigation';
import { useEffect, useState } from 'react';
import { Spin } from '@douyinfe/semi-ui';

export function AuthGuard({ children }: { children: React.ReactNode }) {
  const { isAuthenticated } = useAuth();
  const router = useRouter();
  const [checked, setChecked] = useState(false);

  useEffect(() => {
    // 如果用户未认证，则检查是否需要认证
    if (!isAuthenticated) {
      // 检查是否需要认证
      const auth = localStorage.getItem('auth');
      fetch('/api/basic', {
        headers: {
          'Authorization': `Basic ${auth}` // 使用测试凭据
        }
      }).then(response => {
        if (response.status !== 401) {
          // 如果不需要认证，直接显示内容
          setChecked(true);
        } else {
          // 如果需要认证，重定向到登录页面
          router.push('/login');
        }
      }).catch(() => {
        // 如果发生错误，默认需要认证
        router.push('/login');
      });
    } else {
      setChecked(true);
    }
  }, [isAuthenticated, router]);

  // 如果用户已认证或不需要认证且已检查，显示子组件
  if ((isAuthenticated || checked) && checked) {
    return <>{children}</>;
  }

  // 如果用户未认证且正在检查，显示加载状态
  if (!isAuthenticated && !checked) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '100vh',
      }}>
        <Spin size="large" />
      </div>
    );
  }

  // 其他情况不显示内容
  return null;
}