import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Toaster, toast } from "sonner";
import { SquareKanban, GitPullRequest, Globe, Bot, ChevronLeft, ChevronRight, Info } from "lucide-react";
import { Board } from "@/components/board/Board";
import { MiddleColumn } from "@/components/middle/MiddleColumn";
import { RightColumn } from "@/components/right/RightColumn";
import { Onboarding } from "@/components/onboarding/Onboarding";
import { SettingsPanel } from "@/components/settings/SettingsPanel";
import { CommandPalette } from "@/components/CommandPalette";

type AppView = "loading" | "onboarding" | "main";
type Tab = "plan" | "session" | "local" | "pr";

export interface TicketCard {
  id: string;
  identifier: string;
  title: string;
  priority: number;
  status: string;
  branch_name: string | null;
  tags: string[];
  project: string | null;
  assignee: string | null;
  created_at: string;
  updated_at: string;
}

const PLAN_STATUSES = ["backlog", "todo", "planning"];

export default function App() {
  const [view, setView] = useState<AppView>("loading");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [tickets, setTickets] = useState<TicketCard[]>([]);
  const [activeTicketId, setActiveTicketId] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<Tab>("plan");
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const isNavigating = useRef(false);

  const navigateToTicket = useCallback((id: string) => {
    if (isNavigating.current) return;
    setActiveTicketId(id);
    setHistory((prev) => {
      const newHistory = prev.slice(0, historyIndex + 1);
      newHistory.push(id);
      return newHistory;
    });
    setHistoryIndex((i) => i + 1);
  }, [historyIndex]);

  const goBack = useCallback(() => {
    if (historyIndex <= 0) return;
    isNavigating.current = true;
    const newIndex = historyIndex - 1;
    setHistoryIndex(newIndex);
    setActiveTicketId(history[newIndex]);
    setTimeout(() => { isNavigating.current = false; }, 0);
  }, [history, historyIndex]);

  const goForward = useCallback(() => {
    if (historyIndex >= history.length - 1) return;
    isNavigating.current = true;
    const newIndex = historyIndex + 1;
    setHistoryIndex(newIndex);
    setActiveTicketId(history[newIndex]);
    setTimeout(() => { isNavigating.current = false; }, 0);
  }, [history, historyIndex]);

  useEffect(() => {
    invoke<boolean>("has_repos")
      .then((hasRepos) => {
        setView(hasRepos ? "main" : "onboarding");
      })
      .catch(() => {
        setView("onboarding");
      });
  }, []);

  // Fetch tickets on load, then listen for background polling updates
  useEffect(() => {
    if (view !== "main") return;

    invoke<TicketCard[]>("fetch_linear_tickets")
      .then(setTickets)
      .catch((e) => {
        console.error("Failed to fetch tickets:", e);
        toast.error("Failed to fetch tickets from Linear");
      });

    const unlisten = listen("tickets_updated", () => {
      invoke<TicketCard[]>("get_tickets")
        .then(setTickets)
        .catch((e) => console.error("Failed to get tickets:", e));
    });

    return () => { unlisten.then((f) => f()); };
  }, [view]);

  // Listen for macOS menu events
  useEffect(() => {
    const unlisten1 = listen("open_settings", () => setSettingsOpen(true));
    return () => { unlisten1.then((f) => f()); };
  }, []);

  // When switching tickets, keep current tab if it's still enabled, otherwise default to Linear Ticket
  useEffect(() => {
    if (!activeTicket) return;
    // "plan" (Linear Ticket) is always available when a ticket is selected
    // Only force-switch if current tab would be disabled
    const hasBranchNow = !!activeTicket.branch_name;
    const needsBranch = activeTab === "pr" || activeTab === "local";
    if (needsBranch && !hasBranchNow) {
      setActiveTab("plan");
    }
  }, [activeTicketId]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const isInput = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;

      if (e.metaKey && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen((v) => !v);
        return;
      }
      if (e.metaKey && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
        return;
      }
      if (e.key === "Escape") {
        if (commandPaletteOpen) setCommandPaletteOpen(false);
        else if (settingsOpen) setSettingsOpen(false);
        return;
      }

      // Tab switching: Cmd+1/2/3/4
      if (e.metaKey && e.key === "1") { e.preventDefault(); setActiveTab("plan"); return; }
      if (e.metaKey && e.key === "2") { e.preventDefault(); setActiveTab("session"); return; }
      if (e.metaKey && e.key === "3") { e.preventDefault(); setActiveTab("local"); return; }
      if (e.metaKey && e.key === "4") { e.preventDefault(); setActiveTab("pr"); return; }

      // Board navigation (j/k)
      if (!isInput && !commandPaletteOpen && !settingsOpen) {
        if (e.key === "j" || e.key === "k") {
          e.preventDefault();
          const currentIndex = tickets.findIndex((t) => t.id === activeTicketId);
          if (e.key === "j") {
            const next = Math.min(currentIndex + 1, tickets.length - 1);
            if (tickets[next]) navigateToTicket(tickets[next].id);
          } else {
            const prev = Math.max(currentIndex - 1, 0);
            if (tickets[prev]) navigateToTicket(tickets[prev].id);
          }
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [commandPaletteOpen, settingsOpen, tickets, activeTicketId]);

  const handleOnboardingComplete = useCallback(() => {
    setView("main");
  }, []);

  const handleRerunSetup = useCallback(() => {
    setSettingsOpen(false);
    setView("onboarding");
  }, []);

  const activeTicket = tickets.find((t) => t.id === activeTicketId) || null;

  if (view === "loading") {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );
  }

  if (view === "onboarding") {
    return <Onboarding onComplete={handleOnboardingComplete} />;
  }

  // Determine which tabs are available based on ticket state
  const hasBranch = !!activeTicket?.branch_name;
  const hasTicket = !!activeTicket;

  const tabs: { key: Tab; label: string; icon: React.ReactNode; enabled: boolean; disabledReason: string }[] = [
    { key: "plan", label: "Linear Ticket", icon: <SquareKanban size={13} />, enabled: hasTicket, disabledReason: "Select a ticket to view" },
    { key: "pr", label: "GitHub PR", icon: <GitPullRequest size={13} />, enabled: hasBranch, disabledReason: "Start work on a ticket to create a branch and PR" },
    { key: "local", label: "Local Preview", icon: <Globe size={13} />, enabled: hasBranch, disabledReason: "Start work on a ticket to enable local preview" },
    { key: "session", label: "Agent", icon: <Bot size={13} />, enabled: hasBranch || (hasTicket && PLAN_STATUSES.includes(activeTicket!.status)), disabledReason: "Move ticket to Planning first to start an agent session" },
  ];

  return (
    <div className="flex h-screen flex-col bg-background">
      <div className="flex flex-1 min-h-0 gap-1.5 p-1.5">
        {/* Left — Board */}
        <div className="w-[280px] min-w-[260px] shrink-0 bg-background rounded-xl overflow-hidden flex flex-col">
          {/* Nav buttons — right of traffic lights */}
          <div className="titlebar-drag-region flex items-center gap-0.5 px-[76px] pt-2 pb-0 shrink-0">
            <button
              onClick={goBack}
              disabled={historyIndex <= 0}
              className="titlebar-no-drag flex h-7 w-7 items-center justify-center rounded-lg hover:bg-surface-elevated text-muted-foreground hover:text-foreground disabled:text-muted-foreground/20 disabled:hover:bg-transparent transition-colors duration-75"
              title="Back"
            >
              <ChevronLeft size={16} />
            </button>
            <button
              onClick={goForward}
              disabled={historyIndex >= history.length - 1}
              className="titlebar-no-drag flex h-7 w-7 items-center justify-center rounded-lg hover:bg-surface-elevated text-muted-foreground hover:text-foreground disabled:text-muted-foreground/20 disabled:hover:bg-transparent transition-colors duration-75"
              title="Forward"
            >
              <ChevronRight size={16} />
            </button>
          </div>
          <div className="flex-1 min-h-0">
          <Board
            tickets={tickets}
            activeTicketId={activeTicketId}
            onSelectTicket={navigateToTicket}
          />
          </div>
        </div>

        {/* Main area */}
        <div className="flex-1 min-w-0 flex flex-col">
          {/* Tab row — sits above the content panel */}
          <div className="titlebar-drag-region flex shrink-0 items-end gap-1 pt-2 pb-1">
            {activeTicket ? (
              tabs.map((tab) => (
                <div key={tab.key} className="titlebar-no-drag relative group">
                  <button
                    onClick={() => tab.enabled && setActiveTab(tab.key)}
                    disabled={!tab.enabled}
                    className={`flex items-center gap-2 px-4 py-2 text-[13px] font-medium rounded-xl transition-colors duration-75 ${
                      !tab.enabled
                        ? "text-muted-foreground/30 cursor-not-allowed"
                        : activeTab === tab.key
                          ? "bg-surface text-foreground"
                          : "text-muted-foreground hover:text-foreground hover:bg-surface/50"
                    }`}
                  >
                    <span className={!tab.enabled ? "text-muted-foreground/30" : activeTab === tab.key ? "text-primary" : "text-muted-foreground"}>{tab.icon}</span>
                    {tab.label}
                  </button>
                  {!tab.enabled && (
                    <span className="pointer-events-none absolute left-1/2 -translate-x-1/2 top-full mt-1.5 whitespace-nowrap rounded bg-zinc-900 dark:bg-zinc-800 px-2 py-1 text-[11px] text-white shadow-lg opacity-0 group-hover:opacity-100 transition-opacity duration-100 z-50">
                      {tab.disabledReason}
                    </span>
                  )}
                </div>
              ))
            ) : (
              <div className="h-8" />
            )}
          </div>

          {/* Content panel */}
          <div className="flex-1 min-h-0 bg-surface rounded-xl overflow-hidden flex flex-col">
            {/* Ticket title bar */}
            {activeTicket && (
              <div className="shrink-0 px-4 py-2.5 border-b border-border/50">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-[11px] text-muted-foreground shrink-0">
                    {activeTicket.identifier}
                  </span>
                  <EditableTitle
                    ticketId={activeTicket.id}
                    title={activeTicket.title}
                    onSaved={(newTitle) => {
                      setTickets((prev) =>
                        prev.map((t) => t.id === activeTicket.id ? { ...t, title: newTitle } : t)
                      );
                    }}
                  />
                </div>
                {(activeTicket.project || activeTicket.assignee || activeTicket.branch_name) && (
                  <div className="flex items-center gap-3 mt-1">
                    {activeTicket.project && (
                      <span className="text-[11px] text-muted-foreground">
                        {activeTicket.project}
                      </span>
                    )}
                    {activeTicket.assignee && (
                      <span className="text-[11px] text-muted-foreground">
                        {activeTicket.assignee}
                      </span>
                    )}
                    {activeTicket.branch_name && (
                      <span className="font-mono text-[10px] text-muted-foreground/60">
                        {activeTicket.branch_name}
                      </span>
                    )}
                  </div>
                )}
              </div>
            )}

            {/* Tab content */}
            <div className="flex-1 min-h-0 overflow-hidden">
            {activeTab === "plan" && (
              <LinearTicketTab activeTicket={activeTicket} />
            )}
            {activeTab === "session" && (
              <MiddleColumn activeTicket={activeTicket} hideToolbar sessionOnly />
            )}
            {activeTab === "local" && (
              <LocalPreviewTab activeTicket={activeTicket} />
            )}
            {activeTab === "pr" && (
              <PrTab activeTicket={activeTicket} />
            )}
            </div>
          </div>
        </div>
      </div>

      <SettingsPanel
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onRerunSetup={handleRerunSetup}
      />

      <CommandPalette
        open={commandPaletteOpen}
        onClose={() => setCommandPaletteOpen(false)}
        tickets={tickets}
        onSelectTicket={navigateToTicket}
        onOpenSettings={() => setSettingsOpen(true)}
        onToggleRightColumn={() => {}}
        onNewTicket={() => {}}
      />

      <Toaster
        position="bottom-right"
        toastOptions={{
          duration: 2000,
          className: "!bg-surface-elevated !border-border !text-foreground !text-xs",
        }}
      />
    </div>
  );
}

// PR tab — shows PR info or iframe
// Editable title — click to edit, Enter to save, Escape to cancel
function EditableTitle({ ticketId, title, onSaved }: { ticketId: string; title: string; onSaved: (t: string) => void }) {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState(title);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => { setValue(title); }, [title]);
  useEffect(() => { if (editing) inputRef.current?.select(); }, [editing]);

  const save = async () => {
    const trimmed = value.trim();
    if (!trimmed || trimmed === title) { setEditing(false); setValue(title); return; }
    try {
      await invoke("update_ticket_title", { ticketId, title: trimmed });
      onSaved(trimmed);
    } catch (e) {
      console.error("Failed to update title:", e);
      setValue(title);
    }
    setEditing(false);
  };

  if (editing) {
    return (
      <input
        ref={inputRef}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={save}
        onKeyDown={(e) => {
          if (e.key === "Enter") save();
          if (e.key === "Escape") { setValue(title); setEditing(false); }
        }}
        className="text-[13px] text-foreground bg-transparent outline-none border-b border-primary flex-1 min-w-0"
      />
    );
  }

  return (
    <span
      onClick={() => setEditing(true)}
      className="text-[13px] text-foreground truncate cursor-text hover:border-b hover:border-border"
    >
      {title}
    </span>
  );
}

