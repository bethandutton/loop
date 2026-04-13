# Changelog

## v0.2.0 — Full workflow

### Board
- Flat ticket list with Linear-style status circle icons
- Filter by status, sort by status/priority/created/updated/title
- Search tickets by title or ID
- Right-click context menu: copy ID, open in Linear
- Create new tickets via Linear API
- Background polling with SQLite persistence
- Dynamic status mapping from Linear workflow states

### Plan editor
- Markdown preview with full GFM support
- Save to Linear
- Enhance with Claude via Anthropic API
- Conflict detection (warns when Linear version changes during editing)
- Move to Planning button for backlog/todo tickets
- Loading state for preview mode

### Claude Code sessions
- Per-ticket Git worktrees (auto-created from origin/main)
- Claude Code spawned in PTY per worktree
- xterm.js terminal with live output
- Session persistence across ticket switches
- Kill session support
- Scrollback buffered to disk

### Local environment
- Shared _local worktree with branch switching
- Service runner: detects package.json scripts, start/stop via PTY
- Browser preview iframe
- Running service status indicators

### GitHub integration
- GitHub REST API client for PRs, reviews, comments
- Background polling (60s) for PR status
- Auto status transitions: in_review, attention_required, ready_to_merge, done
- PR tab with info bar and embedded webview

### UI
- Tab-based layout (Plan, Session, Local, PR) instead of three panels
- Cmd+K command palette with fuzzy search
- Keyboard shortcuts: j/k navigation, Cmd+1-4 tab switching
- Floating panel design with gray canvas background
- Rounded app icon with macOS-compatible padding
- Toast notifications via Sonner
- Traffic light positioning

### Settings
- Anthropic API key field for Enhance with Claude
- Token persistence on save
- Theme, density, font size controls

### Release
- GitHub Actions workflow for macOS builds (Apple Silicon + Intel)
- Code signing and notarization support

## v0.1.0 — Skeleton

- Tauri app initialized with React, TypeScript, Tailwind CSS
- Three-column resizable layout (board, middle, right)
- CSS variable theming: dark (default), light, system
- Density controls: compact, comfortable, spacious
- Font size controls: small, medium, large
- SQLite database with full schema (including v2 tables)
- macOS Keychain integration for token storage
- First-run onboarding flow (Linear, GitHub, repo setup)
- Settings panel (Cmd+,)
- Command/event layer scaffolded
