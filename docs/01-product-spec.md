# Loop — Product Spec

A macOS desktop app for managing many in-flight Linear tickets across many Git branches in a single repo, without losing context every time a review cycle interrupts you.

Built with **Tauri** (Rust backend, React frontend).

---

## The problem this solves

The user works in one repo. Tickets are small. The review cycle (CodeRabbit + human reviewers) takes longer than the work itself, so by the time a review comes back, the user has moved on to another ticket and is deep in unrelated context. Switching back to handle the review means: stash, checkout, re-read the PR, answer comments, push, switch back, lose flow. Multiply by four concurrent tickets and the day is mostly context-switching.

Loop fixes this by giving every active ticket its own persistent Git worktree and its own background Claude Code session, surfaced through a single board where the user can jump between tickets in one click and pick up exactly where each one left off.

---

## The mental model

**One ticket = one branch = one worktree = one Claude Code session.**

These four things are born together when a ticket enters active work, live together for the ticket's lifetime, and are torn down together when the ticket is done. The user never thinks about them separately. They click a ticket card, and the whole environment for that ticket rehydrates around them.

---

## The three columns

### Left column — the Board

Vertical Kanban down the left side. Tickets flow through these columns in order:

1. **Backlog** — Linear tickets assigned to the user, not yet in a cycle
2. **To do** — Linear tickets in the current cycle, ordered by priority
3. **Planning** — user has started thinking about how to do this ticket; no branch yet
4. **In progress** — branch and worktree exist, Claude session is actively working
5. **Ready to test** — Claude has run `/handoff`; the work is "done" pending the user manually testing the local
6. **In review** — draft PR is open, waiting on CodeRabbit and/or human reviewers
7. **Attention required** — CodeRabbit or a human has left new comments; ball is in the user's court
8. **Ready to merge** — PR is approved, no outstanding comments, just needs the merge button
9. **Done** — merged. Stays visible for 48 hours then disappears.

Each card shows: ticket ID, title, priority badge, and state-specific badges (e.g. "CodeRabbit running" inside In review, "3 new comments" inside Attention required, unread-output dot if a background Claude session has produced output since last viewed).

Clicking a card makes that ticket the **active ticket** and rehydrates the middle and right columns around it.

There is a **"+ New ticket"** button at the top of the board. It opens a small form (title, description, priority) and creates a real Linear ticket assigned to the user, dropped into To do. No local-only tickets. Linear is the source of truth for what work exists.

### Middle column — Plan or Session

The middle column has exactly two modes, determined by the active ticket's status:

**Plan mode** — when the active ticket is in Backlog, To do, or Planning.

A markdown editor showing the ticket's plan (which is the Linear ticket description). Standard editing, standard markdown rendering. Two buttons in the toolbar:

