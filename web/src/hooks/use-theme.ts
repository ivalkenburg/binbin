import { useEffect, useState } from "react";
export type Theme = "system" | "light" | "dark";
function preference(): Theme {
  try {
    const value = localStorage.getItem("binbin-theme");
    return value === "light" || value === "dark" ? value : "system";
  } catch {
    return "system";
  }
}
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(preference);
  useEffect(() => {
    const query = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const root = document.documentElement;
      if (theme === "system") delete root.dataset.theme;
      else root.dataset.theme = theme;
      root.classList.toggle(
        "dark",
        theme === "dark" || (theme === "system" && query.matches),
      );
    };
    apply();
    query.addEventListener("change", apply);
    try {
      localStorage.setItem("binbin-theme", theme);
    } catch {
      /* Theme works without persistence. */
    }
    return () => query.removeEventListener("change", apply);
  }, [theme]);
  return { theme, setTheme };
}
