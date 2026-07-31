'use client'
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
        {children}
      </body>
    </html>
  )
}