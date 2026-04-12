# Loop — Project Rules

## Git
- Never add Co-Authored-By or any Claude attribution to commits
- Repo is under bethandutton on GitHub, not any org

## Design
- Primary accent color is green (oklch hue ~155), not purple
- Visual reference is Linear — dense, keyboard-first, dark-mode-first

## UX
- Never make inputs purely manual — always add helpers: folder pickers, auto-detection, clickable links, pre-filled defaults
- API token fields need: eye toggle to show/hide, clickable link to where you create the token, description of required scopes/permissions

## Architecture
- API keys stored in macOS Keychain via `keyring` crate, never in SQLite or config files
- Follow the build plan in `docs/03-build-plan.md` phase by phase
- State flows through Tauri command/event layer — React components are subscribers, not owners of state
