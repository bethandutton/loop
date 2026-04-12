# Loop — Build Plan

The order to build Loop in. Each phase should be independently usable. Do not try to build everything at once. The user has explicitly asked for the full thing, but the value of the tool comes from the loop closing — get phase 1 working end-to-end before moving on.

After each phase, the app should be installable, runnable, and useful for at least *some* part of the user's workflow.

---

## Phase 0 — Project skeleton

**Goal:** a Tauri app that opens, runs the first-run onboarding flow if needed, shows the three-column layout with placeholder content, has theming working end to end, and persists settings.

Deliverables:
- Tauri project initialized with React + TypeScript + Tailwind + shadcn/ui
- React Aria added as a dependency for use in specific primitives (Cmd+K palette)
- Three-column layout using `Resizable` panels, with the proportions and constraints from `02-ui-and-design.md`
- CSS variable theming set up: dark (default), light, system modes work, switching is live
- Density and font-size controls work and persist
- SQLite database created and migrated on first run with the **full schema** from `01-product-spec.md` (including the v2 tables and the `Repo` table — leave them empty)
- **First-run onboarding flow**: if there are no `Repo` rows in the database when the app starts, show the guided setup (welcome, Linear token, GitHub token, repo path + name + primary branch + preview port). On completion, write the `Repo` row and dismiss onboarding.
- Settings panel reachable via `Cmd+,`: shows all the same fields plus theme, density, font size, and a "Re-run setup" button
- Tokens stored in macOS Keychain (never in SQLite)
- Command/event layer scaffolded: at least one Tauri command and one event, wired through React, even if it does nothing yet
- Repo root files created: `README.md` (placeholder is fine), `LICENSE` (MIT), `CONTRIBUTING.md` (placeholder), `CHANGELOG.md` (with a "v0.0.1 - skeleton" entry)

**Definition of done:** a fresh user can download a built binary, launch it, walk through the onboarding, configure their tokens, switch themes, change density and font size, and the app remembers everything across restarts. The repo on GitHub has a license, a basic README, and the spec docs in a `docs/` folder.

---

## Phase 1 — The board, read-only

**Goal:** the board shows real Linear tickets in the right columns. The user can see their work without doing anything.

Deliverables:
- Linear API client (Rust side) with read-only methods: list tickets assigned to current user, get ticket details, list cycles, list labels
- Polling worker that runs every 30s, fetches tickets, writes them to the SQLite `Ticket` table, emits a `tickets_updated` event
- React board renders all nine columns from the spec, populated from SQLite via Tauri commands
- Tickets in Backlog and To do columns are real (from Linear). The other columns are empty for now (no branches exist yet).
- Ticket cards have the visual design from `02-ui-and-design.md`: ID, priority, title, badges
- Clicking a card sets the active ticket (stored in app state, no middle/right column behavior yet)
- The "+ New ticket" button works: opens a small form, creates a Linear ticket via the API, refreshes

**Definition of done:** the user can launch Loop, see all their assigned Linear tickets, click between them, and create new tickets without leaving the app.

---

## Phase 2 — The plan editor

**Goal:** the middle column works in Plan mode for tickets in Backlog, To do, and Planning.

Deliverables:
- Markdown editor in the middle column when the active ticket has a Backlog/To do/Planning status
- Editor pulls the ticket description from Linear as its initial content, caches in `plan_markdown`
- "Save to Linear" button writes the editor contents back to the Linear ticket description
- "Enhance with Claude" button calls the Anthropic API with the current plan + ticket title + relevant codebase context (start simple: just the ticket title + description, no codebase grep yet — that can be a phase 2.1 follow-up), replaces the editor contents with the result, marks `plan_dirty` true
- Conflict detection: if Linear's description changes while the user has unsaved local edits, show the warning banner described in `01-product-spec.md`
- A "Move to Planning" button on tickets in Backlog/To do that just updates the local status (does not yet create a branch)

**Definition of done:** the user can read, edit, AI-enhance, and save plans for any of their Linear tickets, with Linear staying as the source of truth.

