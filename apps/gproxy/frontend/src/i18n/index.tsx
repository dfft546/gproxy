import { createContext, useContext, useMemo, useState } from "react";
import type { ReactNode } from "react";

import en from "./translate.en.json";
import zhCn from "./translate.zh_cn.json";

const dictionaries = {
  en,
  zh_cn: zhCn
} as const;

export type Language = keyof typeof dictionaries;

type I18nValue = {
  language: Language;
  setLanguage: (next: Language) => void;
  t: (key: string, vars?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nValue | null>(null);

const STORAGE_KEY = "gproxy_admin_lang";

function pickInitialLanguage(): Language {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "en" || stored === "zh_cn") {
    return stored;
  }
  const nav = navigator.language.toLowerCase();
  if (nav.startsWith("zh")) {
    return "zh_cn";
  }
  return "en";
}

function getByPath(source: unknown, key: string): string | null {
  const parts = key.split(".");
  let current: unknown = source;
  for (const part of parts) {
    if (!current || typeof current !== "object" || !(part in current)) {
      return null;
    }
    current = (current as Record<string, unknown>)[part];
  }
  return typeof current === "string" ? current : null;
}

function applyVars(template: string, vars?: Record<string, string | number>): string {
  if (!vars) {
    return template;
  }
  return template.replace(/\{(\w+)\}/g, (_match, key: string) => {
    const value = vars[key];
    return value === undefined ? `{${key}}` : String(value);
  });
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [language, setLanguageState] = useState<Language>(() => pickInitialLanguage());

  const value = useMemo<I18nValue>(() => {
    const setLanguage = (next: Language) => {
      localStorage.setItem(STORAGE_KEY, next);
      setLanguageState(next);
    };

    const t = (key: string, vars?: Record<string, string | number>) => {
      const primary = getByPath(dictionaries[language], key);
      const fallback = getByPath(dictionaries.en, key);
      const resolved = primary ?? fallback ?? key;
      return applyVars(resolved, vars);
    };

    return { language, setLanguage, t };
  }, [language]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nValue {
  const ctx = useContext(I18nContext);
  if (!ctx) {
    throw new Error("useI18n must be used within I18nProvider");
  }
  return ctx;
}
