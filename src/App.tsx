import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Board } from "@/components/board/Board";
import { MiddleColumn } from "@/components/middle/MiddleColumn";
import { RightColumn } from "@/components/right/RightColumn";
import { Onboarding } from "@/components/onboarding/Onboarding";
import { SettingsPanel } from "@/components/settings/SettingsPanel";

type AppView = "loading" | "onboarding" | "main";

export interface TicketCard {
  id: string;
  title: string;
  priority: number;
  status: string;
  branch_name: string | null;
  tags: string[];
}

export default function App() {
  const [view, setView] = useState<AppView>("loading");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [rightColumnVisible, setRightColumnVisible] = useState(true);
  const [tickets, setTickets] = useState<TicketCard[]>([]);
  const [activeTicketId, setActiveTicketId] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("has_repos")
      .then((hasRepos) => {
        setView(hasRepos ? "main" : "onboarding");
      })
      .catch(() => {
        setView("onboarding");
      });
  }, []);

  // Fetch tickets when main view loads, then poll every 30s
  useEffect(() => {
    if (view !== "main") return;

    const fetchTickets = () => {
      invoke<TicketCard[]>("fetch_linear_tickets")
        .then(setTickets)
        .catch((e) => console.error("Failed to fetch tickets:", e));
    };

    fetchTickets();
    const interval = setInterval(fetchTickets, 30000);
    return () => clearInterval(interval);
  }, [view]);

  // Listen for macOS menu events
  useEffect(() => {
    const unlisten1 = listen("open_settings", () => setSettingsOpen(true));
    const unlisten2 = listen("toggle_right_column", () =>
      setRightColumnVisible((v) => !v)
    );
    return () => {
      unlisten1.then((f) => f());
      unlisten2.then((f) => f());
    };
  }, []);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.metaKey && e.key === ",") {
        e.preventDefault();
        setSettingsOpen(true);
      }
      if (e.metaKey && e.key === "b") {
        e.preventDefault();
        setRightColumnVisible((v) => !v);
      }
      if (e.key === "Escape") {
        setSettingsOpen(false);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

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

  return (
    <div className="flex h-screen flex-col bg-background">
      <div className="flex flex-1 min-h-0">
        {/* Left — Board (fixed 280px) */}
        <div className="w-[280px] min-w-[260px] shrink-0 border-r border-border bg-background overflow-hidden">
          <Board
            tickets={tickets}
            activeTicketId={activeTicketId}
            onSelectTicket={setActiveTicketId}
          />
        </div>

        {/* Middle — Plan or Session (flexible) */}
        <div className="flex-1 min-w-0 bg-background">
          <MiddleColumn activeTicket={activeTicket} />
        </div>

        {/* Right — Local (fixed 400px) */}
        {rightColumnVisible && (
          <div className="w-[400px] min-w-[380px] shrink-0 border-l border-border bg-background overflow-hidden">
            <RightColumn />
          </div>
        )}
      </div>

      <SettingsPanel
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onRerunSetup={handleRerunSetup}
      />
    </div>
  );
}
