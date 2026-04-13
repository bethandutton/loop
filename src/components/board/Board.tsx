import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus } from "lucide-react";
import type { TicketCard } from "@/App";

const COLUMNS = [
  { key: "backlog", label: "Backlog" },
  { key: "todo", label: "To do" },
  { key: "planning", label: "Planning" },
  { key: "in_progress", label: "In progress" },
  { key: "ready_to_test", label: "Ready to test" },
  { key: "in_review", label: "In review" },
  { key: "attention_required", label: "Attention required" },
  { key: "ready_to_merge", label: "Ready to merge" },
  { key: "done", label: "Done" },
];

const PRIORITY_LABELS: Record<number, string> = {
  0: "",
  1: "Urgent",
  2: "High",
  3: "Medium",
  4: "Low",
};

interface BoardProps {
  tickets: TicketCard[];
  activeTicketId: string | null;
  onSelectTicket: (id: string) => void;
}

interface RepoInfo {
  name: string;
}

export function Board({ tickets, activeTicketId, onSelectTicket }: BoardProps) {
  const [repoName, setRepoName] = useState("Loop");

  useEffect(() => {
    invoke<RepoInfo | null>("get_active_repo").then((repo) => {
      if (repo) setRepoName(repo.name);
    }).catch(() => {});
  }, []);

  return (
    <div className="flex h-full flex-col">
      {/* Top bar — project name + add button */}
      <div className="titlebar-drag-region flex h-10 shrink-0 items-center justify-between border-b border-border px-3 pt-5">
        <span className="titlebar-no-drag text-[13px] font-semibold text-foreground truncate">
          {repoName}
        </span>
        <button
          className="titlebar-no-drag flex h-6 w-6 items-center justify-center rounded hover:bg-surface-elevated text-muted-foreground hover:text-foreground transition-colors duration-75"
          title="New ticket (⌘N)"
        >
          <Plus size={14} />
        </button>
      </div>

      {/* Board columns */}
      <div className="flex-1 overflow-y-auto" style={{ padding: "var(--space-list-padding)" }}>
        <div className="flex flex-col" style={{ gap: "var(--space-section-gap)" }}>
          {COLUMNS.map((col) => {
            const colTickets = tickets.filter((t) => t.status === col.key);
            return (
              <BoardColumn
                key={col.key}
                label={col.label}
                tickets={colTickets}
                activeTicketId={activeTicketId}
                onSelectTicket={onSelectTicket}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

function BoardColumn({
  label,
  tickets,
  activeTicketId,
  onSelectTicket,
}: {
  label: string;
  tickets: TicketCard[];
  activeTicketId: string | null;
  onSelectTicket: (id: string) => void;
}) {
  return (
    <div>
      <div className="flex items-center justify-between px-1 pb-1">
        <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
          {label}
        </span>
        {tickets.length > 0 && (
          <span className="text-[11px] text-muted-foreground">{tickets.length}</span>
        )}
      </div>
      {tickets.length === 0 ? (
        <div className="text-[11px] text-muted-foreground/50 px-1">—</div>
      ) : (
        <div className="flex flex-col" style={{ gap: "var(--space-card-gap)" }}>
          {tickets.map((ticket) => (
            <TicketCardView
              key={ticket.id}
              ticket={ticket}
              isActive={ticket.id === activeTicketId}
              onClick={() => onSelectTicket(ticket.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function TicketCardView({
  ticket,
  isActive,
  onClick,
}: {
  ticket: TicketCard;
  isActive: boolean;
  onClick: () => void;
}) {
  const priorityLabel = PRIORITY_LABELS[ticket.priority] || "";

  return (
    <button
      onClick={onClick}
      className={`w-full text-left rounded-md border transition-colors duration-75 ${
        isActive
          ? "border-l-2 border-l-primary border-y-border border-r-border bg-surface-elevated"
          : "border-border bg-surface hover:bg-surface-elevated"
      }`}
      style={{
        padding: "var(--space-card-py) var(--space-card-px)",
      }}
    >
      {/* Row 1: ID + priority */}
      <div className="flex items-center justify-between mb-0.5">
        <span className="font-mono text-[11px] text-muted-foreground truncate">
          {ticket.id.slice(0, 8)}
        </span>
        {priorityLabel && (
          <span
            className={`text-[10px] font-medium px-1.5 py-0.5 rounded ${
              ticket.priority <= 1
                ? "bg-destructive/20 text-destructive"
                : ticket.priority === 2
                ? "bg-warning/20 text-warning"
                : "bg-surface-elevated text-muted-foreground"
            }`}
          >
            {priorityLabel}
          </span>
        )}
      </div>
      {/* Row 2: Title */}
      <p className="text-[13px] text-foreground leading-snug line-clamp-2">
        {ticket.title}
      </p>
    </button>
  );
}
