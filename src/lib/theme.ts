import { theme } from "./store";
import { get } from "svelte/store";

const THEME_KEY = 'pku-art-theme';

export function initTheme() {
  // Try to get saved theme from localStorage
  const savedTheme = localStorage.getItem(THEME_KEY) as 'light' | 'dark' | null;
  
  if (savedTheme) {
    theme.set(savedTheme);
  } else {
    // Use system preference
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    theme.set(prefersDark ? 'dark' : 'light');
  }
  
  // Subscribe to theme changes
  theme.subscribe((value) => {
    localStorage.setItem(THEME_KEY, value);
    document.documentElement.setAttribute('data-theme', value);
  });
}

export function toggleTheme() {
  const current = get(theme);
  theme.set(current === 'light' ? 'dark' : 'light');
}
