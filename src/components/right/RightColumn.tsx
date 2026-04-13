import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { GitBranch, Play, Square, ChevronDown, ChevronRight, Globe } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TicketCard } from "@/App";

interface ServiceDef {
  name: string;
  command: string;
}

interface ServiceStatus {
  id: string;
  name: string;
  state: string;
}

interface RightColumnProps {
  activeTicket: TicketCard | null;
}

export function RightColumn({ activeTicket }: RightColumnProps) {
  const [currentBranch, setCurrentBranch] = useState<string | null>(null);
  const [services, setServices] = useState<ServiceDef[]>([]);
  const [runningServices, setRunningServices] = useState<ServiceStatus[]>([]);
  const [selectedServices, setSelectedServices] = useState<Set<string>>(new Set());
  const [switching, setSwitching] = useState(false);
  const [previewPort, setPreviewPort] = useState(3000);
  const [showPreview, setShowPreview] = useState(true);
  const [expandedService, setExpandedService] = useState<string | null>(null);

  // Load current branch and services
  useEffect(() => {
    invoke<string | null>("get_local_branch").then(setCurrentBranch).catch(() => {});
    invoke<ServiceDef[]>("detect_services").then(setServices).catch(() => {});
    invoke<ServiceStatus[]>("get_running_services").then(setRunningServices).catch(() => {});
    invoke<{ preview_port: number } | null>("get_active_repo").then((repo) => {
      if (repo) setPreviewPort(repo.preview_port);
    }).catch(() => {});
  }, []);

  // Poll running services
  useEffect(() => {
    const interval = setInterval(() => {
      invoke<ServiceStatus[]>("get_running_services").then(setRunningServices).catch(() => {});
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const handleSwitchBranch = async () => {
    if (!activeTicket?.branch_name) return;
    setSwitching(true);
    try {
      await invoke("switch_local_branch", { branchName: activeTicket.branch_name });
      setCurrentBranch(activeTicket.branch_name);
      // Reload services for new branch
      const svc = await invoke<ServiceDef[]>("detect_services");
      setServices(svc);
    } catch (e) {
      console.error("Failed to switch branch:", e);
    } finally {
      setSwitching(false);
    }
  };

  const toggleService = (name: string) => {
    setSelectedServices((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const handleRunServices = async () => {
    for (const name of selectedServices) {
      try {
        await invoke<string>("start_service", { scriptName: name });
      } catch (e) {
        console.error(`Failed to start ${name}:`, e);
      }
    }
    const status = await invoke<ServiceStatus[]>("get_running_services");
    setRunningServices(status);
  };

  const handleStopAll = async () => {
    try {
      await invoke("stop_all_services");
      setRunningServices([]);
    } catch (e) {
      console.error("Failed to stop services:", e);
    }
  };

  const isRunning = runningServices.length > 0;

  return (
    <div className="flex h-full flex-col">
      {/* Branch context bar */}
      <div className="titlebar-drag-region flex h-14 shrink-0 items-end pb-2 px-3">
        <div className="titlebar-no-drag flex items-center gap-2 min-w-0 w-full">
          <GitBranch size={12} className="text-muted-foreground shrink-0" />
          <span className="font-mono text-xs text-foreground truncate">
            {currentBranch || "No branch"}
          </span>
          {activeTicket?.branch_name && activeTicket.branch_name !== currentBranch && (
            <Button
              variant="ghost"
              size="sm"
              onClick={handleSwitchBranch}
              disabled={switching}
              className="ml-auto shrink-0 text-[11px]"
            >
              {switching ? "Switching..." : `Switch to ${activeTicket.branch_name}`}
            </Button>
          )}
        </div>
      </div>

      {/* Service runner */}
      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-2">
        <div className="flex items-center justify-between mb-1">
          <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
            Scripts
          </span>
          <span className="text-[11px] text-muted-foreground">{services.length}</span>
        </div>

        {services.length === 0 ? (
          <p className="text-xs text-muted-foreground/50">No scripts found</p>
        ) : (
          <>
            {services.map((svc) => {
              const running = runningServices.find((r) => r.name === svc.name);
              const isExpanded = expandedService === svc.name;
              return (
                <div key={svc.name}>
                  <div className="flex items-center gap-2 py-1">
                    <input
                      type="checkbox"
                      checked={selectedServices.has(svc.name)}
                      onChange={() => toggleService(svc.name)}
                      className="h-3 w-3 rounded border-border"
                    />
                    <button
                      onClick={() => setExpandedService(isExpanded ? null : svc.name)}
                      className="flex items-center gap-1 flex-1 text-left min-w-0"
                    >
                      {isExpanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
                      <span className="font-mono text-xs text-foreground truncate">{svc.name}</span>
                    </button>
                    {running && (
                      <span className="h-2 w-2 rounded-full bg-success shrink-0" title="Running" />
                    )}
                  </div>
                  {isExpanded && (
                    <div className="ml-7 mb-1">
                      <p className="font-mono text-[11px] text-muted-foreground break-all">
                        {svc.command}
                      </p>
                    </div>
                  )}
                </div>
              );
            })}

            <div className="pt-2">
              {isRunning ? (
                <Button size="sm" variant="outline" onClick={handleStopAll} className="w-full">
                  <Square size={12} className="mr-1" />
                  Stop all
                </Button>
              ) : (
                <Button
                  size="sm"
                  onClick={handleRunServices}
                  disabled={selectedServices.size === 0}
                  className="w-full"
                >
                  <Play size={12} className="mr-1" />
                  Run selected
                </Button>
              )}
            </div>
          </>
        )}
      </div>

      {/* Browser preview */}
      <div className="h-1/2 border-t border-border flex flex-col">
        <div className="flex items-center justify-between px-3 py-1.5 shrink-0">
          <div className="flex items-center gap-1.5">
            <Globe size={11} className="text-muted-foreground" />
            <span className="text-[11px] text-muted-foreground font-mono">
              localhost:{previewPort}
            </span>
          </div>
          <button
            onClick={() => setShowPreview(!showPreview)}
            className="text-[11px] text-muted-foreground hover:text-foreground"
          >
            {showPreview ? "Hide" : "Show"}
          </button>
        </div>
        {showPreview && (
          <iframe
            src={`http://localhost:${previewPort}`}
            className="flex-1 w-full border-0 bg-white"
            title="Browser preview"
          />
        )}
      </div>
    </div>
  );
}
