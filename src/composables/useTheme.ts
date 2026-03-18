import { ref, watch, onMounted } from 'vue';

export type ThemeMode = 'light' | 'dark' | 'system';

const THEME_KEY = 'narrative-loom-theme';

// Global reactive theme state
const currentTheme = ref<ThemeMode>('system');
const resolvedTheme = ref<'light' | 'dark'>('light');

function getSystemTheme(): 'light' | 'dark' {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'light';
}

function applyTheme(theme: 'light' | 'dark') {
  if (typeof document !== 'undefined') {
    const root = document.documentElement;
    if (theme === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }
  resolvedTheme.value = theme;
}

function updateTheme() {
  if (currentTheme.value === 'system') {
    applyTheme(getSystemTheme());
  } else {
    applyTheme(currentTheme.value);
  }
}

export function useTheme() {
  // Initialize theme from localStorage
  onMounted(() => {
    const savedTheme = localStorage.getItem(THEME_KEY) as ThemeMode | null;
    if (savedTheme && ['light', 'dark', 'system'].includes(savedTheme)) {
      currentTheme.value = savedTheme;
    }
    updateTheme();

    // Listen for system theme changes
    if (typeof window !== 'undefined' && window.matchMedia) {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      mediaQuery.addEventListener('change', () => {
        if (currentTheme.value === 'system') {
          updateTheme();
        }
      });
    }
  });

  // Watch for theme changes
  watch(currentTheme, (newTheme) => {
    localStorage.setItem(THEME_KEY, newTheme);
    updateTheme();
  });

  function setTheme(theme: ThemeMode) {
    currentTheme.value = theme;
  }

  return {
    theme: currentTheme,
    resolvedTheme,
    setTheme,
  };
}

// Initialize theme immediately for SSR/hydration
if (typeof window !== 'undefined') {
  const savedTheme = localStorage.getItem(THEME_KEY) as ThemeMode | null;
  if (savedTheme && ['light', 'dark', 'system'].includes(savedTheme)) {
    currentTheme.value = savedTheme;
  }
  updateTheme();
}
