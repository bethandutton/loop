import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";

interface TerminalSessionProps {
  sessionId: string;
}

export function TerminalSession({ sessionId }: TerminalSessionProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 13,
      fontFamily: "'JetBrains Mono', monospace",
      theme: {
        background: getComputedStyle(document.documentElement)
          .getPropertyValue("--surface")
          .trim() || "#1a1a2e",
        foreground: getComputedStyle(document.documentElement)
          .getPropertyValue("--foreground")
          .trim() || "#e0e0e0",
        cursor: getComputedStyle(document.documentElement)
          .getPropertyValue("--primary")
          .trim() || "#6e8efb",
      },
      convertEol: true,
      scrollback: 10000,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(new WebLinksAddon());

    term.open(containerRef.current);
    fitAddon.fit();

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    // Load existing scrollback
    invoke<number[]>("get_scrollback", { sessionId }).then((data) => {
      if (data && data.length > 0) {
        const bytes = new Uint8Array(data);
        const text = new TextDecoder().decode(bytes);
        term.write(text);
      }
    }).catch((e) => console.error("Failed to load scrollback:", e));

    // Listen for live output
    const unlistenPromise = listen<number[]>(`session_output_${sessionId}`, (event) => {
      const bytes = new Uint8Array(event.payload);
      const text = new TextDecoder().decode(bytes);
      term.write(text);
    });

    // Send user input to PTY
    const disposable = term.onData((data) => {
      const encoded = new TextEncoder().encode(data);
      invoke("write_to_session", {
        sessionId,
        data: Array.from(encoded),
      }).catch((e) => console.error("Failed to write to session:", e));
    });

    // Handle resize
    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      disposable.dispose();
      resizeObserver.disconnect();
      unlistenPromise.then((f) => f());
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, [sessionId]);

  return (
    <div
      ref={containerRef}
      className="h-full w-full p-2"
      style={{ backgroundColor: "var(--surface)" }}
    />
  );
}
