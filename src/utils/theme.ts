import { ref, computed } from 'vue';

export type ThemeMode = 'dark' | 'light' | 'auto';
export type ResolvedTheme = 'dark' | 'light';

const STORAGE_KEY = 'vidnest_theme';

function getInitialTheme(): ThemeMode {
  const saved = localStorage.getItem(STORAGE_KEY) as ThemeMode | null;
  if (saved && ['dark', 'light', 'auto'].includes(saved)) {
    return saved;
  }
  return 'dark'; // Default to geek dark theme
}

export const themeMode = ref<ThemeMode>(getInitialTheme());
export const activeTheme = ref<ResolvedTheme>('dark');

function resolveSystemTheme(): ResolvedTheme {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'dark';
}

function applyTheme(resolved: ResolvedTheme) {
  activeTheme.value = resolved;
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', resolved);
    document.documentElement.classList.remove('theme-dark', 'theme-light');
    document.documentElement.classList.add(`theme-${resolved}`);
  }
}

export function updateResolvedTheme() {
  if (themeMode.value === 'auto') {
    applyTheme(resolveSystemTheme());
  } else {
    applyTheme(themeMode.value);
  }
}

export function setTheme(mode: ThemeMode) {
  themeMode.value = mode;
  try {
    localStorage.setItem(STORAGE_KEY, mode);
  } catch (e) {
    console.error('Failed to save theme to localStorage', e);
  }
  updateResolvedTheme();
}

export function toggleTheme() {
  if (activeTheme.value === 'dark') {
    setTheme('light');
  } else {
    setTheme('dark');
  }
}

export function initTheme() {
  updateResolvedTheme();

  // Listen to OS theme changes if user selects 'auto'
  if (typeof window !== 'undefined' && window.matchMedia) {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    mediaQuery.addEventListener('change', () => {
      if (themeMode.value === 'auto') {
        updateResolvedTheme();
      }
    });
  }
}

export const isDark = computed(() => activeTheme.value === 'dark');
