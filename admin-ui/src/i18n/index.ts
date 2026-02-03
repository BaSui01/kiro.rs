import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

import zhCN from './zh-CN.json';
import enUS from './en-US.json';
import jaJP from './ja-JP.json';

// 语言资源
const resources = {
  'zh-CN': {
    translation: zhCN,
  },
  'en-US': {
    translation: enUS,
  },
  'ja-JP': {
    translation: jaJP,
  },
};

// 初始化 i18next
i18n
  .use(LanguageDetector) // 自动检测浏览器语言
  .use(initReactI18next) // 传递 i18n 实例给 react-i18next
  .init({
    resources,
    fallbackLng: 'zh-CN', // 默认语言
    debug: false, // 开发环境可以设置为 true

    // 语言检测选项
    detection: {
      order: ['localStorage', 'navigator'], // 优先从 localStorage 读取，然后是浏览器设置
      caches: ['localStorage'], // 缓存用户选择的语言
      lookupLocalStorage: 'kiro-admin-language', // localStorage 的 key
    },

    interpolation: {
      escapeValue: false, // React 已经处理了 XSS
    },

    // 支持的语言列表
    supportedLngs: ['zh-CN', 'en-US', 'ja-JP'],

    // 非严格模式，允许回退到相似语言
    nonExplicitSupportedLngs: true,
  });

export default i18n;
