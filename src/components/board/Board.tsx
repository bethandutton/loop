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

export function Board() {
  return (
    <div className="flex h-full flex-col">
      {/* Top bar */}
      <div className="titlebar-drag-region flex h-8 shrink-0 items-center justify-between border-b border-border px-3">
        <span className="titlebar-no-drag text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
          Loop
        </span>
        <button className="titlebar-no-drag flex h-5 w-5 items-center justify-center rounded text-muted-foreground hover:bg-surface-elevated hover:text-foreground">
          <span className="text-sm leading-none">+</span>
        </button>
      </div>

      {/* Board columns */}
      <div className="flex-1 overflow-y-auto" style={{ padding: "var(--space-list-padding)" }}>
        <div className="flex flex-col" style={{ gap: "var(--space-section-gap)" }}>
          {COLUMNS.map((col) => (
            <BoardColumn key={col.key} label={col.label} count={0} />
          ))}
        </div>
      </div>
    </div>
  );
}

function BoardColumn({ label, count }: { label: string; count: number }) {
  return (
    <div>
      <div className="flex items-center justify-between px-1 pb-1">
        <span className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
          {label}
        </span>
        {count > 0 && (
          <span className="text-[11px] text-muted-foreground">{count}</span>
        )}
      </div>
      <div className="text-[11px] text-muted-foreground/50 px-1">—</div>
    </div>
  );
}
