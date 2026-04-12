export function MiddleColumn() {
  return (
    <div className="flex h-full flex-col">
      {/* Top toolbar */}
      <div className="titlebar-drag-region flex h-8 shrink-0 items-center border-b border-border px-3">
        <span className="titlebar-no-drag text-[13px] text-muted-foreground">
          &nbsp;
        </span>
      </div>

      {/* Empty state */}
      <div className="flex flex-1 items-center justify-center">
        <p className="text-sm text-muted-foreground">
          Pick a ticket to get started.
        </p>
      </div>
    </div>
  );
}
