# Loop

A macOS desktop app for managing many in-flight Linear tickets across many Git branches in a single repo, without losing context.

Loop gives every active ticket its own persistent Git worktree and its own background Claude Code session, surfaced through a single board where you can jump between tickets in one click and pick up exactly where each one left off.

## Status

**v0.1.0 — Skeleton.** The app opens, runs onboarding, shows the three-column layout, and persists settings. Real ticket management is coming in the next phase.

## Install

Download the latest `.dmg` from [GitHub Releases](../../releases) (coming soon), or build from source:

```bash
# Prerequisites: Rust, Node.js 20+
npm install
npm run tauri dev
```

## Privacy

**Loop has no telemetry. Ever.** No crash reporting, no usage analytics, no network calls except to the Linear API, GitHub API, and Anthropic API — all with your own tokens.

## License

MIT — see [LICENSE](LICENSE).
