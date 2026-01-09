import type { LinchDesktopConfig } from '@linch-tech/desktop-core';

export const config: Partial<LinchDesktopConfig> = {
  brand: {
    name: 'app.name',
    version: `v${__APP_VERSION__}`,
  },

  // 单页编辑器应用，不需要导航
  nav: [],

  features: {
    updater: true,
    database: false, // 使用自己的配置系统
    sentry: false,
  },

  layout: {
    sidebar: {
      width: 0, // 不使用默认侧边栏
    },
  },

  i18n: {
    defaultLanguage: 'zh',
    supportedLanguages: ['zh', 'en'],
    resources: {
      en: {
        app: {
          name: 'Linch Redact',
          description: 'PDF Sensitive Content Redaction Tool',
          dragDropHint: 'Drop PDF files here',
        },
      },
      zh: {
        app: {
          name: 'Linch Redact',
          description: 'PDF 敏感信息遮盖工具',
          dragDropHint: '释放以添加 PDF 文件',
        },
      },
    },
  },
};
