<img src="app-icon.png" width="80" alt="Herd app icon" />

# Herd

A macOS desktop app for managing many in-flight Linear tickets across Git worktrees, each with its own background Claude Code agent session.

## What it does

- **Ticket board** — all your assigned Linear tickets in one place, with search, filter, sort, and kanban view
- **Per-ticket worktrees** — each ticket gets its own Git branch and worktree, isolated from the rest
- **Claude Code agents** — background terminal sessions per ticket, running in their worktree
- **Plan editor** — edit ticket descriptions with a rich markdown editor, enhance with Claude, save back to Linear
- **GitHub PR tracking** — auto-detects PRs, shows status, transitions tickets through review stages
- **Local preview** — run services and preview your app without leaving Herd
- **Keyboard-first** — Cmd+K palette, j/k navigation, tab switching

## Install

Download the latest `.dmg` from [Releases](../../releases), or build from source:

```bash
# Prerequisites: macOS, Rust, Node.js 20+
git clone https://github.com/bethandutton/herd.git
cd herd
npm install
npm run tauri dev
```

## Setup

On first launch, Herd walks you through connecting:

1. **Linear** — paste your API token ([create one here](https://linear.app/settings/account/security))
2. **GitHub** — paste a personal access token with `repo` scope
3. **Repository** — pick your local clone folder

Tokens are stored securely. Herd never sends data anywhere except Linear, GitHub, and the Anthropic API — all with your own keys.

## Privacy

**No telemetry. Ever.** No crash reporting, no analytics, no tracking.

## License

MIT — see [LICENSE](LICENSE).
