import { useTheme, type ThemeMode, type Density, type FontSize } from "@/hooks/useTheme";
import { Sun, Moon, Monitor, Settings } from "lucide-react";

interface FooterProps {
  onOpenSettings: () => void;
  rightColumnVisible: boolean;
  onToggleRightColumn: () => void;
}

export function Footer({
  onOpenSettings,
  rightColumnVisible,
  onToggleRightColumn,
}: FooterProps) {
  const { theme, density, fontSize, setTheme, setDensity, setFontSize } =
    useTheme();

  const themeIcons: Record<ThemeMode, typeof Sun> = {
    dark: Moon,
    light: Sun,
    system: Monitor,
  };

  const nextTheme: Record<ThemeMode, ThemeMode> = {
    dark: "light",
    light: "system",
    system: "dark",
  };

  const nextDensity: Record<Density, Density> = {
    compact: "comfortable",
    comfortable: "spacious",
    spacious: "compact",
  };

  const nextFontSize: Record<FontSize, FontSize> = {
    small: "medium",
    medium: "large",
    large: "small",
  };

  const ThemeIcon = themeIcons[theme];

  return (
    <footer className="flex h-7 shrink-0 items-center justify-between border-t border-border bg-surface px-3 text-[11px] text-muted-foreground">
      {/* Left */}
      <div className="flex items-center gap-3">
        <span>0 sessions running</span>
      </div>

      {/* Center */}
      <div />

      {/* Right */}
      <div className="flex items-center gap-1">
        <button
          onClick={() => setTheme(nextTheme[theme])}
          className="flex h-5 items-center gap-1 rounded px-1.5 hover:bg-surface-elevated"
          title={`Theme: ${theme}`}
        >
          <ThemeIcon size={12} />
          <span className="capitalize">{theme}</span>
        </button>

        <button
          onClick={() => setDensity(nextDensity[density])}
          className="flex h-5 items-center rounded px-1.5 hover:bg-surface-elevated"
          title={`Density: ${density}`}
        >
          <span className="capitalize">{density}</span>
        </button>

        <button
          onClick={() => setFontSize(nextFontSize[fontSize])}
          className="flex h-5 items-center rounded px-1.5 hover:bg-surface-elevated"
          title={`Font: ${fontSize}`}
        >
          <span className="capitalize">{fontSize}</span>
        </button>

        <button
          onClick={onToggleRightColumn}
          className="flex h-5 items-center rounded px-1.5 hover:bg-surface-elevated"
          title={rightColumnVisible ? "Hide right column" : "Show right column"}
        >
          {rightColumnVisible ? "⊟" : "⊞"}
        </button>

        <button
          onClick={onOpenSettings}
          className="flex h-5 w-5 items-center justify-center rounded hover:bg-surface-elevated"
          title="Settings (⌘,)"
        >
          <Settings size={12} />
        </button>
      </div>
    </footer>
  );
}
