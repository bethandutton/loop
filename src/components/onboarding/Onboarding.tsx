import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

type Step = "welcome" | "linear" | "github" | "repo" | "done";

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
  const [repoName, setRepoName] = useState("");
  const [primaryBranch, setPrimaryBranch] = useState("main");
  const [previewPort, setPreviewPort] = useState("3000");

  const verifyLinear = async () => {
    setError(null);
    setLoading(true);
    try {
      await invoke("store_token", {
        key: "linear_api_token",
        value: linearToken,
      });
      // For now, just accept the token. Real verification will come in Phase 1.
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

  const setupRepo = async () => {
    setError(null);
    setLoading(true);
    try {
      const path = repoPath.trim();
      const name = repoName.trim() || path.split("/").pop() || "repo";
      const worktreesDir = path.replace(/\/?$/, "") + "-worktrees";
      await invoke("create_repo", {
        name,
        path,
        worktreesDir,
        primaryBranch: primaryBranch.trim() || "main",
        previewPort: parseInt(previewPort) || 3000,
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
          <div className="space-y-4">
            <h1 className="text-base font-semibold tracking-tight">
              Welcome to Loop
            </h1>
            <p className="text-sm text-muted-foreground">
              Loop manages your in-flight Linear tickets across Git worktrees,
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
              Paste your Linear API token. Create one at{" "}
              <a
                href="https://linear.app/settings/api"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline underline-offset-2 hover:opacity-80"
              >
                linear.app/settings/api
              </a>{" "}
              → Personal API keys.
            </p>
            <Input
              type="password"
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
              Paste a GitHub personal access token with <code className="font-mono text-xs">repo</code> scope.
              Create one at{" "}
              <a
                href="https://github.com/settings/tokens/new?scopes=repo&description=Loop"
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary underline underline-offset-2 hover:opacity-80"
              >
                github.com/settings/tokens
              </a>.
            </p>
            <Input
              type="password"
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
              Set up your repo
            </h2>
            <p className="text-sm text-muted-foreground">
              Point Loop at your local clone.
            </p>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">
                Repo path
              </label>
              <Input
                placeholder="/Users/you/code/my-project"
                value={repoPath}
                onChange={(e) => setRepoPath(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <label className="text-xs text-muted-foreground">
                Friendly name
              </label>
              <Input
                placeholder="my-project"
                value={repoName}
                onChange={(e) => setRepoName(e.target.value)}
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-2">
                <label className="text-xs text-muted-foreground">
                  Primary branch
                </label>
                <Input
                  placeholder="main"
                  value={primaryBranch}
                  onChange={(e) => setPrimaryBranch(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <label className="text-xs text-muted-foreground">
                  Preview port
                </label>
                <Input
                  type="number"
                  placeholder="3000"
                  value={previewPort}
                  onChange={(e) => setPreviewPort(e.target.value)}
                />
              </div>
            </div>
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
                disabled={!repoPath.trim() || loading}
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
              Loop is ready. Your tokens are stored securely in macOS Keychain.
            </p>
            <Button onClick={onComplete} className="w-full">
              Open Loop
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
