import type { LinchDesktopConfig } from "@linch-tech/desktop-core"
import { zh, en } from "./locales"

export const config: Partial<LinchDesktopConfig> = {
  brand: {
    name: "app.name",
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
    defaultLanguage: "zh",
    supportedLanguages: ["zh", "en"],
    resources: {
      zh,
      en,
    },
  },
}
