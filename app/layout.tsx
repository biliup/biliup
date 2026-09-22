'use client'
// React 19 移除了 react-dom 上的 render/createRoot,Semi 的 Toast / Notification / Spin 等
// 命令式渲染需要先注入 react-dom/client 的 createRoot,必须在任何 Semi 组件之前导入
import '@douyinfe/semi-ui/react19-adapter'
import './globals.css'

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="zh-Hans">
      <head>
        {/* 首屏前置设置主题，避免水合期从默认主题闪到已保存主题 */}
        <script
          dangerouslySetInnerHTML={{
            __html: `(function(){try{var m=localStorage.getItem('mode')||'auto';var s=window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light';var t=m==='auto'?s:m;document.documentElement.setAttribute('theme-mode',t);}catch(e){}})();`,
          }}
        />
      </head>
      <body style={{ width: '100%' }}>
        {/*
          同步 <head> 脚本设到 <html> 上的 theme-mode 到 <body>。
          Semi Design 的 CSS 变量绑定在 body[theme-mode] 上，
          而 <head> 阻塞脚本执行时 body 尚未解析，只能写 html。
        */}
        <script
          dangerouslySetInnerHTML={{
            __html: `(function(){try{var t=document.documentElement.getAttribute('theme-mode')||'light';document.body.setAttribute('theme-mode',t);}catch(e){}})();`,
          }}
        />
        {children}
      </body>
    </html>
  )
}