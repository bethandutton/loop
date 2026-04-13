import { useState, useEffect, useRef, useMemo } from "react";
import { Search, ArrowRight, Settings, Plus, Eye, EyeOff, Sun, Moon } from "lucide-react";
import type { TicketCard } from "@/App";

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
  tickets: TicketCard[];
  onSelectTicket: (id: string) => void;
  onOpenSettings: () => void;
  onToggleRightColumn: () => void;
  onNewTicket: () => void;
}

interface CommandItem {
  id: string;
  label: string;
  sublabel?: string;
  icon?: React.ReactNode;
  action: () => void;
}

export function CommandPalette({
  open,
  onClose,
  tickets,
  onSelectTicket,
  onOpenSettings,
  onToggleRightColumn,
  onNewTicket,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  const actions: CommandItem[] = useMemo(() => [
    { id: "settings", label: "Open Settings", sublabel: "⌘,", icon: <Settings size={14} />, action: () => { onOpenSettings(); onClose(); } },
    { id: "new-ticket", label: "New Ticket", sublabel: "⌘N", icon: <Plus size={14} />, action: () => { onNewTicket(); onClose(); } },
    { id: "toggle-right", label: "Toggle Right Panel", sublabel: "⌘B", icon: <Eye size={14} />, action: () => { onToggleRightColumn(); onClose(); } },
  ], [onOpenSettings, onNewTicket, onToggleRightColumn, onClose]);

  const filteredItems: CommandItem[] = useMemo(() => {
    const q = query.toLowerCase();

    const ticketItems: CommandItem[] = tickets
      .filter((t) =>
        !q ||
        t.title.toLowerCase().includes(q) ||
        t.identifier.toLowerCase().includes(q)
      )
      .slice(0, 20)
      .map((t) => ({
        id: t.id,
        label: t.title,
        sublabel: t.identifier,
        icon: <ArrowRight size={14} className="text-muted-foreground" />,
        action: () => { onSelectTicket(t.id); onClose(); },
      }));

    const actionItems = actions.filter((a) =>
      !q || a.label.toLowerCase().includes(q)
    );

    return q ? [...ticketItems, ...actionItems] : [...ticketItems, ...actionItems];
  }, [query, tickets, actions, onSelectTicket, onClose]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filteredItems.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === "Enter" && filteredItems[selectedIndex]) {
      e.preventDefault();
      filteredItems[selectedIndex].action();
    } else if (e.key === "Escape") {
      onClose();
    }
  };

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current) {
      const selected = listRef.current.children[selectedIndex] as HTMLElement;
      if (selected) {
        selected.scrollIntoView({ block: "nearest" });
      }
    }
  }, [selectedIndex]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]" onClick={onClose}>
      <div
        className="w-full max-w-md rounded-xl border border-border bg-surface-elevated shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search input */}
        <div className="flex items-center gap-2 px-3 py-2.5 border-b border-border">
          <Search size={14} className="text-muted-foreground shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search tickets or run a command..."
            className="flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground/50 outline-none"
          />
        </div>

        {/* Results */}
        <div ref={listRef} className="max-h-[300px] overflow-y-auto py-1">
          {filteredItems.length === 0 && (
            <p className="text-xs text-muted-foreground/50 text-center py-6">No results</p>
          )}
          {filteredItems.map((item, i) => (
            <button
              key={item.id}
              onClick={item.action}
              className={`flex w-full items-center gap-2.5 px-3 py-2 text-left transition-colors duration-50 ${
                i === selectedIndex ? "bg-primary/10" : "hover:bg-surface"
              }`}
            >
              <span className="shrink-0 text-muted-foreground">{item.icon}</span>
              <span className="flex-1 text-sm text-foreground truncate">{item.label}</span>
              {item.sublabel && (
                <span className="text-[11px] text-muted-foreground shrink-0 font-mono">
                  {item.sublabel}
                </span>
              )}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
