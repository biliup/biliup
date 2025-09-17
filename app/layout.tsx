'use client'
import './globals.css'

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="zh-Hans">
      <body style={{ width: '100%' }}>
        {children}
      </body>
    </html>
  )
}