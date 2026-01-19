import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'Kuse Cowork',
  description: 'Open-source AI Agent Framework for Desktop',
  base: '/kuse_cowork/',

  head: [
    ['link', { rel: 'icon', href: '/kuse_cowork/favicon.ico' }]
  ],

  themeConfig: {
    logo: '/logo.svg',

    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/getting-started/installation' },
      { text: 'Features', link: '/features/overview' },
      { text: 'Architecture', link: '/architecture/overview' }
    ],

    sidebar: {
      '/': [
        {
          text: 'Getting Started',
          items: [
            { text: 'Installation', link: '/getting-started/installation' },
            { text: 'Quick Start', link: '/getting-started/quickstart' },
            { text: 'Configuration', link: '/getting-started/configuration' }
          ]
        },
        {
          text: 'Features',
          items: [
            { text: 'Overview', link: '/features/overview' },
            { text: 'AI Providers', link: '/features/providers' },
            { text: 'Agent System', link: '/features/agent' },
            { text: 'Tools', link: '/features/tools' },
            { text: 'Skills', link: '/features/skills' },
            { text: 'MCP Protocol', link: '/features/mcp' }
          ]
        },
        {
          text: 'Architecture',
          items: [
            { text: 'Overview', link: '/architecture/overview' },
            { text: 'Frontend', link: '/architecture/frontend' },
            { text: 'Backend', link: '/architecture/backend' }
          ]
        },
        {
          text: 'Development',
          items: [
            { text: 'Setup', link: '/development/setup' },
            { text: 'Contributing', link: '/development/contributing' },
            { text: 'Building', link: '/development/building' }
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/kuse-ai/kuse_cowork' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026 Kuse AI'
    },

    search: {
      provider: 'local'
    },

    editLink: {
      pattern: 'https://github.com/kuse-ai/kuse_cowork/edit/main/docs/:path',
      text: 'Edit this page on GitHub'
    }
  }
})
