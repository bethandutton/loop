import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Button } from "@/components/ui/button";
import { Sparkles, Save, Loader2, Pencil, Eye, Play } from "lucide-react";
import type { TicketCard } from "@/App";

interface PlanEditorProps {
  ticket: TicketCard;
  hideToolbar?: boolean;
}

export function PlanEditor({ ticket, hideToolbar }: PlanEditorProps) {
  const [content, setContent] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [enhancing, setEnhancing] = useState(false);
  const [editing, setEditing] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [conflict, setConflict] = useState(false);
  const lastRemoteContent = useRef<string>("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    setContent("");
    setDirty(false);
    setEditing(false);
    setLoading(true);
    setError(null);
    invoke<string | null>("get_ticket_description", { ticketId: ticket.id })
      .then((desc) => {
        const val = desc || "";
        setContent(val);
        lastRemoteContent.current = val;
        setDirty(false);
        setConflict(false);
      })
      .catch((e) => {
        console.error("Failed to load ticket description:", e);
        setError(String(e));
      })
      .finally(() => setLoading(false));
  }, [ticket.id]);

  useEffect(() => {
    if (editing && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [editing]);

  // Poll for remote changes every 30s to detect conflicts
  useEffect(() => {
    const interval = setInterval(() => {
      if (!dirty) return; // Only check when there are local edits
      invoke<string | null>("get_ticket_description", { ticketId: ticket.id })
        .then((desc) => {
          const remote = desc || "";
          if (remote !== lastRemoteContent.current) {
            setConflict(true);
          }
        })
        .catch(() => {});
    }, 30000);
    return () => clearInterval(interval);
  }, [ticket.id, dirty]);

  const handleReloadRemote = useCallback(() => {
    invoke<string | null>("get_ticket_description", { ticketId: ticket.id })
      .then((desc) => {
        const val = desc || "";
        setContent(val);
        lastRemoteContent.current = val;
        setDirty(false);
        setConflict(false);
      })
      .catch(() => {});
  }, [ticket.id]);

  const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setContent(e.target.value);
    setDirty(true);
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await invoke("save_plan_to_linear", { ticketId: ticket.id, content });
      setDirty(false);
    } catch (e) {
      console.error("Failed to save plan:", e);
    } finally {
      setSaving(false);
    }
  };

  const handleEnhance = async () => {
    setEnhancing(true);
    try {
      const enhanced = await invoke<string>("enhance_plan", {
        ticketId: ticket.id,
        title: ticket.title,
        currentPlan: content,
      });
      setContent(enhanced);
      setDirty(true);
    } catch (e) {
      console.error("Failed to enhance plan:", e);
    } finally {
      setEnhancing(false);
    }
  };

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      {!hideToolbar && (
        <div className="titlebar-drag-region flex h-14 shrink-0 items-end justify-between pb-2 px-4">
          <div className="titlebar-no-drag flex items-center gap-2 min-w-0">
            <span className="font-mono text-[11px] text-muted-foreground shrink-0">
              {ticket.identifier}
            </span>
            <span className="text-[13px] text-foreground truncate">
              {ticket.title}
            </span>
            {dirty && (
              <span className="text-[10px] text-warning shrink-0">unsaved</span>
            )}
          </div>
        <div className="titlebar-no-drag flex items-center gap-1.5">
          {(ticket.status === "backlog" || ticket.status === "todo") && (
            <Button
              variant="ghost"
              size="sm"
              onClick={async () => {
                try {
                  await invoke("update_ticket_status", { ticketId: ticket.id, status: "planning" });
                } catch (e) {
                  console.error("Failed to update status:", e);
                }
              }}
              title="Move to Planning"
            >
              <Play size={13} className="mr-1" />
              Plan
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setEditing(!editing)}
            title={editing ? "Preview" : "Edit"}
          >
            {editing ? <Eye size={13} className="mr-1" /> : <Pencil size={13} className="mr-1" />}
            {editing ? "Preview" : "Edit"}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleEnhance}
            disabled={enhancing}
            title="Enhance with Claude"
          >
            {enhancing ? (
              <Loader2 size={13} className="animate-spin mr-1" />
            ) : (
              <Sparkles size={13} className="mr-1" />
            )}
            Enhance
          </Button>
          <Button
            size="sm"
            onClick={handleSave}
            disabled={!dirty || saving}
            title="Save to Linear (⌘↩)"
          >
            {saving ? (
              <Loader2 size={13} className="animate-spin mr-1" />
            ) : (
              <Save size={13} className="mr-1" />
            )}
            Save
          </Button>
        </div>
      </div>
      )}

      {/* Action bar when toolbar is hidden */}
      {hideToolbar && (
        <div className="flex items-center justify-between px-4 py-1.5 shrink-0">
          <div className="flex items-center gap-2">
            {dirty && <span className="text-[10px] text-warning">unsaved</span>}
          </div>
          <div className="flex items-center gap-1.5">
            <Button variant="ghost" size="sm" onClick={() => setEditing(!editing)}>
              {editing ? <Eye size={13} className="mr-1" /> : <Pencil size={13} className="mr-1" />}
              {editing ? "Preview" : "Edit"}
            </Button>
            <Button variant="ghost" size="sm" onClick={handleEnhance} disabled={enhancing}>
              {enhancing ? <Loader2 size={13} className="animate-spin mr-1" /> : <Sparkles size={13} className="mr-1" />}
              Enhance
            </Button>
            <Button size="sm" onClick={handleSave} disabled={!dirty || saving}>
              {saving ? <Loader2 size={13} className="animate-spin mr-1" /> : <Save size={13} className="mr-1" />}
              Save
            </Button>
          </div>
        </div>
      )}

      {/* Conflict banner */}
      {conflict && (
        <div className="mx-4 rounded-md bg-warning/10 border border-warning/20 px-3 py-2 flex items-center justify-between">
          <p className="text-xs text-warning">Linear's version has changed since you started editing.</p>
          <button
            onClick={handleReloadRemote}
            className="text-xs text-warning font-medium hover:underline shrink-0 ml-3"
          >
            Reload
          </button>
        </div>
      )}

      {/* Error banner */}
      {error && (
        <div className="mx-4 rounded-md bg-destructive/10 border border-destructive/20 px-3 py-2">
          <p className="text-xs text-destructive">{error}</p>
        </div>
      )}

      {/* Content area */}
      <div className="flex-1 overflow-y-auto p-8">
        <div className="max-w-3xl mx-auto">
          {loading ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground py-8">
              <Loader2 size={14} className="animate-spin" />
              Loading plan...
            </div>
          ) : editing ? (
            <textarea
              ref={textareaRef}
              value={content}
              onChange={handleChange}
              className="w-full h-full min-h-[500px] resize-none bg-transparent text-[15px] leading-relaxed text-foreground placeholder:text-muted-foreground focus:outline-none"
              placeholder="Write your plan here (markdown supported)..."
              spellCheck={false}
            />
          ) : content ? (
            <div className="plan-markdown">
              <ReactMarkdown
                remarkPlugins={[remarkGfm]}
                components={{
                  img: ({ src, alt }) => (
                    <img
                      src={src}
                      alt={alt || ""}
                      className="max-w-full rounded-md border border-border my-3"
                      loading="lazy"
                    />
                  ),
                  h1: ({ children }) => (
                    <h1 className="text-xl font-semibold tracking-tight text-foreground mt-6 mb-3 first:mt-0">
                      {children}
                    </h1>
                  ),
                  h2: ({ children }) => (
                    <h2 className="text-base font-semibold tracking-tight text-foreground mt-5 mb-2">
                      {children}
                    </h2>
                  ),
                  h3: ({ children }) => (
                    <h3 className="text-sm font-semibold text-foreground mt-4 mb-1.5">
                      {children}
                    </h3>
                  ),
                  p: ({ children }) => (
                    <p className="text-[15px] leading-relaxed text-foreground mb-3">
                      {children}
                    </p>
                  ),
                  ul: ({ children }) => (
                    <ul className="list-disc pl-5 mb-3 space-y-1 text-[15px] text-foreground">
                      {children}
                    </ul>
                  ),
                  ol: ({ children }) => (
                    <ol className="list-decimal pl-5 mb-3 space-y-1 text-[15px] text-foreground">
                      {children}
                    </ol>
                  ),
                  li: ({ children }) => (
                    <li className="leading-relaxed">{children}</li>
                  ),
                  code: ({ children, className }) => {
                    const isBlock = className?.includes("language-");
                    if (isBlock) {
                      return (
                        <pre className="bg-surface rounded-md border border-border p-3 my-3 overflow-x-auto">
                          <code className="font-mono text-[13px] text-foreground">
                            {children}
                          </code>
                        </pre>
                      );
                    }
                    return (
                      <code className="font-mono text-[13px] bg-surface px-1 py-0.5 rounded border border-border">
                        {children}
                      </code>
                    );
                  },
                  pre: ({ children }) => <>{children}</>,
                  a: ({ href, children }) => (
                    <a
                      href={href}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-primary underline underline-offset-2 hover:opacity-80"
                    >
                      {children}
                    </a>
                  ),
                  blockquote: ({ children }) => (
                    <blockquote className="border-l-2 border-primary pl-4 my-3 text-muted-foreground italic">
                      {children}
                    </blockquote>
                  ),
                  hr: () => <hr className="border-border my-6" />,
                  table: ({ children }) => (
                    <div className="overflow-x-auto my-3">
                      <table className="w-full text-[13px] border-collapse border border-border">
                        {children}
                      </table>
                    </div>
                  ),
                  th: ({ children }) => (
                    <th className="border border-border bg-surface px-3 py-1.5 text-left font-medium text-muted-foreground">
                      {children}
                    </th>
                  ),
                  td: ({ children }) => (
                    <td className="border border-border px-3 py-1.5 text-foreground">
                      {children}
                    </td>
                  ),
                  input: ({ checked, ...props }) => (
                    <input
                      type="checkbox"
                      checked={checked}
                      readOnly
                      className="mr-1.5 h-3.5 w-3.5 rounded border-border"
                      {...props}
                    />
                  ),
                }}
              />
            </div>
          ) : (
            <p
              className="text-sm text-muted-foreground cursor-pointer hover:text-foreground transition-colors"
              onClick={() => setEditing(true)}
            >
              No plan yet. Click to start writing...
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
