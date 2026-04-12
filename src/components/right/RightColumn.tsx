export function RightColumn() {
  return (
    <div className="flex h-full flex-col">
      {/* Branch context bar */}
      <div className="titlebar-drag-region flex h-7 shrink-0 items-center border-b border-border px-3">
        <span className="titlebar-no-drag font-mono text-xs text-muted-foreground">
          No branch
        </span>
      </div>

      {/* Service runner */}
      <div className="flex-1 flex items-center justify-center">
        <p className="text-sm text-muted-foreground">
          No services configured.
        </p>
      </div>

      {/* Browser preview placeholder */}
      <div className="h-1/2 border-t border-border bg-surface flex items-center justify-center">
        <p className="text-xs text-muted-foreground">Browser preview</p>
      </div>
    </div>
  );
}
