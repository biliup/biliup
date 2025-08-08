"use client";

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { useRouter } from 'next/navigation';

interface AuthContextType {
  isAuthenticated: boolean;
  login: (username: string, password: string) => Promise<boolean>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const router = useRouter();

  useEffect(() => {
    // 检查是否存在保存的认证信息
    const auth = localStorage.getItem('auth');
    if (auth) {
      // 验证凭据是否仍然有效
      verifyCredentials(auth).then(valid => {
        setIsAuthenticated(valid);
        if (!valid) {
          localStorage.removeItem('auth');
        }
      });
    } else {
      // 如果没有保存的认证信息，检查是否需要认证
      checkAuthRequired().then(required => {
        if (!required) {
          // 如果不需要认证，设置为已认证状态
          setIsAuthenticated(true);
        }
      });
    }
  }, []);

  const checkAuthRequired = async () => {
    try {
      const auth = localStorage.getItem('auth');
      const response = await fetch('/api/basic', {
        headers: {
          'Authorization': `Basic ${auth}` // 使用测试凭据
        }
      });
      // 如果返回401，说明需要认证
      return response.status === 401;
    } catch (error) {
      // 如果发生错误，默认需要认证
      return true;
    }
  };

  const verifyCredentials = async (credentials: string) => {
    try {
      const response = await fetch('/api/basic', {
        headers: {
          'Authorization': `Basic ${credentials}`
        }
      });
      return response.ok;
    } catch (error) {
      return false;
    }
  };

  const login = async (username: string, password: string) => {
    try {
      const credentials = btoa(`${username}:${password}`);
      const valid = await verifyCredentials(credentials);
      
      if (valid) {
        localStorage.setItem('auth', credentials);
        setIsAuthenticated(true);
        return true;
      }
      return false;
    } catch (error) {
      return false;
    }
  };

  const logout = () => {
    localStorage.removeItem('auth');
    setIsAuthenticated(false);
    // 检查是否真的需要登录页面
    checkAuthRequired().then(required => {
      if (required) {
        router.push('/login');
      } else {
        router.push('/');
      }
    });
  };

  return (
    <AuthContext.Provider value={{ isAuthenticated, login, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}