import { useEffect, useState } from 'react'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { cs, en, type Language, type MessageKey } from '../i18n'

export type ThemePreference = 'system' | 'light' | 'dark'
type Preferences = { language: Language; theme: ThemePreference }
const storageKey = 'warply.appearance'

function initialPreferences(): Preferences {
  const defaults: Preferences = { language: 'en', theme: 'system' }
  try {
    const saved = JSON.parse(
      localStorage.getItem(storageKey) ?? 'null',
    ) as Partial<Preferences> | null
    return {
      language: saved?.language === 'cs' ? 'cs' : 'en',
      theme:
        saved?.theme === 'light' || saved?.theme === 'dark'
          ? saved.theme
          : 'system',
    }
  } catch {
    return defaults
  }
}

export function usePreferences() {
  const [preferences, setPreferences] = useState(initialPreferences)
  const [systemDark, setSystemDark] = useState(
    () => window.matchMedia('(prefers-color-scheme: dark)').matches,
  )
  const resolvedTheme =
    preferences.theme === 'system'
      ? systemDark
        ? 'dark'
        : 'light'
      : preferences.theme

  useEffect(() => {
    const query = window.matchMedia('(prefers-color-scheme: dark)')
    const change = () => setSystemDark(query.matches)
    query.addEventListener('change', change)
    return () => query.removeEventListener('change', change)
  }, [])

  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme
    document.documentElement.lang = preferences.language === 'cs' ? 'cs' : 'en'
    try {
      localStorage.setItem(storageKey, JSON.stringify(preferences))
    } catch {
      /* Preferences remain available for this session. */
    }
    let active = true
    if (isTauri()) {
      void invoke<boolean>('set_window_appearance', {
        preference: preferences.theme,
        dark: resolvedTheme === 'dark',
      })
        .then((mica) => {
          if (active)
            document.documentElement.dataset.backdrop = mica ? 'mica' : 'solid'
        })
        .catch(() => {
          if (active) document.documentElement.dataset.backdrop = 'solid'
        })
    }
    return () => {
      active = false
    }
  }, [preferences, resolvedTheme])

  return {
    ...preferences,
    resolvedTheme,
    setLanguage: (language: Language) =>
      setPreferences((current) => ({ ...current, language })),
    setTheme: (theme: ThemePreference) =>
      setPreferences((current) => ({ ...current, theme })),
    t: (key: MessageKey) => (preferences.language === 'cs' ? cs[key] : en[key]),
  }
}

export type AppearanceControls = ReturnType<typeof usePreferences>
