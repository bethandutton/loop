import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus, Search, X, Filter, Check, AlertTriangle, Copy, ExternalLink, ArrowUpDown, Loader2, GitBranch, ChevronRight as ChevronRightIcon, List, LayoutGrid, SquareKanban, FileText } from "lucide-react";
import type { TicketCard } from "@/App";

// Status config: priority for sort order, icon style, color
type StatusIconType = "dashed" | "empty" | "quarter" | "half" | "three-quarter" | "full" | "alert";
interface StatusDef {
  label: string;
  sortOrder: number;
  icon: StatusIconType;
  color: string;
}
const STATUS_CONFIG: Record<string, StatusDef> = {
  attention_required: { label: "Attention required", sortOrder: 0, icon: "alert",         color: "#e5484d" },
  ready_to_merge:     { label: "Ready to merge",     sortOrder: 1, icon: "three-quarter", color: "#30a46c" },
  in_progress:        { label: "In progress",        sortOrder: 2, icon: "quarter",       color: "#e5a83b" },
  ready_to_test:      { label: "Ready to test",      sortOrder: 3, icon: "half",          color: "#e5a83b" },
  in_review:          { label: "In review",           sortOrder: 4, icon: "three-quarter", color: "#30a46c" },
  planning:           { label: "Planning",            sortOrder: 5, icon: "empty",         color: "#8b8d98" },
  todo:               { label: "To do",               sortOrder: 6, icon: "empty",         color: "#8b8d98" },
  backlog:            { label: "Backlog",              sortOrder: 7, icon: "dashed",        color: "#8b8d98" },
  done:               { label: "Done",                 sortOrder: 8, icon: "full",          color: "#6e6ade" },
};

function StatusCircle({ icon, color, size = 14 }: { icon: StatusIconType; color: string; size?: number }) {
  const r = 5;
  const cx = 7;
  const cy = 7;
  const circumference = 2 * Math.PI * r;

  if (icon === "alert") {
    return (
      <svg width={size} height={size} viewBox="0 0 14 14" className="shrink-0">
        <circle cx={cx} cy={cy} r={r} fill={color} />
        <line x1="7" y1="4.5" x2="7" y2="7.5" stroke="white" strokeWidth="1.5" strokeLinecap="round" />
        <circle cx={7} cy={9.5} r={0.75} fill="white" />
      </svg>
    );
  }

  if (icon === "full") {
    return (
      <svg width={size} height={size} viewBox="0 0 14 14" className="shrink-0">
        <circle cx={cx} cy={cy} r={r} fill={color} />
        <path d="M5.5 7l1.2 1.2 2.3-2.4" stroke="white" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" fill="none" />
      </svg>
    );
  }

  if (icon === "dashed") {
    return (
      <svg width={size} height={size} viewBox="0 0 14 14" className="shrink-0">
        <circle cx={cx} cy={cy} r={r} fill="none" stroke={color} strokeWidth="1.5" strokeDasharray="2.5 2.5" opacity="0.5" />
      </svg>
    );
  }

  if (icon === "empty") {
    return (
      <svg width={size} height={size} viewBox="0 0 14 14" className="shrink-0">
        <circle cx={cx} cy={cy} r={r} fill="none" stroke={color} strokeWidth="1.5" opacity="0.4" />
      </svg>
    );
  }

  const fillPct = icon === "quarter" ? 0.25 : icon === "half" ? 0.5 : 0.75;
  const dashLen = circumference * fillPct;
  const gapLen = circumference - dashLen;

  return (
    <svg width={size} height={size} viewBox="0 0 14 14" className="shrink-0">
      <circle cx={cx} cy={cy} r={r} fill="none" stroke={color} strokeWidth="1.5" opacity="0.2" />
      <circle
        cx={cx} cy={cy} r={r}
        fill="none"
        stroke={color}
        strokeWidth="1.5"
        strokeDasharray={`${dashLen} ${gapLen}`}
        strokeLinecap="round"
        transform={`rotate(-90 ${cx} ${cy})`}
      />
    </svg>
  );
}

