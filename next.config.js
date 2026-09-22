/** @type {import('next').NextConfig} */
const nextConfig = {
    // reactStrictMode: true,
    output: 'export',
    images: {
        unoptimized: true
    },
    // Next 16.3 起 `next dev` 会在仓库根目录自动生成 AGENTS.md / CLAUDE.md,这里关掉以保持工作树干净
    agentRules: false,
}

module.exports = nextConfig
