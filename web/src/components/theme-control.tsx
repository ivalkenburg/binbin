import { Monitor, Moon, Sun } from "lucide-react";
import type { Theme } from "@/hooks/use-theme";
import { IconButton } from "./icon-button";

const themes = {
  system: { label: "System", icon: Monitor, next: "light" },
  light: { label: "Light", icon: Sun, next: "dark" },
  dark: { label: "Dark", icon: Moon, next: "system" },
} as const;

export function ThemeControl({
  theme,
  onChange,
}: {
  theme: Theme;
  onChange: (theme: Theme) => void;
}) {
  const { label, icon: Icon, next } = themes[theme];
  return (
    <IconButton
      id="theme"
      className="theme-control"
      label={`Color theme: ${label}. Switch to ${themes[next].label}.`}
      onClick={() => onChange(next)}
    >
      <Icon aria-hidden="true" />
    </IconButton>
  );
}
