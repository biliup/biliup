import { defineConfig, globalIgnores } from 'eslint/config'
import nextVitals from 'eslint-config-next/core-web-vitals'
import prettier from 'eslint-config-prettier/flat'

export default defineConfig([
  ...nextVitals,
  prettier,
  globalIgnores([
    // eslint-config-next 的默认忽略项
    '.next/**',
    'out/**',
    'build/**',
    'next-env.d.ts',
    // 独立的 Vite/Tauri 子项目与 Rust 产物，不属于 Next 前端
    'tauri-app/**',
    'target/**',
  ]),
])