const PRIORITY_LABELS: Record<number, string> = {
  0: "",
  1: "Urgent",
  2: "High",
  3: "Medium",
  4: "Low",
};

type SortOption = "status" | "priority" | "created" | "updated" | "title";
const SORT_OPTIONS: { key: SortOption; label: string }[] = [
  { key: "status", label: "Status" },
  { key: "priority", label: "Priority" },
  { key: "created", label: "Date created" },
  { key: "updated", label: "Last updated" },
  { key: "title", label: "Title" },
];

interface BoardProps {
  tickets: TicketCard[];
  activeTicketId: string | null;
  onSelectTicket: (id: string) => void;
}

interface RepoInfo {
  name: string;
}

export function Board({ tickets, activeTicketId, onSelectTicket }: BoardProps) {
  const [repoName, setRepoName] = useState("Herd");
  const [viewMode, setViewMode] = useState<"list" | "board">("list");
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [filterOpen, setFilterOpen] = useState(false);
  const [activeFilters, setActiveFilters] = useState<Set<string>>(new Set([
    "attention_required", "ready_to_merge", "in_progress", "ready_to_test",
    "in_review", "planning", "todo",
  ]));
  const [sortBy, setSortBy] = useState<SortOption>("status");
  const [createMenuOpen, setCreateMenuOpen] = useState(false);
  const [newTicketOpen, setNewTicketOpen] = useState(false);
  const [newTicketTitle, setNewTicketTitle] = useState("");
  const [newTicketCreating, setNewTicketCreating] = useState(false);
  const [newTicketMode, setNewTicketMode] = useState<"linear" | "draft">("linear");
  const newTicketRef = useRef<HTMLInputElement>(null);
  const createMenuRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const filterRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<RepoInfo | null>("get_active_repo").then((repo) => {
      if (repo) setRepoName(repo.name);
    }).catch(() => {});
  }, []);

  useEffect(() => {
    if (searchOpen) searchInputRef.current?.focus();
  }, [searchOpen]);

  useEffect(() => {
    if (newTicketOpen) newTicketRef.current?.focus();
  }, [newTicketOpen]);

  const handleCreateTicket = async () => {
    if (!newTicketTitle.trim() || newTicketCreating) return;
    setNewTicketCreating(true);
    try {
      await invoke("create_linear_ticket", {
        title: newTicketTitle.trim(),
        description: "",
        priority: 0,
      });
      setNewTicketTitle("");
      setNewTicketOpen(false);
      // Refresh tickets
      const updated = await invoke<TicketCard[]>("fetch_linear_tickets");
      // App.tsx handles this via event, but also update immediately if parent passes a callback
    } catch (e) {
      console.error("Failed to create ticket:", e);
    } finally {
      setNewTicketCreating(false);
    }
  };

  // Close filter menu on outside click
  useEffect(() => {
    if (!filterOpen) return;
    const handler = (e: MouseEvent) => {
      if (filterRef.current && !filterRef.current.contains(e.target as Node)) {
        setFilterOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [filterOpen]);

  const toggleFilter = (status: string) => {
    setActiveFilters((prev) => {
      const next = new Set(prev);
      if (next.has(status)) {
        next.delete(status);
      } else {
        next.add(status);
      }
      return next;
    });
  };

  const query = searchQuery.toLowerCase();
  let filteredTickets = tickets;

  // Text search
  if (query) {
    filteredTickets = filteredTickets.filter(
      (t) =>
        t.title.toLowerCase().includes(query) ||
        t.identifier.toLowerCase().includes(query)
    );
  }

  // Status filter
  if (activeFilters.size > 0) {
    filteredTickets = filteredTickets.filter((t) => activeFilters.has(t.status));
  }

  // Sort tickets
  const sortedTickets = [...filteredTickets].sort((a, b) => {
    switch (sortBy) {
      case "priority":
        // Lower number = higher priority (1=urgent, 4=low, 0=none goes last)
        return (a.priority || 5) - (b.priority || 5);
      case "created":
        return b.created_at.localeCompare(a.created_at);
      case "updated":
        return b.updated_at.localeCompare(a.updated_at);
      case "title":
        return a.title.localeCompare(b.title);
      case "status":
      default:
        return (STATUS_CONFIG[a.status]?.sortOrder ?? 99) - (STATUS_CONFIG[b.status]?.sortOrder ?? 99);
    }
  });

  // All statuses for filter menu, sorted by priority
  const allStatuses = Object.entries(STATUS_CONFIG)
    .sort((a, b) => a[1].sortOrder - b[1].sortOrder);

  return (
    <div className="flex h-full flex-col">
      {/* Top bar */}
      <div className="titlebar-drag-region flex h-16 shrink-0 items-end justify-between px-3 pb-2">
        <span className="titlebar-no-drag text-[13px] font-semibold text-foreground truncate">
          {repoName}
        </span>
        <div className="titlebar-no-drag flex items-center gap-1">
          {/* View toggle */}
          <button
            onClick={() => setViewMode(viewMode === "list" ? "board" : "list")}
            className="flex h-6 w-6 items-center justify-center rounded hover:bg-surface-elevated text-muted-foreground hover:text-foreground transition-colors duration-75"
            title={viewMode === "list" ? "Switch to board view" : "Switch to list view"}
          >
            {viewMode === "list" ? <LayoutGrid size={14} /> : <List size={14} />}
          </button>
          {/* Filter */}
          <div className="relative" ref={filterRef}>
            <button
              onClick={() => setFilterOpen(!filterOpen)}
              className={`flex h-6 w-6 items-center justify-center rounded transition-colors duration-75 ${
                activeFilters.size > 0
                  ? "bg-primary/10 text-primary"
                  : filterOpen
                    ? "bg-primary/10 text-primary"
                    : "hover:bg-surface-elevated text-muted-foreground hover:text-foreground"
              }`}
              title="Filter by status"
            >
              <Filter size={14} />
            </button>
            {filterOpen && (
              <div className="absolute right-0 top-7 z-50 w-48 rounded-md border border-border bg-surface-elevated py-1 shadow-lg">
                {/* Sort options */}
                <div className="px-2.5 py-1 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">Sort by</div>
                {SORT_OPTIONS.map((opt) => (
                  <button
                    key={opt.key}
                    onClick={() => setSortBy(opt.key)}
                    className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left hover:bg-primary/5 transition-colors duration-75"
                  >
                    <ArrowUpDown size={12} className="text-muted-foreground shrink-0" />
                    <span className="flex-1 text-xs text-foreground">{opt.label}</span>
                    {sortBy === opt.key && <Check size={12} className="text-primary shrink-0" />}
                  </button>
                ))}

                <div className="my-1 border-t border-border" />

                {/* Status filters */}
                <div className="px-2.5 py-1 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">Filter by status</div>
                {allStatuses.map(([key, config]) => {
                  const count = tickets.filter((t) => t.status === key).length;
                  const isActive = activeFilters.has(key);
                  return (
                    <button
                      key={key}
                      onClick={() => toggleFilter(key)}
                      className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left hover:bg-primary/5 transition-colors duration-75"
                    >
                      <StatusCircle icon={config.icon} color={config.color} />
                      <span className="flex-1 text-xs text-foreground">{config.label}</span>
                      <span className="text-[11px] text-muted-foreground">{count}</span>
                      {isActive && <Check size={12} className="text-primary shrink-0" />}
                    </button>
                  );
                })}
                {activeFilters.size > 0 && (
                  <>
                    <div className="my-1 border-t border-border" />
                    <button
                      onClick={() => setActiveFilters(new Set())}
                      className="flex w-full items-center px-2.5 py-1.5 text-left hover:bg-primary/5 transition-colors duration-75"
                    >
                      <span className="text-xs text-muted-foreground">Clear filters</span>
                    </button>
                  </>
                )}
              </div>
            )}
          </div>
          {/* Search */}
          <button
            onClick={() => {
              setSearchOpen(!searchOpen);
              if (searchOpen) setSearchQuery("");
            }}
            className={`flex h-6 w-6 items-center justify-center rounded transition-colors duration-75 ${
              searchOpen
                ? "bg-primary/10 text-primary"
                : "hover:bg-surface-elevated text-muted-foreground hover:text-foreground"
            }`}
            title="Search (⌘K)"
          >
            <Search size={14} />
          </button>
          {/* New ticket dropdown */}
          <div className="relative" ref={createMenuRef}>
            <button
              onClick={() => setCreateMenuOpen(!createMenuOpen)}
              className={`flex h-6 w-6 items-center justify-center rounded transition-colors duration-75 ${
                createMenuOpen
                  ? "bg-primary/10 text-primary"
                  : "hover:bg-surface-elevated text-muted-foreground hover:text-foreground"
              }`}
              title="New (⌘N)"
            >
              <Plus size={14} />
            </button>
            {createMenuOpen && (
              <div className="absolute right-0 top-7 z-50 w-48 rounded-md border border-border bg-surface-elevated py-1 shadow-lg">
                <button
                  onClick={() => {
                    setNewTicketMode("linear");
                    setNewTicketOpen(true);
                    setCreateMenuOpen(false);
                  }}
                  className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-foreground hover:bg-primary/5 transition-colors duration-75"
                >
                  <SquareKanban size={12} className="text-muted-foreground" />
                  New Linear ticket
                </button>
                <button
                  onClick={() => {
                    setNewTicketMode("draft");
                    setNewTicketOpen(true);
                    setCreateMenuOpen(false);
                  }}
                  className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-foreground hover:bg-primary/5 transition-colors duration-75"
                >
                  <FileText size={12} className="text-muted-foreground" />
                  New local draft
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Search bar */}
      {searchOpen && (
        <div className="shrink-0 px-3 pb-2">
          <div className="flex items-center gap-2 rounded-md bg-surface px-2 py-1.5">
            <Search size={12} className="shrink-0 text-muted-foreground" />
            <input
              ref={searchInputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape") {
                  setSearchQuery("");
                  setSearchOpen(false);
                }
              }}
              placeholder="Filter tickets..."
              className="flex-1 bg-transparent text-xs text-foreground placeholder:text-muted-foreground/50 outline-none"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery("")}
                className="text-muted-foreground hover:text-foreground"
              >
                <X size={12} />
              </button>
            )}
          </div>
        </div>
      )}

      {/* New ticket form */}
      {newTicketOpen && (
        <div className="shrink-0 px-3 pb-2">
          <div className="rounded-md bg-surface px-2 py-1.5 space-y-1.5">
            <div className="flex items-center gap-1.5">
              <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
                {newTicketMode === "linear" ? "Linear ticket" : "Local draft"}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <input
                ref={newTicketRef}
                type="text"
                value={newTicketTitle}
                onChange={(e) => setNewTicketTitle(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleCreateTicket();
                  if (e.key === "Escape") {
                    setNewTicketTitle("");
                    setNewTicketOpen(false);
                  }
                }}
                placeholder={newTicketMode === "linear" ? "Ticket title..." : "Draft title..."}
                className="flex-1 bg-transparent text-xs text-foreground placeholder:text-muted-foreground/50 outline-none"
                disabled={newTicketCreating}
              />
              {newTicketCreating ? (
                <Loader2 size={12} className="animate-spin text-muted-foreground" />
              ) : (
                <button
                  onClick={handleCreateTicket}
                  disabled={!newTicketTitle.trim()}
                  className="text-primary hover:text-primary/80 disabled:text-muted-foreground/30"
                >
                  <Plus size={14} />
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Ticket list / board */}
      <div className="flex-1 overflow-y-auto" style={{ padding: "var(--space-list-padding)" }}>
        {viewMode === "list" ? (
          <div className="flex flex-col" style={{ gap: "var(--space-card-gap)" }}>
            {sortedTickets.map((ticket) => (
              <TicketCardView
                key={ticket.id}
                ticket={ticket}
                isActive={ticket.id === activeTicketId}
                onClick={() => onSelectTicket(ticket.id)}
              />
            ))}
            {sortedTickets.length === 0 && (
              <p className="text-xs text-muted-foreground/50 text-center py-8">
                No tickets match.
              </p>
            )}
          </div>
        ) : (
          /* Kanban board view */
          <div className="flex flex-col gap-3">
            {Object.entries(STATUS_CONFIG)
              .sort((a, b) => a[1].sortOrder - b[1].sortOrder)
              .map(([key, config]) => {
                const colTickets = sortedTickets.filter((t) => t.status === key);
                if (colTickets.length === 0) return null;
                return (
                  <BoardSection
                    key={key}
                    statusKey={key}
                    config={config}
                    tickets={colTickets}
                    activeTicketId={activeTicketId}
                    onSelectTicket={onSelectTicket}
                  />
                );
              })}
          </div>
        )}
      </div>
    </div>
  );
}

function BoardSection({
  statusKey,
  config,
  tickets,
  activeTicketId,
  onSelectTicket,
}: {
  statusKey: string;
  config: StatusDef;
  tickets: TicketCard[];
  activeTicketId: string | null;
  onSelectTicket: (id: string) => void;
}) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div>
      <button
        onClick={() => setCollapsed(!collapsed)}
        className="flex w-full items-center gap-1.5 px-1 py-1 hover:text-foreground transition-colors duration-75"
      >
        <StatusCircle icon={config.icon} color={config.color} />
        <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
          {config.label}
        </span>
        <span className="text-[11px] text-muted-foreground ml-auto">{tickets.length}</span>
      </button>
      {!collapsed && (
        <div className="flex flex-col" style={{ gap: "var(--space-card-gap)" }}>
          {tickets.map((ticket) => (
            <TicketCardView
              key={ticket.id}
              ticket={ticket}
              isActive={ticket.id === activeTicketId}
              onClick={() => onSelectTicket(ticket.id)}
              hideStatusIcon
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PriorityBars({ priority }: { priority: number }) {
  if (priority === 0) return null;

  const label = PRIORITY_LABELS[priority];

  if (priority === 1) {
    return (
      <span className="relative group">
        <AlertTriangle size={12} className="text-destructive shrink-0" />
        <span className="pointer-events-none absolute right-0 bottom-full mb-1.5 whitespace-nowrap rounded bg-zinc-900 dark:bg-zinc-800 px-2 py-1 text-[11px] text-white shadow-lg opacity-0 group-hover:opacity-100 transition-opacity duration-100 z-50">
          {label}
        </span>
      </span>
    );
  }

  const filled = priority === 2 ? 3 : priority === 3 ? 2 : 1;
  const barColor = priority === 2 ? "bg-warning" : "bg-muted-foreground";

  return (
    <span className="relative group">
      <div className="flex items-end gap-[2px] h-3 shrink-0">
        {[1, 2, 3].map((i) => (
          <div
            key={i}
            className={`w-[3px] rounded-sm ${i <= filled ? barColor : "bg-border"}`}
            style={{ height: `${4 + i * 3}px` }}
          />
        ))}
      </div>
      <span className="pointer-events-none absolute right-0 bottom-full mb-1.5 whitespace-nowrap rounded bg-zinc-900 dark:bg-zinc-800 px-2 py-1 text-[11px] text-white shadow-lg opacity-0 group-hover:opacity-100 transition-opacity duration-100 z-50">
        {label}
      </span>
    </span>
  );
}

function TicketCardView({
  ticket,
  isActive,
  onClick,
  hideStatusIcon,
}: {
  ticket: TicketCard;
  isActive: boolean;
  onClick: () => void;
  hideStatusIcon?: boolean;
}) {
  const statusDef = STATUS_CONFIG[ticket.status];
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!contextMenu) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [contextMenu]);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const copyId = () => {
    navigator.clipboard.writeText(ticket.identifier);
    setContextMenu(null);
  };

  const goToTicket = () => {
    // Linear URL format: https://linear.app/team/issue/IDENTIFIER
    window.open(`https://linear.app/issue/${ticket.identifier}`, "_blank");
    setContextMenu(null);
  };

  return (
    <>
      <button
        onClick={onClick}
        onContextMenu={handleContextMenu}
        className={`w-full text-left rounded-md transition-colors duration-75 ${
          isActive
            ? "bg-surface"
            : "hover:bg-surface/50"
        }`}
        style={{
          padding: "var(--space-card-py) var(--space-card-px)",
        }}
      >
        {/* Row 1: Status icon + ID + priority */}
        <div className="flex items-center justify-between mb-0.5">
          <div className="flex items-center gap-1.5 min-w-0">
            {statusDef && !hideStatusIcon && (
              <span className="relative group">
                <StatusCircle icon={statusDef.icon} color={statusDef.color} />
                <span className="pointer-events-none absolute left-full top-1/2 -translate-y-1/2 ml-1.5 whitespace-nowrap rounded bg-zinc-900 dark:bg-zinc-800 px-2 py-1 text-[11px] text-white shadow-lg opacity-0 group-hover:opacity-100 transition-opacity duration-100 z-50">
                  {statusDef.label}
                </span>
              </span>
            )}
            <span className="font-mono text-[11px] text-muted-foreground truncate">
              {ticket.identifier}
            </span>
          </div>
          <PriorityBars priority={ticket.priority} />
        </div>
        {/* Row 2: Title */}
        <p className="text-[13px] text-foreground leading-snug line-clamp-2">
          {ticket.title}
        </p>
        {/* Row 3: Branch name */}
        {ticket.branch_name && (
          <div className="flex items-center gap-1 mt-1">
            <GitBranch size={10} className="text-muted-foreground/50 shrink-0" />
            <span className="font-mono text-[10px] text-muted-foreground/50 truncate">
              {ticket.branch_name}
            </span>
          </div>
        )}
      </button>

      {/* Right-click context menu */}
      {contextMenu && (
        <div
          ref={menuRef}
          className="fixed z-50 w-52 rounded-md border border-border bg-surface-elevated py-1 shadow-lg"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            onClick={copyId}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-foreground hover:bg-primary/5 transition-colors duration-75"
          >
            <Copy size={12} className="text-muted-foreground" />
            Copy ID
          </button>
          <button
            onClick={goToTicket}
            className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-foreground hover:bg-primary/5 transition-colors duration-75"
          >
            <ExternalLink size={12} className="text-muted-foreground" />
            Open in Linear
          </button>

          <div className="my-1 border-t border-border" />

          {/* Status changes */}
          <div className="px-2.5 py-1 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">Move to</div>
          {Object.entries(STATUS_CONFIG)
            .sort((a, b) => a[1].sortOrder - b[1].sortOrder)
            .filter(([key]) => key !== ticket.status)
            .map(([key, config]) => (
              <button
                key={key}
                onClick={() => {
                  invoke("update_ticket_status", { ticketId: ticket.id, status: key }).catch(() => {});
                  setContextMenu(null);
                }}
                className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-foreground hover:bg-primary/5 transition-colors duration-75"
              >
                <StatusCircle icon={config.icon} color={config.color} />
                {config.label}
              </button>
            ))}

          <div className="my-1 border-t border-border" />

          {/* Priority changes */}
          <div className="px-2.5 py-1 text-[10px] uppercase tracking-wider text-muted-foreground font-medium">Priority</div>
          {[
            { value: 1, label: "Urgent" },
            { value: 2, label: "High" },
            { value: 3, label: "Medium" },
            { value: 4, label: "Low" },
            { value: 0, label: "None" },
          ].map((p) => (
            <button
              key={p.value}
              onClick={() => {
                invoke("update_ticket_priority", { ticketId: ticket.id, priority: p.value }).catch(() => {});
                setContextMenu(null);
              }}
              className="flex w-full items-center gap-2 px-2.5 py-1.5 text-left text-xs text-foreground hover:bg-primary/5 transition-colors duration-75"
            >
              <span className="w-3 text-center">{ticket.priority === p.value ? <Check size={10} className="text-primary" /> : null}</span>
              {p.label}
            </button>
          ))}
        </div>
      )}
    </>
  );
}