---

## Phase 3 — Per-ticket worktrees and background Claude sessions

**This is the load-bearing phase. If this works, the tool is real. If it doesn't, nothing else matters.**

**Goal:** the middle column works in Session mode. Each active ticket has its own background Claude Code session in its own worktree. Sessions persist across ticket switches.

Deliverables:
- "Start ticket" button on tickets in Planning. Clicking it:
  1. Runs `git fetch origin main`
  2. Resolves the branch name from Linear (the "git branch name" field if present, else `<TICKET-ID>-<slug>`)
  3. Checks if the branch already exists locally or remotely; if yes, offers to use it
  4. Runs `git worktree add <path> -b <branch-name> origin/main`
  5. Copies allowlisted gitignored files (`.env*` etc) from the local worktree
  6. Spawns a Claude Code PTY in the new worktree directory, idle (no initial prompt)
  7. Updates the ticket: status → In progress, `branch_name`, `worktree_path`, `claude_session_id` set
- xterm.js terminal in the middle column when a ticket has an active session
- Scrollback buffered to disk per session (`scrollbacks/<session_id>.log`), restored when the user clicks back into a ticket
- Switching active ticket swaps the visible terminal instantly. Background sessions keep running and producing output.
- Unread output dot on a ticket card whose session has produced output since the user last viewed it
- "Kill session" button in the middle column toolbar
- The `/handoff` slash command is taught to Claude Code via its initial system prompt; Loop watches the PTY output for the marker, captures the summary, terminates the PTY, moves the ticket to "Ready to test", fires a Mac notification

**Definition of done:** the user can have many tickets in In progress / Ready to test simultaneously. Each one has its own running Claude Code session in its own worktree. Switching between them is one click and instant. Sessions never lose state.

**Notes for Claude Code on this phase:**
- Use `portable-pty` or similar Rust crate for PTY management. node-pty is also fine if going through Node, but Rust is cleaner here.
- Each session is one child process. On app quit, terminate all children gracefully.
- The scrollback buffer is the single source of truth for what the user sees. Do not try to keep React in sync with PTY output line-by-line — buffer in Rust, expose a "give me the last N lines" command, let xterm.js render.
- Branch creation can fail in a thousand interesting ways. Surface git's stderr to the user verbatim in a modal. Do not try to recover automatically.

---

## Phase 4 — The local column

**Goal:** the right column works. The user can run services for the active ticket and see a browser preview.

Deliverables:
- Single shared "_local" worktree exists in the worktrees parent folder (created on first run if missing)
- When the active ticket changes, the right column prompts: "Switch local to ticket X?" — on confirm, stops any running services and runs `git checkout <branch>` in the local worktree
- Service runner panel auto-detects scripts from `package.json` (start with this; other formats can come later) and renders them as checkboxes
- Ticking a service and clicking Run spawns it in a hidden PTY, with output buffered to a per-service scrollback
- Service status indicators (running, stopped, errored) update live
- Clicking a service name expands a drawer showing its terminal output
- Browser preview at the bottom of the right column: a Tauri webview pointed at `localhost:<port>`, where port comes from settings (default 3000)
- "Hide right column" toggle in the footer

**Definition of done:** the user can pick a ticket, switch the local to it, run its dev services, and see the running app in the embedded browser without leaving Loop.

---

## Phase 5 — GitHub polling and the review columns

**Goal:** the review side of the workflow works. PRs are detected, comments trigger Attention required, notifications fire.

Deliverables:
- GitHub API client (Rust side) with read-only methods: get PR by branch, list PR reviews, list PR issue comments, get PR state
- Polling worker that runs every 60s for each known PR (i.e. each ticket with a `pr_number`)
- PR detection: when a session pushes a branch and a PR exists, store `pr_number` and `pr_url` on the ticket
- Status transition logic:
  - First time a draft PR is detected for a ticket → status to In review
  - New comment from anyone other than the user → status to Attention required + Mac notification
  - PR approved + no unresolved comments → status to Ready to merge
  - PR merged → status to Done, start the 48-hour cleanup countdown
