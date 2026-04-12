# Contributing to Loop

Loop is intentionally narrow. It does one thing — manage many in-flight tickets across worktrees — for one kind of user — a solo developer with a slow review cycle. Contributions that improve the core loop, fix bugs, improve accessibility, or add genuine quality improvements are welcomed.

## Development setup

### Prerequisites

- macOS (Loop is a macOS-only app)
- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 20+
- A Linear account and API token
- A GitHub account and personal access token with `repo` scope

### Running from source

```bash
git clone <repo-url>
cd loop
npm install
npm run tauri dev
```

### Building

```bash
npm run tauri build
```

The built `.dmg` will be in `src-tauri/target/release/bundle/dmg/`.

## Contribution philosophy

Contributions that broaden Loop past its intended scope — multi-user features, team collaboration, web access, mobile-first redesigns, generic Kanban features — should be pointed to a fork. This isn't gatekeeping; it's focus.

If you're unsure whether your idea fits, open an issue to discuss before writing code.
