import { ref, computed } from 'vue';
import zhCN from './locales/zh-CN';
import enUS from './locales/en-US';

export type SupportedLocale = 'zh-CN' | 'en-US';

const STORAGE_KEY = 'vidnest_locale';

// Auto detect system or stored preference
function getInitialLocale(): SupportedLocale {
  const saved = localStorage.getItem(STORAGE_KEY) as SupportedLocale | null;
  if (saved && (saved === 'zh-CN' || saved === 'en-US')) {
    return saved;
  }
  if (typeof navigator !== 'undefined') {
    const lang = (navigator.language || '').toLowerCase();
    if (lang.startsWith('zh')) {
      return 'zh-CN';
    }
  }
  return 'en-US';
}

export const currentLocale = ref<SupportedLocale>(getInitialLocale());

const messages: Record<SupportedLocale, any> = {
  'zh-CN': zhCN,
  'en-US': enUS,
};

/**
 * Translation helper supporting dot-notation keys and parameter interpolation:
 * e.g. t('update.upToDateDesc', { version: '1.1.0' })
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const parts = key.split('.');
  const activeLang = currentLocale.value;

  let current: any = messages[activeLang];
  for (const p of parts) {
    if (current && typeof current === 'object' && p in current) {
      current = current[p];
    } else {
      current = undefined;
      break;
    }
  }

  // Fallback to zh-CN if translation is missing
  if (current === undefined && activeLang !== 'zh-CN') {
    let fallback: any = messages['zh-CN'];
    for (const p of parts) {
      if (fallback && typeof fallback === 'object' && p in fallback) {
        fallback = fallback[p];
      } else {
        fallback = undefined;
        break;
      }
    }
    current = fallback;
  }

  let text = typeof current === 'string' ? current : key;

  if (params && typeof text === 'string') {
    for (const [paramKey, paramVal] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${paramKey}\\}`, 'g'), String(paramVal));
    }
  }

  return text;
}

export function setLocale(locale: SupportedLocale) {
  currentLocale.value = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
    document.documentElement.setAttribute('lang', locale);
  } catch (e) {
    console.error('Failed to save locale to localStorage', e);
  }
}

export function toggleLocale() {
  const next = currentLocale.value === 'zh-CN' ? 'en-US' : 'zh-CN';
  setLocale(next);
}

export const isZh = computed(() => currentLocale.value === 'zh-CN');