- Mac notifications fire on the relevant transitions, with click-to-open-ticket
- PR overlay toggle in the middle column toolbar: when on, shows a webview pointed at the PR URL beside the terminal
- Done cleanup worker: 48 hours after a ticket lands in Done, remove the worktree, terminate any lingering session, archive scrollback, remove the card
- Stale worktree sweep on app startup

**Definition of done:** the user can complete a full real-world cycle in Loop: pick up a ticket, work on it, push a draft PR, get pinged when CodeRabbit comments arrive, address them, get pinged when humans review, merge, watch the ticket disappear.

---

## Phase 6 — Polish and remaining v1 features

**Goal:** the app feels finished as a v1.

Deliverables:
- Branch-collision detection (the soft warning before branch creation, described in `01-product-spec.md`)
- Codebase context for the "Enhance with Claude" button (grep relevant files by ticket title/description, include their contents in the prompt)
- `Cmd+K` ticket switcher (modal with fuzzy search)
- All keyboard shortcuts from `02-ui-and-design.md`
- Rate-limit indicator in the footer (if available from Anthropic's API headers)
- Settings: "mirror status to Linear" toggle (off by default), "files to copy into new worktrees" allowlist
- Empty states, loading skeletons, error toasts everywhere they should be
- A first-run experience: if no tokens are configured, the Settings panel opens automatically with a brief explanation

**Definition of done:** the app is something the user could give to a friend and not feel embarrassed about.

---

## Phase 7 — Release and distribution

**Goal:** Loop is downloadable, installable, and runnable by anyone with a Mac. Not just runnable from source.

Deliverables:
- Tauri build configuration finalized for both Apple Silicon and Intel
- Code signing with a Developer ID Application certificate
- Notarization via Apple's notary service
- `.dmg` packaging with a proper installer background
- GitHub Actions workflow that builds, signs, notarizes, and uploads to GitHub Releases on tag push
- README updated with screenshots, install instructions, quickstart, link to docs, badge for latest release
- CONTRIBUTING.md filled out with dev setup, build-from-source instructions, and the contribution philosophy from `01-product-spec.md`
- CHANGELOG.md updated with the v1.0.0 release notes
- A GitHub Release for v1.0.0 with the signed `.dmg` attached and release notes

See `04-release-and-distribution.md` for the full distribution process, the gotchas (notarization can fail in interesting ways), and the GitHub Actions workflow shape.

**Definition of done:** a stranger on the internet can find Loop on GitHub, click "Download", install the `.dmg` without macOS Gatekeeper warnings, launch it, complete onboarding, and start using it within five minutes.

---



See the "Out of scope for v1" section in `01-product-spec.md`. The short version:

- No code editor (Monaco for code, file tree, diffs against main)
- No drag-and-drop on the board
- No multi-provider agents
- No webhook integrations
- No PR writes from inside Loop
- No multi-repo
- No tags rendering (data is stored, UI comes in v2)
- No auto-actions (data model exists, UI and execution come in v2)
- No mobile companion app (architecture supports it, but the server layer and the iOS app are v2)

Resist the temptation to sneak any of these into v1, even small versions. The whole point of the build order above is that each phase delivers value. Sneaking in v2 features during v1 phases delays the value of the v1 features.

---

## When to ask the user

Stop and ask, do not guess, when you hit any of these:

- **Team-specific git rules** the spec doesn't cover: CI requirements, commit message format, rebase vs merge, draft PR labels, protected branches
- **Linear API quirks**: the "git branch name" field's exact name and structure has changed over the API's lifetime. Verify before using.
- **CodeRabbit's actual comment format and how to detect "CodeRabbit is done"**: this varies by repo configuration. Ask the user to point you at a real PR with CodeRabbit comments to inspect the structure before writing detection logic.
- **The user's primary service port** for the browser preview: 3000 is the default but check.
- **`.env` file naming and location**: `.env*` is the assumption but the user may have files in unusual places that need copying.

Asking is faster than getting it wrong and rebuilding.