- **Enhance with Claude** — fires a one-shot Claude API call. The prompt: take the current plan text, the ticket title, and codebase context (relevant files grep'd by ticket title and description keywords), and produce a better version of the plan. The result *replaces* the editor contents but does **not** push to Linear yet. The user can edit the result, hit undo, or hit save.
- **Save to Linear** — pushes the editor contents to the Linear ticket description via the Linear API.

If Linear's version of the plan changes while the user has unsaved edits, show a non-blocking warning at the top of the editor: "Linear's version has changed. Reload to see the new version (your edits will be lost)."

**Session mode** — when the active ticket is in In progress, Ready to test, In review, Attention required, or Ready to merge.

A real `xterm.js` terminal connected to the Claude Code PTY for that ticket's worktree. The user types into it like any terminal. Scrollback persists across ticket switches (each session has its own buffer in memory, swapped in when the ticket becomes active).

Above the terminal there is a thin toolbar with:
- Ticket ID and title
- A toggle to show/hide a **PR overlay** (described below)
- A "Kill session" button (for when an agent is stuck or off the rails)

**PR overlay** — when the active ticket has an open PR, the user can toggle a side panel that shows the PR's GitHub page (review comments, CodeRabbit threads, CI status). This is a webview pointing at the PR URL. It's optional, off by default, and the user's preference is remembered per ticket status.

### Right column — the Local

A **single, persistent local environment** that follows the active ticket. Only one ticket's branch can be checked out here at a time, because there is only one set of ports on the machine.

The right column contains:

1. **Branch context bar** at the top, showing which branch is currently checked out and the active ticket it belongs to.
2. **Service runner panel** — auto-detects scripts from the worktree's `package.json` (and any other recognized service definition files) and presents them as checkboxes. The user ticks the services they want, hits **Run**, and each ticked service spawns in a hidden background terminal. A small status indicator per service (running, stopped, errored) sits next to its checkbox. Clicking a service name reveals its terminal output in a drawer.
3. **Browser preview** — a webview pointed at `localhost:<port>` for the primary service. The port is configurable per project. This is the main visual surface for testing.

When the user clicks a different ticket on the board, the right column shows a confirmation: "Switch local to ticket X? This will stop currently running services on ticket Y." On confirm: stop services, `git checkout` the new branch in the right column's worktree, the user re-runs whichever services they need.

(Important: the right column's "worktree" is a *single fixed worktree*, not per-ticket. Branch switching happens inside it. This is different from the per-ticket worktrees that hold the background Claude sessions — see "Worktrees" below.)

---

## Worktrees — the central architectural piece

This is the part that has to be exactly right or nothing else works.

### Two kinds of worktrees

Loop maintains **two distinct kinds of worktrees** in `~/code/<repo>-worktrees/`:

1. **Per-ticket worktrees** — one folder per active ticket. Named after the branch (which is named after the Linear ticket ID + title). Each one has its own checked-out branch, its own `node_modules`, and is the working directory for that ticket's background Claude Code session. The session's PTY is spawned with this folder as cwd.

2. **The local worktree** — a single folder used exclusively by the right column. The right column checks out different branches into this one worktree as the user switches active ticket. This is the only place services actually run, because services need ports and only one branch can own them at a time.

The reason for the split: background Claude sessions don't need ports. They're talking to the API, editing files, running tests in their own isolated worktrees. They can all run in parallel without conflict because they don't touch localhost. The local worktree is where localhost lives, and it's a singleton.

### Branch creation rules

When a ticket transitions out of Planning (i.e. when the user starts work), Loop creates the branch and worktree. The rules:

1. **Always branch from `origin/main`, never from local `main`.** Run `git fetch origin main` first, then `git worktree add <path> -b <branch-name> origin/main`.
2. **Branch name** comes from Linear: the ticket ID and a slugified version of the title. Loop reads the convention from Linear's "git branch name" field if present, otherwise constructs it as `<TICKET-ID>-<slugified-title>`.
3. **Worktree path:** `~/code/<repo>-worktrees/<branch-name>`.
4. **Before creating:** check whether this branch already exists locally or remotely. If yes, offer to use the existing one instead of creating a duplicate.
5. **After creating:** copy `.env*` files (and any other gitignored files matching a configurable allowlist) from the local worktree into the new one, so the dev environment works out of the box.
6. **Spawn the Claude Code session** in the new worktree as soon as it's ready. The session starts paused with no prompt; the user types the first instruction when they click into the ticket.

### Branch-collision detection (soft warning)

Before creating a new branch, Loop runs a check: for each other active worktree, get the list of files that branch has changed against `origin/main` (`git diff --name-only origin/main...<branch>`). Also fetch open PRs from GitHub and their changed files. Then make a single Claude API call with the new ticket's plan and the list of files touched by other active branches and open PRs, asking: "Does this ticket likely overlap with any of these?"

If the answer is yes, show a non-blocking warning before branch creation: "Heads up — branch X (and/or PR Y by Z) is touching files that look related to this ticket. Continue anyway?"

This is a warning, never a block.

### Worktree cleanup

When a PR is merged (detected by polling GitHub), the ticket moves to Done and stays visible for 48 hours. After 48 hours:
- The Done card disappears from the board
- The worktree is removed (`git worktree remove`)
- The Claude session for that ticket is terminated and its scrollback is archived to disk

If the user manually closes a ticket from the board, same cleanup happens immediately.

There is also a "stale worktree" sweep on app startup: any worktree whose branch has been merged and deleted on the remote gets cleaned up automatically.

---

## Claude Code sessions

### Lifecycle

1. **Created** when a ticket leaves Planning and a worktree is built. The PTY is spawned but the session is idle until the user gives it a prompt.
2. **Active** as the user interacts with it through the middle column.
3. **Background** when the user clicks away to another ticket. The PTY keeps running, output buffers in memory, the session continues whatever it was doing.
4. **Completed** when the agent runs `/handoff` (a slash command Loop teaches Claude Code about via its system prompt). The PTY process is terminated cleanly, scrollback is saved to disk, the ticket moves to Ready to test.
5. **Killed** when the user hits the "Kill session" button (for stuck or broken sessions). Same cleanup as completed but no status transition — the user can restart the session manually.

### The handoff signal

Loop teaches Claude Code about a `/handoff` slash command via its initial system prompt. The instruction to the agent: "When you genuinely believe the work is complete and you have tested it as well as you can, run `/handoff` with a short summary of what's done, what's still TODO, and what the user should manually verify."

Loop watches the session's output for the handoff marker. When it sees one, it:
- Captures the summary into the ticket's metadata
- Terminates the PTY
- Moves the ticket card to Ready to test
- Fires a Mac notification: "Ticket X is ready for you to test"

### Running many sessions

There is no artificial cap on concurrent sessions. The user can have ten active tickets and ten background Claude sessions. The constraint is the Anthropic API rate limit (or the user's Claude Pro/Max subscription), which Loop does not try to hide. A small usage indicator in the app footer shows current rate limit headroom if available.

### Killing stuck sessions

Every ticket card in In progress has a "Kill" affordance. Killing a session terminates the PTY but does not delete the worktree or the branch. The user can restart the session from scratch.

---

## Linear integration

### Reads

Loop polls the Linear API every 30 seconds for:
- Tickets assigned to the user (drives Backlog and To do columns)
- Ticket plan (description) updates (used to detect mid-edit conflicts in the plan editor)
- Labels on tickets (stored in the `tags` field for v2 rendering)

### Writes

Loop writes to Linear when:
- The user hits **Save to Linear** in the plan editor (updates ticket description)
- The user creates a new ticket via the **+ New ticket** button
- A ticket transitions to certain statuses, Loop optionally updates the Linear status to match (configurable per status; off by default to avoid surprising teammates)

### Auth

Linear API token, stored in macOS Keychain. Set up via a Settings panel on first run.

---

## GitHub integration

### Reads (polling)

Loop polls GitHub every 60 seconds for each open PR it knows about:
- PR state (draft, open, merged, closed)
- New review comments (CodeRabbit and humans)
- New issue comments on the PR
- CI status

### What the polling drives

- **In review → Attention required**: when a new comment or review appears on a PR that wasn't there last poll, and the comment is not from the user themselves
- **In review / Attention required → Ready to merge**: when the PR is approved and has no outstanding unresolved comments
- **Ready to merge → Done**: when the PR is merged
- **Mac notification** fires for any new comment that flips a card into Attention required

### Writes

Loop does not write to GitHub in v1. The user merges PRs manually through the GitHub UI (or via the PR overlay in the middle column).

### Auth

GitHub personal access token with `repo` scope, stored in macOS Keychain.

---

## Notifications

All notifications are native macOS notifications. No sounds by default, just the visual ping and a dock badge count.

Triggers:
- New CodeRabbit comment on a user's PR → "Ticket X has new CodeRabbit comments"
- New human review/comment on a user's PR → "Ticket X has new review comments from <author>"
- Claude session runs `/handoff` → "Ticket X is ready for you to test"
- Claude session errors out or is killed unexpectedly → "Ticket X session ended unexpectedly"

Clicking a notification opens Loop and makes the relevant ticket the active ticket.

---

## State management — command/event layer

All user actions and all state changes flow through a single command/event layer on the Tauri (Rust) side. React components dispatch commands and subscribe to events. They do not mutate state directly.

This is good practice in itself but it is also load-bearing for the v2 mobile companion app: the mobile client will be a thin adapter over this same layer, so anything the desktop UI can do, the mobile client can do too without a rewrite.

Concrete shape:
- Every user action is a Tauri command (e.g. `start_ticket`, `save_plan`, `kill_session`, `switch_local_to_ticket`)
- Every state change emits a Tauri event (e.g. `ticket_status_changed`, `session_output_appended`, `pr_state_updated`)
- React components are subscribers and dispatchers, never owners of canonical state
- Canonical state lives in the Rust side, backed by SQLite

---

## Data model

A local SQLite database in `~/Library/Application Support/Loop/loop.db`.

```
Repo {
  id            TEXT PRIMARY KEY    // generated UUID
  name          TEXT                // friendly name shown in UI
  path          TEXT                // absolute path to the user's clone
  worktrees_dir TEXT                // where this repo's worktrees live
  primary_branch TEXT               // usually "main"
  preview_port  INTEGER             // default browser preview port
  is_active     BOOLEAN             // v1: only one repo, this is always the active one
  created_at    TIMESTAMP
}

Ticket {
  id                    TEXT PRIMARY KEY  // Linear ticket ID
  repo_id               TEXT REFERENCES Repo(id)  // v1 always points to the single active repo
  title                 TEXT
  plan_markdown         TEXT              // local cache of the Linear description
  plan_dirty            BOOLEAN           // unsaved local edits
  status                TEXT              // backlog, todo, planning, in_progress, ready_to_test, in_review, attention_required, ready_to_merge, done
  priority              INTEGER
  cycle_id              TEXT
  branch_name           TEXT NULL
  worktree_path         TEXT NULL
  claude_session_id     TEXT NULL
  pr_number             INTEGER NULL
  pr_state              TEXT NULL
  pr_url                TEXT NULL
  last_seen_pr_event_id TEXT NULL         // for diffing on next poll
  handoff_summary       TEXT NULL
  tags                  TEXT              // JSON array of label names, synced from Linear (v2 renders these)
  created_at            TIMESTAMP
  updated_at            TIMESTAMP
  done_at               TIMESTAMP NULL
}

ClaudeSession {
  id              TEXT PRIMARY KEY
  ticket_id       TEXT REFERENCES Ticket(id)
  repo_id         TEXT REFERENCES Repo(id)
  pty_pid         INTEGER NULL              // null if not running
  scrollback_path TEXT                      // file on disk where output is buffered
  state           TEXT                      // idle, running, completed, killed
  started_at      TIMESTAMP
  ended_at        TIMESTAMP NULL
}

ServiceRun {
  id            TEXT PRIMARY KEY
  repo_id       TEXT REFERENCES Repo(id)
  worktree_path TEXT
  script_name   TEXT                        // e.g. "dev", "api"
  pty_pid       INTEGER NULL
  state         TEXT                        // running, stopped, errored
  started_at    TIMESTAMP
}

// v2 tables — present in v1 schema, unused in v1
AutoAction {
  id              TEXT PRIMARY KEY
  ticket_id       TEXT NULL REFERENCES Ticket(id)  // null = applies to all tickets (default rule)
  trigger_status  TEXT                              // status name that fires this rule
  prompt          TEXT                              // templated prompt sent to the Claude session
  enabled         BOOLEAN
  created_at      TIMESTAMP
}

AutoActionRun {
  id             TEXT PRIMARY KEY
  auto_action_id TEXT REFERENCES AutoAction(id)
  ticket_id      TEXT REFERENCES Ticket(id)
  fired_at       TIMESTAMP
  outcome        TEXT                            // sent, skipped, errored
  notes          TEXT NULL
}

Settings {
  key   TEXT PRIMARY KEY
  value TEXT
}
```

---

## File and folder layout

```
~/code/
  <repo>/                          # the user's normal clone, untouched by Loop
  <repo>-worktrees/
    LOOP-101-fix-auth-bug/         # per-ticket worktree (background Claude session)
    LOOP-104-add-feature-x/        # another per-ticket worktree
    _local/                        # the singleton local worktree (right column)

~/Library/Application Support/Loop/
  loop.db
  scrollbacks/
    <claude_session_id>.log
  archived_scrollbacks/
    <ticket_id>-<timestamp>.log
```

---

## Settings (minimum)

A small Settings panel:

- Linear API token
- GitHub API token
- Repo path (the user's main clone)
- Worktrees parent folder (default `~/code/<repo>-worktrees/`)
- Local browser preview default port
- Files to copy from local worktree into new worktrees (default `.env*`)
- Whether to mirror status changes back to Linear (default off)
- Theme (light, dark, system) — see `02-ui-and-design.md`
- Density (compact, comfortable, spacious) — see `02-ui-and-design.md`
- Font size (small, medium, large) — see `02-ui-and-design.md`

---

## Open source considerations

Loop is an open-source project, MIT-licensed, distributed via GitHub Releases. Anyone can download it, run it, fork it, and contribute. This shapes several decisions throughout the spec:

### No assumptions about the user

The user is not "Bethan with this specific repo at this specific path." The user is anyone who downloads Loop. Everything that varies between users must be configurable through the UI, never hardcoded:

- Repo path (where the user's clone lives)
- Worktrees parent folder
- Primary branch name (default `main` but can be `master`, `develop`, etc.)
- Browser preview port (default 3000 but configurable per repo)
- Files to copy into new worktrees (default `.env*`)
- Linear and GitHub tokens (always set by the user, never shipped)

The `Repo` table holds all repo-specific settings. v1 only ever has one row in it, but the structure exists so v2 multi-repo doesn't require a migration.

### First-run experience

When Loop launches for the first time (no `Repo` rows in the database), it shows a guided onboarding flow rather than the empty three-column layout. The flow:

1. **Welcome panel** explaining what Loop does in two sentences and what the user is about to set up
2. **Linear connection**: paste API token, verify by fetching the user's name, store in keychain
3. **GitHub connection**: paste personal access token (with required scopes listed: `repo`), verify, store in keychain
4. **Repo setup**: pick the repo folder via a native folder picker, give it a friendly name, confirm primary branch name (auto-detected from `git symbolic-ref refs/remotes/origin/HEAD`), set browser preview port (default 3000)
5. **Done**: dismiss the onboarding, show the main app

The onboarding is also reachable from Settings later (as "Re-run setup"), useful if the user wants to switch repos or rotate tokens. Each step can be skipped by the user except Linear (the app does nothing without it).

### Privacy and telemetry

**Loop has no telemetry. Ever.** No crash reporting, no usage analytics, no network calls except to:

- The Linear API (with the user's own token)
- The GitHub API (with the user's own token)
- The Anthropic API (via Claude Code, with the user's own subscription or key)

This is a hard commitment, documented in the README and reinforced in the privacy section of the project's GitHub repo. If a contributor proposes adding telemetry, the answer is no, regardless of how anonymized or opt-in it is. The trust win of "this tool never phones home" is worth more than any usage data.

### Secrets handling

All tokens (Linear, GitHub, optionally Anthropic API key) are stored in **macOS Keychain**, never in plain text on disk, never in environment variables that could leak to child processes. The SQLite database holds non-secret settings only.

### Documentation as part of the build

The repo must include, at the time of v1 release:

- `README.md` at the repo root: what Loop is, screenshots, install instructions, quickstart, link to the spec docs
- `LICENSE` (MIT)
- `CONTRIBUTING.md`: how to set up a dev environment, how to run from source, how to submit a PR, the project's stance on scope (i.e. "this is a personal-tool-shaped project, not all features will be accepted")
- `CHANGELOG.md`: at least one entry for the v1 release
- The four spec docs (this one, the UI doc, the build plan, the release doc) in a `docs/` folder

Documentation is not optional and is not a follow-up. It ships with v1.

### Contribution philosophy

Loop is intentionally narrow. It does one thing (manage many in-flight tickets across worktrees) for one kind of user (a solo developer with a slow review cycle). Contributions that broaden it past that — multi-user features, team collaboration, web access, mobile-first redesigns, generic Kanban features — should be politely declined or pointed to a fork. Contributions that improve the core loop, fix bugs, improve accessibility, support more git providers, or add genuine quality improvements are welcomed.

The CONTRIBUTING.md should say this directly so contributors don't waste time on unwanted PRs.

### Distribution

See `04-release-and-distribution.md` for the macOS-specific build, sign, notarize, and release process. The short version: GitHub Releases, code-signed with a Developer ID certificate, notarized via Apple's notary service, distributed as a `.dmg` file. No App Store.

---



These have come up in conversation but are deliberately not in v1. They are listed here so the v1 architecture leaves room for them rather than blocking them.

### Hard out of scope (not planned)

- Multi-provider agents (OpenAI Codex, Gemini CLI). One agent type: Claude Code.
- Built-in code editor (Monaco for code, file tree, diffs against main). The middle column is the Claude session — if the user wants to see code, they ask the agent.
- Drag-and-drop between board columns. Status changes happen via signals (handoff, PR events) and user actions (start ticket, save plan), not by dragging cards.
- Auto-stopping the previous local's services when switching tickets without confirmation. v1 always confirms.
- Multi-repo support. v1 is one repo at a time.
- Webhook-based integrations (Linear and GitHub are both polled). Webhooks are nicer but require a public endpoint.
- Writing PR comments or merging PRs from inside Loop. v1 reads only.

### Planned for v2 (build v1 to leave room for these)

**Tags on tickets.** Linear's labels, synced bidirectionally, displayed as colored chips on each card. Tags describe *kind of work* (design, draft, spike, blocked) while columns describe *workflow state*. A ticket can be "In review" and tagged "design" simultaneously. Tags do not drive automation in v2 — they're for filtering and at-a-glance recognition. Filter affordance on the board: show/hide by tag. Tag management lives in Linear, Loop only reads and renders.

The `tags` column is already in the v1 data model so v2 tag rendering doesn't require a schema migration.

**Auto-actions on status transitions.** Each ticket can have rules attached: "when this ticket enters status X, automatically send this prompt to its Claude session." Default rules apply to all tickets unless overridden. The canonical example: when a ticket lands in Attention required, auto-send "Read the new review comments on this PR and start addressing them. When done, run /handoff." When the handoff fires, the ticket flows on as normal.

Mechanics:
- Auto-actions fire from the same place that processes status transitions, *before* the notification fires
- The notification still fires after the auto-action runs, so the user knows the bot is working
- A global "pause auto-actions" toggle in the app footer for days when the user wants full manual control
- An audit log (the `AutoActionRun` table, already in the v1 schema) records every fire so the user can debug surprises
- Auto-action prompts support simple templating: `{ticket_id}`, `{ticket_title}`, `{pr_url}`, `{new_comments}` etc.

The `AutoAction` and `AutoActionRun` tables are in the v1 schema so v2 doesn't need a migration. v1 doesn't read or write them.

**Mobile companion app.** A thin client for iOS that connects to the user's running Loop instance on their Mac. Use case: leave the laptop open at home, control Loop from the phone while away.

Architecture:
- Loop runs a local HTTP + WebSocket server (loopback only by default, configurable to bind to LAN)
- The phone app connects directly over local network when on the same Wi-Fi
- For "anywhere" access, Loop documents Tailscale as the recommended setup. Tailscale gives the phone a private route to the Mac without exposing anything publicly. No relay server, no auth headache beyond a single shared token.
- The mobile app does not try to be a full IDE. It can: see the board, see ticket details, read recent Claude session output, send a prompt to a session, fire an auto-action manually, receive push notifications mirroring the Mac notifications
- The mobile app cannot: run services, show the browser preview, edit plans (read-only), open the PR overlay (just links out to GitHub mobile)

The command/event layer described above is the v1 work that makes this possible. Without it, v2 mobile becomes a rewrite. With it, v2 mobile is a thin server adapter over the same commands and events.

Authentication for the mobile companion: a single shared token generated on first pairing (QR code shown on the Mac, scanned by the phone), stored in the phone's keychain. No accounts, no cloud, no Loop-the-service.
