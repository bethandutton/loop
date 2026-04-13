import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CheckCircle, AlertCircle, Play, Square, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { PlanEditor } from "@/components/middle/PlanEditor";
import { TerminalSession } from "@/components/middle/TerminalSession";
import type { TicketCard } from "@/App";

interface ClaudeCodeStatus {
  installed: boolean;
  path: string | null;
  authenticated: boolean;
}

interface StartTicketResult {
  session_id: string;
  branch_name: string;
  worktree_path: string;
}

const PLAN_STATUSES = ["backlog", "todo", "planning"];
const SESSION_STATUSES = ["in_progress", "ready_to_test", "in_review", "attention_required", "ready_to_merge"];

interface MiddleColumnProps {
  activeTicket: TicketCard | null;
  hideToolbar?: boolean;
  sessionOnly?: boolean;
  planOnly?: boolean;
}

export function MiddleColumn({ activeTicket, hideToolbar, sessionOnly, planOnly }: MiddleColumnProps) {
  const [claudeStatus, setClaudeStatus] = useState<ClaudeCodeStatus | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const [startError, setStartError] = useState<string | null>(null);

  // Track sessions per ticket
  const [ticketSessions, setTicketSessions] = useState<Record<string, string>>({});

  useEffect(() => {
    invoke<ClaudeCodeStatus>("check_claude_code")
      .then(setClaudeStatus)
      .catch(() => setClaudeStatus({ installed: false, path: null, authenticated: false }));
  }, []);

  // When active ticket changes, check if we have a session for it
  useEffect(() => {
    if (activeTicket && ticketSessions[activeTicket.id]) {
      setSessionId(ticketSessions[activeTicket.id]);
    } else {
      setSessionId(null);
    }
    setStartError(null);
  }, [activeTicket?.id]);

  const handleStartTicket = async () => {
    if (!activeTicket || starting) return;
    setStarting(true);
    setStartError(null);
    try {
      const result = await invoke<StartTicketResult>("start_ticket", {
        ticketId: activeTicket.id,
      });
      setSessionId(result.session_id);
      setTicketSessions((prev) => ({ ...prev, [activeTicket.id]: result.session_id }));
    } catch (e) {
      setStartError(String(e));
    } finally {
      setStarting(false);
    }
  };

  const handleKillSession = async () => {
    if (!sessionId) return;
    try {
      await invoke("kill_session", { sessionId });
      if (activeTicket) {
        setTicketSessions((prev) => {
          const next = { ...prev };
          delete next[activeTicket.id];
          return next;
        });
      }
      setSessionId(null);
    } catch (e) {
      console.error("Failed to kill session:", e);
    }
  };

  // Plan mode — always show for planOnly, or when ticket is in a plan status
  if (activeTicket && (planOnly || (!sessionOnly && PLAN_STATUSES.includes(activeTicket.status)))) {
    return <PlanEditor ticket={activeTicket} hideToolbar={hideToolbar} />;
  }

  // Session mode for in-progress+ tickets (or when sessionOnly forced)
  if (activeTicket && (sessionOnly || SESSION_STATUSES.includes(activeTicket.status))) {
    return (
      <div className="flex h-full flex-col">
        {!hideToolbar && (
          <div className="titlebar-drag-region flex h-14 shrink-0 items-end justify-between pb-2 px-4">
            <div className="titlebar-no-drag flex items-center gap-2 min-w-0">
              <span className="font-mono text-[11px] text-muted-foreground shrink-0">
                {activeTicket.identifier}
              </span>
              <span className="text-[13px] text-foreground truncate">
                {activeTicket.title}
              </span>
            </div>
            <div className="titlebar-no-drag flex items-center gap-1.5">
              {!sessionId && (
                <Button
                  size="sm"
                  onClick={handleStartTicket}
                  disabled={starting}
                >
                  {starting ? (
                    <Loader2 size={13} className="animate-spin mr-1" />
                  ) : (
                    <Play size={13} className="mr-1" />
                  )}
                  {starting ? "Starting..." : "Start session"}
                </Button>
              )}
              {sessionId && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleKillSession}
                  title="Kill session"
                  className="text-destructive hover:text-destructive"
                >
                  <Square size={13} className="mr-1" />
                  Kill
                </Button>
              )}
            </div>
          </div>
        )}

        {/* Inline action buttons when toolbar is hidden */}
        {hideToolbar && (
          <div className="flex items-center justify-end px-4 py-1.5 shrink-0">
            {!sessionId && (
              <Button size="sm" onClick={handleStartTicket} disabled={starting}>
                {starting ? <Loader2 size={13} className="animate-spin mr-1" /> : <Play size={13} className="mr-1" />}
                {starting ? "Starting..." : "Start session"}
              </Button>
            )}
            {sessionId && (
              <Button variant="ghost" size="sm" onClick={handleKillSession} className="text-destructive hover:text-destructive">
                <Square size={13} className="mr-1" /> Kill
              </Button>
            )}
          </div>
        )}

        {startError && (
          <div className="mx-4 mb-2 rounded-md bg-destructive/10 border border-destructive/20 px-3 py-2">
            <p className="text-xs text-destructive">{startError}</p>
          </div>
        )}

        {sessionId ? (
          <div className="flex-1 min-h-0">
            <TerminalSession sessionId={sessionId} />
          </div>
        ) : (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-center space-y-2">
              <p className="text-sm text-muted-foreground">
                No active session.
              </p>
              <p className="text-xs text-muted-foreground">
                Click "Start session" to spawn Claude Code in a worktree.
              </p>
            </div>
          </div>
        )}
      </div>
    );
  }

  // Ticket selected but in a status we don't handle yet (e.g. done)
  if (activeTicket) {
    return (
      <div className="flex h-full flex-col">
        <div className="titlebar-drag-region flex h-14 shrink-0 items-end pb-2 px-4">
          <span className="titlebar-no-drag font-mono text-[11px] text-muted-foreground mr-2">
            {activeTicket.identifier}
          </span>
          <span className="titlebar-no-drag text-[13px] text-foreground truncate">
            {activeTicket.title}
          </span>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-sm text-muted-foreground">
            {activeTicket.status.replace(/_/g, " ")}
          </p>
        </div>
      </div>
    );
  }

  // Empty state — no ticket selected
  return (
    <div className="flex h-full flex-col">
      <div className="titlebar-drag-region flex h-14 shrink-0 items-end pb-2 px-3">
        <span className="titlebar-no-drag text-[13px] text-muted-foreground">&nbsp;</span>
      </div>

      <div className="flex flex-1 items-center justify-center p-8">
        <div className="max-w-sm space-y-6">
          <div className="text-center space-y-2">
            <p className="text-sm text-foreground font-medium">Getting started</p>
            <p className="text-xs text-muted-foreground">
              Your Linear tickets should appear on the left. Pick one to start working.
            </p>
          </div>

          <div className="rounded-md border border-border bg-surface px-4 py-3 space-y-3">
            <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">Setup</p>
            <SetupItem done={true} label="Linear connected" />
            <SetupItem done={true} label="Repository configured" />
            {claudeStatus && (
              <div className="space-y-1">
                <SetupItem
                  done={claudeStatus.installed}
                  label={claudeStatus.installed ? "Claude Code installed" : "Claude Code not found"}
                />
                {!claudeStatus.installed && (
                  <p className="text-[11px] text-muted-foreground pl-5">
                    Install:{" "}
                    <code className="font-mono bg-background px-1 rounded text-[10px]">
                      npm install -g @anthropic-ai/claude-code
                    </code>
                  </p>
                )}
                {claudeStatus.installed && (
                  <SetupItem
                    done={claudeStatus.authenticated}
                    label={claudeStatus.authenticated ? "Claude Code ready" : "Claude Code needs login"}
                  />
                )}
                {claudeStatus.installed && !claudeStatus.authenticated && (
                  <p className="text-[11px] text-muted-foreground pl-5">
                    Run{" "}
                    <code className="font-mono bg-background px-1 rounded text-[10px]">claude auth login</code>{" "}
                    in your terminal
                  </p>
                )}
              </div>
            )}
          </div>

          <p className="text-[11px] text-muted-foreground text-center">
            Preferences:{" "}
            <kbd className="font-mono bg-surface border border-border rounded px-1 text-[10px]">⌘,</kbd>
            {" "}or Herd menu
          </p>
        </div>
      </div>
    </div>
  );
}

function SetupItem({ done, label }: { done: boolean; label: string }) {
  return (
    <div className="flex items-center gap-2">
      {done ? (
        <CheckCircle size={14} className="text-success shrink-0" />
      ) : (
        <AlertCircle size={14} className="text-warning shrink-0" />
      )}
      <span className={`text-xs ${done ? "text-muted-foreground" : "text-foreground"}`}>
        {label}
      </span>
    </div>
  );
}
