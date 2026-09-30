import { defineConfig, globalIgnores } from 'eslint/config'
import nextVitals from 'eslint-config-next/core-web-vitals'
import prettier from 'eslint-config-prettier/flat'

export default defineConfig([
  ...nextVitals,
  prettier,
  {
    files: ['app/**/*.{ts,tsx}'],
    ignores: ['app/ui/shell/**'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: '@douyinfe/semi-ui',
              importNames: ['Modal', 'SideSheet'],
              message:
                '弹窗和抽屉请用 app/ui/shell 的 FormDialog / FormSheet：页面、抽屉、弹窗的归属规则与统一底栏写在 app/ui/shell/sizes.ts',
            },
          ],
        },
      ],
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
