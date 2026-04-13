import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { PasswordInput } from "@/components/ui/password-input";
import { FolderOpen } from "lucide-react";

type Step = "welcome" | "linear" | "github" | "repo" | "done";

interface RepoInfo {
  name: string;
  primary_branch: string;
  worktrees_dir: string;
}

interface OnboardingProps {
  onComplete: () => void;
}

export function Onboarding({ onComplete }: OnboardingProps) {
  const [step, setStep] = useState<Step>("welcome");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Linear
  const [linearToken, setLinearToken] = useState("");
  const [linearVerified, setLinearVerified] = useState(false);

  // GitHub
  const [githubToken, setGithubToken] = useState("");

  // Repo
  const [repoPath, setRepoPath] = useState("");
  const [repoInfo, setRepoInfo] = useState<RepoInfo | null>(null);

  const verifyLinear = async () => {
    setError(null);
    setLoading(true);
    try {
      await invoke("store_token", {
        key: "linear_api_token",
        value: linearToken,
      });
      setLinearVerified(true);
      setStep("github");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const verifyGithub = async () => {
    setError(null);
    setLoading(true);
    try {
      await invoke("store_token", {
        key: "github_api_token",
        value: githubToken,
      });
      setStep("repo");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const pickFolder = async () => {
    setError(null);
    try {
      const selected = await open({ directory: true, multiple: false });
      if (!selected) return;
      const path = selected as string;
      setRepoPath(path);
      const info = await invoke<RepoInfo>("detect_repo_info", { path });
      setRepoInfo(info);
    } catch (e) {
      setError(String(e));
    }
  };

  const setupRepo = async () => {
    if (!repoInfo) return;
    setError(null);
    setLoading(true);
    try {
      await invoke("create_repo", {
        name: repoInfo.name,
        path: repoPath,
        worktreesDir: repoInfo.worktrees_dir,
        primaryBranch: repoInfo.primary_branch,
        previewPort: 3000,
      });
      setStep("done");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-background">
      <div className="w-full max-w-md rounded-lg border border-border bg-surface p-8">
        {step === "welcome" && (
          <div className="space-y-4 text-center">
            <div className="flex justify-center">
              <img src="/app-icon.png" alt="Herd" className="h-16 w-16" />
            </div>
            <h1 className="text-base font-semibold tracking-tight">
              Welcome to Herd
            </h1>
            <p className="text-sm text-muted-foreground">
              Herd manages your in-flight Linear tickets across Git worktrees,
              each with its own background Claude Code session. Let's set up
              your connections.
            </p>
            <Button onClick={() => setStep("linear")} className="w-full">
              Get started
            </Button>
          </div>
        )}

        {step === "linear" && (
          <div className="space-y-4">
            <h2 className="text-base font-semibold tracking-tight">
              Connect Linear
            </h2>
            <p className="text-sm text-muted-foreground">
              Paste your Linear personal API key. Create one at{" "}
              <a
                href="https://linear.app/settings/api"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline underline-offset-2 hover:opacity-80"
              >
                Linear Settings → Account → Security
              </a>.
            </p>
            <div className="rounded-md border border-border bg-background px-3 py-2">
              <p className="text-[11px] font-medium text-muted-foreground mb-1">Required access:</p>
              <ul className="text-[11px] text-muted-foreground space-y-0.5">
                <li>Read issues, labels, and cycles assigned to you</li>
                <li>Read and write issue descriptions (for plan editor)</li>
                <li>Create new issues</li>
              </ul>
            </div>
            <PasswordInput
              placeholder="lin_api_..."
              value={linearToken}
              onChange={(e) => setLinearToken(e.target.value)}
            />
            {error && (
              <p className="text-xs text-destructive">{error}</p>
            )}
            {linearVerified && (
              <p className="text-xs text-success">Connected</p>
            )}
            <div className="flex gap-2">
              <Button
                variant="outline"
                onClick={() => setStep("welcome")}
              >
                Back
              </Button>
              <Button
                onClick={verifyLinear}
                disabled={!linearToken.trim() || loading}
                className="flex-1"
              >
                {loading ? "Verifying..." : "Connect"}
              </Button>
            </div>
          </div>
        )}

        {step === "github" && (
          <div className="space-y-4">
            <h2 className="text-base font-semibold tracking-tight">
              Connect GitHub
            </h2>
            <p className="text-sm text-muted-foreground">
              Paste a GitHub personal access token (classic). Create one at{" "}
              <a
                href="https://github.com/settings/tokens/new?scopes=repo&description=Herd"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline underline-offset-2 hover:opacity-80"
              >
                GitHub Settings → Developer settings → Tokens
              </a>.
            </p>
            <div className="rounded-md border border-border bg-background px-3 py-2">
              <p className="text-[11px] font-medium text-muted-foreground mb-1">Required scopes:</p>
              <ul className="text-[11px] text-muted-foreground space-y-0.5">
                <li><code className="font-mono bg-surface px-1 rounded">repo</code> — read PR state, reviews, comments, and CI status</li>
              </ul>
              <p className="text-[11px] text-muted-foreground/70 mt-1.5">Herd only reads from GitHub in v1. It never writes, merges, or comments.</p>
            </div>
            <PasswordInput
              placeholder="ghp_..."
              value={githubToken}
              onChange={(e) => setGithubToken(e.target.value)}
            />
            {error && (
              <p className="text-xs text-destructive">{error}</p>
            )}
            <div className="flex gap-2">
              <Button
                variant="outline"
                onClick={() => setStep("linear")}
              >
                Back
              </Button>
              <Button
                variant="ghost"
                onClick={() => setStep("repo")}
              >
                Skip
              </Button>
              <Button
                onClick={verifyGithub}
                disabled={!githubToken.trim() || loading}
                className="flex-1"
              >
                {loading ? "Verifying..." : "Connect"}
              </Button>
            </div>
          </div>
        )}

        {step === "repo" && (
          <div className="space-y-4">
            <h2 className="text-base font-semibold tracking-tight">
              Select your repo
            </h2>
            <p className="text-sm text-muted-foreground">
              Pick the folder of your local Git clone. Herd will auto-detect
              the branch and set up worktrees alongside it.
            </p>

            <button
              onClick={pickFolder}
              className="flex w-full items-center gap-3 rounded-md border border-border bg-background px-3 py-3 text-left hover:bg-surface-elevated transition-colors duration-75"
            >
              <FolderOpen size={18} className="shrink-0 text-muted-foreground" />
              {repoPath ? (
                <span className="font-mono text-xs text-foreground truncate">{repoPath}</span>
              ) : (
                <span className="text-sm text-muted-foreground">Choose a folder...</span>
              )}
            </button>

            {repoInfo && (
              <div className="rounded-md border border-border bg-background px-3 py-2 space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-muted-foreground">Name</span>
                  <span className="font-mono text-xs text-foreground">{repoInfo.name}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-muted-foreground">Branch</span>
                  <span className="font-mono text-xs text-foreground">{repoInfo.primary_branch}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-muted-foreground">Worktrees</span>
                  <span className="font-mono text-xs text-foreground truncate ml-4">{repoInfo.worktrees_dir}</span>
                </div>
              </div>
            )}

            {error && (
              <p className="text-xs text-destructive">{error}</p>
            )}
            <div className="flex gap-2">
              <Button
                variant="outline"
                onClick={() => setStep("github")}
              >
                Back
              </Button>
              <Button
                onClick={setupRepo}
                disabled={!repoInfo || loading}
                className="flex-1"
              >
                {loading ? "Setting up..." : "Complete setup"}
              </Button>
            </div>
          </div>
        )}

        {step === "done" && (
          <div className="space-y-4">
            <h2 className="text-base font-semibold tracking-tight">
              You're all set
            </h2>
            <p className="text-sm text-muted-foreground">
              Herd is ready. Your tokens are stored securely.
            </p>
            <Button onClick={onComplete} className="w-full">
              Open Herd
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
