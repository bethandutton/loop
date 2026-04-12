import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export type ThemeMode = "dark" | "light" | "system";
export type Density = "compact" | "comfortable" | "spacious";
export type FontSize = "small" | "medium" | "large";

function getSystemTheme(): "dark" | "light" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function applyTheme(mode: ThemeMode) {
  const resolved = mode === "system" ? getSystemTheme() : mode;
  document.documentElement.setAttribute("data-theme", resolved);
  if (resolved === "dark") {
    document.documentElement.classList.add("dark");
  } else {
    document.documentElement.classList.remove("dark");
  }
}

function applyDensity(density: Density) {
  document.documentElement.setAttribute("data-density", density);
}

function applyFontSize(size: FontSize) {
  document.documentElement.setAttribute("data-font-size", size);
}

export function useTheme() {
  const [theme, setThemeState] = useState<ThemeMode>("dark");
  const [density, setDensityState] = useState<Density>("comfortable");
  const [fontSize, setFontSizeState] = useState<FontSize>("medium");

  useEffect(() => {
    invoke<string | null>("get_setting", { key: "theme" }).then((val) => {
      const mode = (val as ThemeMode) || "dark";
      setThemeState(mode);
      applyTheme(mode);
    }).catch(() => {
      applyTheme("dark");
    });

    invoke<string | null>("get_setting", { key: "density" }).then((val) => {
      const d = (val as Density) || "comfortable";
      setDensityState(d);
      applyDensity(d);
    }).catch(() => {
      applyDensity("comfortable");
    });

    invoke<string | null>("get_setting", { key: "font_size" }).then((val) => {
      const fs = (val as FontSize) || "medium";
      setFontSizeState(fs);
      applyFontSize(fs);
    }).catch(() => {
      applyFontSize("medium");
    });
  }, []);

  // Listen for system theme changes
  useEffect(() => {
    if (theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => applyTheme("system");
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme]);

  const setTheme = useCallback((mode: ThemeMode) => {
    setThemeState(mode);
    applyTheme(mode);
    invoke("set_setting", { key: "theme", value: mode }).catch(console.error);
  }, []);

  const setDensity = useCallback((d: Density) => {
    setDensityState(d);
    applyDensity(d);
    invoke("set_setting", { key: "density", value: d }).catch(console.error);
  }, []);

  const setFontSize = useCallback((fs: FontSize) => {
    setFontSizeState(fs);
    applyFontSize(fs);
    invoke("set_setting", { key: "font_size", value: fs }).catch(console.error);
  }, []);

  return { theme, density, fontSize, setTheme, setDensity, setFontSize };
}
