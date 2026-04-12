import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { useTheme, type ThemeMode, type Density, type FontSize } from "@/hooks/useTheme";
import { X } from "lucide-react";

interface SettingsPanelProps {
  open: boolean;
  onClose: () => void;
  onRerunSetup: () => void;
}

export function SettingsPanel({ open, onClose, onRerunSetup }: SettingsPanelProps) {
  const { theme, density, fontSize, setTheme, setDensity, setFontSize } = useTheme();

  const [linearToken, setLinearToken] = useState("");
  const [githubToken, setGithubToken] = useState("");
  const [repoPath, setRepoPath] = useState("");
  const [worktreesDir, setWorktreesDir] = useState("");
  const [primaryBranch, setPrimaryBranch] = useState("main");
  const [previewPort, setPreviewPort] = useState("3000");
  const [copyFiles, setCopyFiles] = useState(".env*");
  const [mirrorToLinear, setMirrorToLinear] = useState(false);

  useEffect(() => {
    if (!open) return;

    // Load tokens (masked)
    invoke<string | null>("get_token", { key: "linear_api_token" }).then((val) => {
      if (val) setLinearToken("••••••••" + val.slice(-4));
    }).catch(() => {});

    invoke<string | null>("get_token", { key: "github_api_token" }).then((val) => {
      if (val) setGithubToken("••••••••" + val.slice(-4));
    }).catch(() => {});

    // Load repo settings
    invoke<{ path: string; worktrees_dir: string; primary_branch: string; preview_port: number } | null>("get_active_repo").then((repo) => {
      if (repo) {
        setRepoPath(repo.path);
        setWorktreesDir(repo.worktrees_dir);
        setPrimaryBranch(repo.primary_branch);
        setPreviewPort(String(repo.preview_port));
      }
    }).catch(() => {});

    invoke<string | null>("get_setting", { key: "copy_files" }).then((val) => {
      if (val) setCopyFiles(val);
    }).catch(() => {});

    invoke<string | null>("get_setting", { key: "mirror_to_linear" }).then((val) => {
      setMirrorToLinear(val === "true");
    }).catch(() => {});
  }, [open]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-full max-w-lg rounded-lg border border-border bg-surface-elevated shadow-lg">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-border px-4 py-3">
          <h2 className="text-sm font-semibold">Settings</h2>
          <button
            onClick={onClose}
            className="flex h-6 w-6 items-center justify-center rounded hover:bg-surface"
          >
            <X size={14} />
          </button>
        </div>

        {/* Content */}
        <div className="max-h-[70vh] overflow-y-auto p-4 space-y-6">
          {/* Appearance */}
          <section className="space-y-3">
            <h3 className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
              Appearance
            </h3>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">Theme</label>
              <SegmentedControl
                value={theme}
                onChange={setTheme}
                options={[
                  { value: "system" as ThemeMode, label: "System" },
                  { value: "light" as ThemeMode, label: "Light" },
                  { value: "dark" as ThemeMode, label: "Dark" },
                ]}
              />
            </div>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">Density</label>
              <SegmentedControl
                value={density}
                onChange={setDensity}
                options={[
                  { value: "compact" as Density, label: "Compact" },
                  { value: "comfortable" as Density, label: "Comfortable" },
                  { value: "spacious" as Density, label: "Spacious" },
                ]}
              />
            </div>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">Font size</label>
              <SegmentedControl
                value={fontSize}
                onChange={setFontSize}
                options={[
                  { value: "small" as FontSize, label: "Small" },
                  { value: "medium" as FontSize, label: "Medium" },
                  { value: "large" as FontSize, label: "Large" },
                ]}
              />
            </div>
          </section>

          {/* Connections */}
          <section className="space-y-3">
            <h3 className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
              Connections
            </h3>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">Linear API token</label>
              <Input
                type="password"
                value={linearToken}
                onChange={(e) => setLinearToken(e.target.value)}
                placeholder="lin_api_..."
              />
            </div>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">GitHub personal access token</label>
              <Input
                type="password"
                value={githubToken}
                onChange={(e) => setGithubToken(e.target.value)}
                placeholder="ghp_..."
              />
            </div>
          </section>

          {/* Repo */}
          <section className="space-y-3">
            <h3 className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
              Repository
            </h3>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">Repo path</label>
              <Input value={repoPath} readOnly className="opacity-70" />
            </div>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">Worktrees directory</label>
              <Input value={worktreesDir} readOnly className="opacity-70" />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <label className="text-xs text-muted-foreground">Primary branch</label>
                <Input value={primaryBranch} readOnly className="opacity-70" />
              </div>
              <div className="space-y-2">
                <label className="text-xs text-muted-foreground">Preview port</label>
                <Input value={previewPort} readOnly className="opacity-70" />
              </div>
            </div>
          </section>

          {/* Advanced */}
          <section className="space-y-3">
            <h3 className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
              Advanced
            </h3>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">
                Files to copy into new worktrees
              </label>
              <Input
                value={copyFiles}
                onChange={(e) => setCopyFiles(e.target.value)}
                placeholder=".env*"
              />
            </div>
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={mirrorToLinear}
                onChange={(e) => setMirrorToLinear(e.target.checked)}
                className="h-3.5 w-3.5 rounded border-border"
              />
              <label className="text-xs text-muted-foreground">
                Mirror status changes back to Linear
              </label>
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-border px-4 py-3">
          <Button variant="ghost" size="sm" onClick={onRerunSetup}>
            Re-run setup
          </Button>
          <Button size="sm" onClick={onClose}>
            Done
          </Button>
        </div>
      </div>
    </div>
  );
}
