import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Panel,
  Group as PanelGroup,
  Separator as PanelResizeHandle,
} from "react-resizable-panels";
import { Board } from "@/components/board/Board";
import { MiddleColumn } from "@/components/middle/MiddleColumn";
import { RightColumn } from "@/components/right/RightColumn";
import { Footer } from "@/components/Footer";
import { Onboarding } from "@/components/onboarding/Onboarding";
import { SettingsPanel } from "@/components/settings/SettingsPanel";

type AppView = "loading" | "onboarding" | "main";

export default function App() {
  const [view, setView] = useState<AppView>("loading");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [rightColumnVisible, setRightColumnVisible] = useState(true);

  useEffect(() => {
    invoke<boolean>("has_repos")
      .then((hasRepos) => {
        setView(hasRepos ? "main" : "onboarding");
      })
      .catch(() => {
        setView("onboarding");
      });
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
      {/* Three-column layout */}
      <PanelGroup orientation="horizontal" className="flex-1">
        {/* Left — Board */}
        <Panel
          defaultSize={20}
          minSize={15}
          maxSize={30}
          className="border-r border-border bg-background"
        >
          <Board />
        </Panel>

        <PanelResizeHandle className="w-px bg-border hover:bg-primary transition-colors duration-75" />

        {/* Middle — Plan or Session */}
        <Panel minSize={30} className="bg-background">
          <MiddleColumn />
        </Panel>

        {rightColumnVisible && (
          <>
            <PanelResizeHandle className="w-px bg-border hover:bg-primary transition-colors duration-75" />

            {/* Right — Local */}
            <Panel
              defaultSize={28}
              minSize={20}
              maxSize={40}
              className="border-l border-border bg-background"
            >
              <RightColumn />
            </Panel>
          </>
        )}
      </PanelGroup>

      {/* Footer */}
      <Footer
        onOpenSettings={() => setSettingsOpen(true)}
        rightColumnVisible={rightColumnVisible}
        onToggleRightColumn={() => setRightColumnVisible((v) => !v)}
      />

      {/* Settings modal */}
      <SettingsPanel
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onRerunSetup={handleRerunSetup}
      />
    </div>
  );
}