// Linear Ticket tab — embeds the Linear issue page
function LinearTicketTab({ activeTicket }: { activeTicket: TicketCard | null }) {
  if (!activeTicket) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">Select a ticket to view.</p>
      </div>
    );
  }

  const linearUrl = `https://linear.app/issue/${activeTicket.identifier}`;
  return (
    <iframe
      src={linearUrl}
      className="h-full w-full border-0"
      title="Linear ticket"
    />
  );
}

// Local Preview tab — localhost iframe
function LocalPreviewTab({ activeTicket }: { activeTicket: TicketCard | null }) {
  const [previewPort, setPreviewPort] = useState(3000);

  useEffect(() => {
    invoke<{ preview_port: number } | null>("get_active_repo").then((repo) => {
      if (repo) setPreviewPort(repo.preview_port);
    }).catch(() => {});
  }, []);

  if (!activeTicket) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">Select a ticket to preview.</p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 px-4 py-1.5 border-b border-border/50 flex items-center gap-2">
        <span className="font-mono text-[11px] text-muted-foreground">localhost:{previewPort}</span>
      </div>
      <iframe
        src={`http://localhost:${previewPort}`}
        className="flex-1 w-full border-0 bg-white"
        title="Local preview"
      />
    </div>
  );
}

function PrTab({ activeTicket }: { activeTicket: TicketCard | null }) {
  const [prInfo, setPrInfo] = useState<any>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!activeTicket?.branch_name) {
      setPrInfo(null);
      return;
    }
    setLoading(true);
    invoke("check_pr_status", { branchName: activeTicket.branch_name })
      .then((info) => setPrInfo(info))
      .catch(() => setPrInfo(null))
      .finally(() => setLoading(false));
  }, [activeTicket?.branch_name]);

  if (!activeTicket) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">Select a ticket to view PR status.</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="h-4 w-4 animate-spin rounded-full border-2 border-primary border-t-transparent" />
      </div>
    );
  }

  if (!prInfo) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-center space-y-2">
          <p className="text-sm text-muted-foreground">No PR found for this ticket.</p>
          {activeTicket.branch_name && (
            <p className="text-xs text-muted-foreground/70">
              Branch: <span className="font-mono">{activeTicket.branch_name}</span>
            </p>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* PR info bar */}
      <div className="shrink-0 px-4 py-3 border-b border-border flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          <span className="text-xs font-medium text-foreground">#{prInfo.number}</span>
          <span className="text-xs text-muted-foreground truncate">{prInfo.title}</span>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          {prInfo.approved && (
            <span className="text-[10px] bg-success/20 text-success rounded-full px-2 py-0.5">Approved</span>
          )}
          {prInfo.changes_requested && (
            <span className="text-[10px] bg-destructive/20 text-destructive rounded-full px-2 py-0.5">Changes requested</span>
          )}
          {prInfo.draft && (
            <span className="text-[10px] bg-muted-foreground/20 text-muted-foreground rounded-full px-2 py-0.5">Draft</span>
          )}
          <span className="text-[11px] text-muted-foreground">{prInfo.comment_count} comments</span>
        </div>
      </div>
      {/* PR webview */}
      <iframe
        src={prInfo.url}
        className="flex-1 w-full border-0"
        title="Pull request"
      />
    </div>
  );
}
