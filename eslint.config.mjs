import { defineConfig, globalIgnores } from 'eslint/config'
import nextVitals from 'eslint-config-next/core-web-vitals'
import prettier from 'eslint-config-prettier/flat'

export default defineConfig([
  ...nextVitals,
  prettier,
  {
    rules: {
      // eslint-plugin-react-hooks 7 新增的 React Compiler 系列规则，默认为 error。
      // 现有代码里有十余处「effect 内同步 setState」等历史写法，与依赖升级无关，
      // 先降为 warn 保留提示，待专门的重构 PR 逐个处理后再恢复为 error。
      'react-hooks/set-state-in-effect': 'warn',
      'react-hooks/static-components': 'warn',
      'react-hooks/purity': 'warn',
      'react-hooks/immutability': 'warn',
    },
  },
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
