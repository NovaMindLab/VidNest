import { ref, computed } from 'vue';
import zhCN from './locales/zh-CN';
import enUS from './locales/en-US';
import jaJP from './locales/ja-JP';
import koKR from './locales/ko-KR';
import deDE from './locales/de-DE';
import esES from './locales/es-ES';

export type SupportedLocale = 'zh-CN' | 'en-US' | 'ja-JP' | 'ko-KR' | 'de-DE' | 'es-ES';
export type LocaleMode = 'auto' | SupportedLocale;

export interface LocaleOption {
  code: SupportedLocale;
  flag: string;
  name: string;
  nativeName: string;
  englishName: string;
}

export const SUPPORTED_LOCALES: LocaleOption[] = [
  { code: 'zh-CN', flag: '🇨🇳', name: '简体中文', nativeName: '简体中文', englishName: 'Chinese (Simplified)' },
  { code: 'en-US', flag: '🇺🇸', name: 'English', nativeName: 'English (US)', englishName: 'English (US)' },
  { code: 'ja-JP', flag: '🇯🇵', name: '日本語', nativeName: '日本語', englishName: 'Japanese' },
  { code: 'ko-KR', flag: '🇰🇷', name: '한국어', nativeName: '한국어', englishName: 'Korean' },
  { code: 'de-DE', flag: '🇩🇪', name: 'Deutsch', nativeName: 'Deutsch', englishName: 'German' },
  { code: 'es-ES', flag: '🇪🇸', name: 'Español', nativeName: 'Español', englishName: 'Spanish' },
];

const STORAGE_KEY_MODE = 'vidnest_locale_mode';
const STORAGE_KEY_LEGACY = 'vidnest_locale';

/**
 * Sniff OS/Browser preferred language
 */
export function detectSystemLocale(): SupportedLocale {
  if (typeof navigator !== 'undefined') {
    const lang = (navigator.language || '').toLowerCase();
    if (lang.startsWith('zh')) return 'zh-CN';
    if (lang.startsWith('ja')) return 'ja-JP';
    if (lang.startsWith('ko')) return 'ko-KR';
    if (lang.startsWith('de')) return 'de-DE';
    if (lang.startsWith('es')) return 'es-ES';
  }
  return 'en-US';
}

function getInitialLocaleMode(): LocaleMode {
  try {
    const savedMode = localStorage.getItem(STORAGE_KEY_MODE) as LocaleMode | null;
    if (savedMode && (savedMode === 'auto' || SUPPORTED_LOCALES.some(l => l.code === savedMode))) {
      return savedMode;
    }
    const legacy = localStorage.getItem(STORAGE_KEY_LEGACY) as SupportedLocale | null;
    if (legacy && SUPPORTED_LOCALES.some(l => l.code === legacy)) {
      return legacy;
    }
  } catch {
    // ignore storage access errors
  }
  return 'auto';
}

export const currentLocaleMode = ref<LocaleMode>(getInitialLocaleMode());

export function computeActiveLocale(mode: LocaleMode): SupportedLocale {
  if (mode === 'auto') {
    return detectSystemLocale();
  }
  return mode;
}

export const currentLocale = ref<SupportedLocale>(computeActiveLocale(currentLocaleMode.value));

const messages: Record<SupportedLocale, any> = {
  'zh-CN': zhCN,
  'en-US': enUS,
  'ja-JP': jaJP,
  'ko-KR': koKR,
  'de-DE': deDE,
  'es-ES': esES,
};

function resolveKey(dict: any, parts: string[]): any {
  let curr = dict;
  for (const p of parts) {
    if (curr && typeof curr === 'object' && p in curr) {
      curr = curr[p];
    } else {
      return undefined;
    }
  }
  return curr;
}

/**
 * Translation helper supporting dot-notation keys, cascading fallbacks, and parameter interpolation:
 * Fallback pipeline: activeLang -> en-US -> zh-CN -> rawKey
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const parts = key.split('.');
  const activeLang = currentLocale.value;

  // 1. Try active language
  let current = resolveKey(messages[activeLang], parts);

  // 2. Cascade fallback to en-US
  if (current === undefined && activeLang !== 'en-US') {
    current = resolveKey(messages['en-US'], parts);
  }

  // 3. Cascade fallback to zh-CN
  if (current === undefined && activeLang !== 'zh-CN') {
    current = resolveKey(messages['zh-CN'], parts);
  }

  let text = typeof current === 'string' ? current : key;

  if (params && typeof text === 'string') {
    for (const [paramKey, paramVal] of Object.entries(params)) {
      text = text.replace(new RegExp(`\\{${paramKey}\\}`, 'g'), String(paramVal));
    }
  }

  return text;
}

export function setLocaleMode(mode: LocaleMode) {
  currentLocaleMode.value = mode;
  const active = computeActiveLocale(mode);
  currentLocale.value = active;
  try {
    localStorage.setItem(STORAGE_KEY_MODE, mode);
    localStorage.setItem(STORAGE_KEY_LEGACY, active);
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('lang', active);
    }
  } catch (e) {
    console.error('Failed to save locale mode', e);
  }
}

export function setLocale(locale: SupportedLocale) {
  setLocaleMode(locale);
}

export function toggleLocale() {
  const list: SupportedLocale[] = ['zh-CN', 'en-US', 'ja-JP', 'ko-KR', 'de-DE', 'es-ES'];
  const currentIndex = list.indexOf(currentLocale.value);
  const nextIndex = (currentIndex + 1) % list.length;
  setLocaleMode(list[nextIndex]);
}

export const isZh = computed(() => currentLocale.value === 'zh-CN');

// Sync with system language changes if in 'auto' mode
if (typeof window !== 'undefined') {
  window.addEventListener('languagechange', () => {
    if (currentLocaleMode.value === 'auto') {
      currentLocale.value = detectSystemLocale();
      document.documentElement.setAttribute('lang', currentLocale.value);
    }
  });
}
