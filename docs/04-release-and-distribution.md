# Loop — Release and Distribution

How to ship Loop to other people. This doc is specifically about the macOS distribution process, which is the most fiddly part of the whole project after the worktree management.

This is the doc to read before phase 7 of the build plan. It's also the doc most likely to send Claude Code into rabbit holes if it tries to figure things out from scratch, so the rules of engagement are: follow this doc, and when in doubt ask the user rather than guessing.

---

## What "shipping" means for Loop

A user on the internet should be able to:

1. Find Loop on GitHub
2. Click a "Download for macOS" link in the README
3. Get a `.dmg` file
4. Open it, drag Loop to Applications
5. Launch Loop without macOS Gatekeeper warnings
6. Complete the onboarding flow
7. Start using it

That's the bar. Anything that gets in the way of those seven steps is a release blocker.

The "without Gatekeeper warnings" part is the hard one, and it's why this doc exists.

---

## Distribution channels

**GitHub Releases.** Tagged releases on the GitHub repo, each with a signed `.dmg` attached. This is the only distribution channel for v1.

**Not the Mac App Store.** Loop spawns subprocesses (Claude Code, dev servers), embeds webviews pointing at arbitrary localhost ports, accesses files outside its sandbox, and stores tokens in keychain. The App Store sandbox is incompatible with most of this and the review process for tools like this is painful. App Store distribution is explicitly not in scope.

**Not Homebrew (yet).** Once the app is stable and has users, a Homebrew Cask is a nice addition (`brew install --cask loop`). It's a follow-up after v1, not part of v1.

**Auto-updates: deliberately not in v1.** Tauri has an updater module, and it works, but it adds: a signing key for update manifests, an updates.json hosted somewhere, careful version comparison logic, and a UI for "an update is available." For v1, users update by downloading a new `.dmg` from GitHub Releases. The README's install instructions explain this. Auto-update can be added in a later release.

---

## Code signing

macOS will refuse to launch unsigned apps downloaded from the internet without scary "this app is from an unidentified developer" warnings. Even with a right-click bypass, it's a terrible first impression and will lose 90% of potential users at the door.

**Loop must be code-signed with an Apple Developer ID Application certificate.**

### Getting the certificate

The maintainer (initially Bethan, eventually anyone running official releases) needs:

1. An **Apple Developer Program** membership ($99/year). This is non-negotiable for distributing signed apps outside the App Store.
2. A **Developer ID Application** certificate created in the Apple Developer portal. This is the certificate used to sign apps for distribution outside the App Store.
3. The certificate installed in the macOS Keychain on the build machine, with both the certificate and its private key

This is a one-time setup per maintainer. The CONTRIBUTING.md should document that contributors don't need this — only the person publishing official releases needs the certificate. Forks and self-built copies can skip signing entirely (the user just runs from source or accepts the unsigned-app warning).

### Signing in the build

Tauri's bundler handles signing if the right environment variables are set:

- `APPLE_CERTIFICATE` (base64-encoded `.p12` export of the certificate)
- `APPLE_CERTIFICATE_PASSWORD` (password set when exporting the `.p12`)
- `APPLE_SIGNING_IDENTITY` (the certificate's common name, e.g. "Developer ID Application: Bethan Smith (TEAMID)")

These get set as GitHub Actions secrets for the release workflow. Locally, the maintainer can sign by setting them in the environment before running `tauri build`.

---

## Notarization

Code signing alone isn't enough. Since macOS Catalina, apps distributed outside the App Store also need to be **notarized**: uploaded to Apple's notary service, scanned for malware, and stamped with a notarization ticket. Without it, Gatekeeper still shows warnings.

### The notary service workflow

1. Build the app with Tauri (`tauri build`)
2. Tauri produces a signed `.app` bundle and a `.dmg`
3. Submit the `.dmg` to Apple's notary service via `xcrun notarytool submit`
4. Wait for Apple to scan (usually 1 to 15 minutes)
5. Once approved, "staple" the notarization ticket to the `.dmg` so it works offline (`xcrun stapler staple`)
6. The stapled `.dmg` is what gets uploaded to GitHub Releases

### Tauri's built-in notarization

Tauri can do this automatically as part of `tauri build` if these environment variables are set:

- `APPLE_ID` (the maintainer's Apple ID email)
- `APPLE_PASSWORD` (an app-specific password generated at appleid.apple.com, NOT the regular Apple ID password)
- `APPLE_TEAM_ID` (the Developer Program team ID)

When these are set alongside the signing variables, Tauri will sign, notarize, and staple in one step. This is what the GitHub Actions workflow should rely on.

### When notarization fails

It will, eventually. Common causes:

- An entitlement is required that isn't declared in the entitlements file (e.g. `com.apple.security.cs.allow-jit` if using a JIT for any reason)
- A binary inside the app bundle isn't signed (Tauri usually handles this, but bundled tools might not be)
- Hardened runtime is required and isn't enabled
- An app-specific password has expired or is wrong

The notary service returns a JSON log with the specific reason. The GitHub Actions workflow should download this log on failure and surface it in the workflow run. Don't try to handle notarization errors silently; surface them.

---

## Hardened runtime and entitlements

Notarization requires the **hardened runtime** to be enabled. Tauri does this by default. The entitlements file lives at `src-tauri/entitlements.plist`.

Loop's required entitlements (start with this set, add more only if necessary):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <!-- Allow spawning subprocesses (Claude Code, dev servers) -->
  <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
  <true/>
  
  <!-- Allow loading dynamic libraries (needed for some Node-based subprocesses) -->
  <key>com.apple.security.cs.disable-library-validation</key>
  <true/>
  
  <!-- Allow access to user-selected files (the repo folder) -->
  <key>com.apple.security.files.user-selected.read-write</key>
  <true/>
  
  <!-- Outbound network for Linear, GitHub, Anthropic APIs -->
  <key>com.apple.security.network.client</key>
  <true/>
  
  <!-- Inbound network for the local browser preview webview -->
  <key>com.apple.security.network.server</key>
  <true/>
</dict>
</plist>
```

Each entitlement weakens the security model slightly. The principle: include only what's needed, and document why each one is there in a comment in the actual entitlements file. If notarization fails for an entitlement reason, it's usually because *too few* are declared, not too many, but err on the side of fewer and add as needed.

---

## DMG packaging

Tauri builds a `.dmg` automatically. The default is functional but ugly. For v1 it's worth a small amount of polish:

- Custom DMG background image with an arrow pointing from the Loop icon to the Applications folder shortcut
- Window size and icon positions configured in `tauri.conf.json` under `bundle.macOS.dmg`
- App icon at `src-tauri/icons/icon.icns`, generated from a 1024×1024 source PNG via Tauri's icon CLI

The icon and DMG background are design assets — Loop should have a real icon, not the default Tauri rocket. A simple, recognizable mark in the indigo-violet of the app's primary color, designed to look good at both 16px (favicon) and 1024px (Retina app icon).

---

## GitHub Actions release workflow

A `.github/workflows/release.yml` that runs on tag push (`v*.*.*`) and produces a signed, notarized release.

Outline of what it does:

1. **Trigger:** push of a tag matching `v*.*.*`
2. **Runs on:** `macos-latest` (and ideally also `macos-14` for Apple Silicon native builds)
3. **Matrix:** build for both `aarch64-apple-darwin` (Apple Silicon) and `x86_64-apple-darwin` (Intel). Two separate `.dmg` files.
4. **Steps:**
   - Check out the code
   - Install Rust toolchain
   - Install Node and pnpm/npm (for the frontend)
   - Install frontend dependencies and build
   - Import the signing certificate from secrets (decode `APPLE_CERTIFICATE` from base64 and `security import` it into a temporary keychain)
   - Run `tauri build` with all the signing and notarization environment variables set
   - Wait for notarization to complete (Tauri does this synchronously)
   - Upload both `.dmg` files to the GitHub Release as assets
   - Optionally generate release notes from CHANGELOG.md or commit messages
5. **On failure:** preserve and upload notarization logs as workflow artifacts so they can be inspected

### Required GitHub Actions secrets

The maintainer sets these once in the repo's Settings → Secrets:

- `APPLE_CERTIFICATE` (base64 of the `.p12` export)
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD` (app-specific password)
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD` (any string; used to create the temporary keychain in CI)

These secrets are never visible in workflow logs and never accessible to PRs from forks (GitHub blocks secret access from fork PRs by default; do not change this).

---

## Versioning

Semantic versioning: `MAJOR.MINOR.PATCH`.

- v1 launches as `v1.0.0`
- Bug fixes increment patch (`v1.0.1`)
- New features increment minor (`v1.1.0`)
- Breaking changes (e.g. database schema migrations that aren't backward-compatible) increment major (`v2.0.0`)

The version lives in three places that must stay in sync:

1. `package.json` (frontend)
2. `src-tauri/tauri.conf.json` (Tauri bundle version)
3. `src-tauri/Cargo.toml` (Rust crate version)

A small script (`scripts/bump-version.sh`) should update all three at once. The release workflow assumes the tag matches the version in these files; CI should verify and fail loudly if they don't match.

---

## Release checklist (for the maintainer)

Before tagging a release:

- [ ] All v1 phases from the build plan are done
- [ ] CHANGELOG.md has an entry for the new version
- [ ] The version is bumped in `package.json`, `tauri.conf.json`, and `Cargo.toml`
- [ ] The app launches cleanly on a fresh Mac (test on a separate user account or VM)
- [ ] The onboarding flow works end to end with real Linear and GitHub tokens
- [ ] The README's screenshots are up to date
- [ ] No secrets are committed to the repo (audit `.env` files, hardcoded tokens, etc.)
- [ ] `tauri build` succeeds locally with signing env vars set
- [ ] Notarization succeeds locally (test before relying on CI for it)

Then:

- [ ] Push the tag (`git tag v1.0.0 && git push --tags`)
- [ ] Watch the GitHub Actions workflow
- [ ] Once the release is published, download the `.dmg` from a clean machine and verify it installs and runs without warnings
- [ ] Tweet, post, share, whatever — the release is live

---

## Things that will go wrong (be honest with the user about these)

**Notarization will fail at least once.** Probably for an entitlements reason. The notary service log will tell you exactly what's wrong. Don't panic.

**The first signed build will produce a "damaged app, move to trash" error** if the keychain password gets mangled or the certificate isn't fully trusted. This is almost always a CI environment issue, not a real problem with the app. Test the build flow locally before trusting CI.

**Apple's notary service goes down sometimes.** When it does, releases just have to wait. There is no workaround. Plan releases for Apple's working hours (Pacific time business hours) to minimize the risk.

**The Apple Developer Program membership has to be renewed annually** ($99/year). If it lapses, all signed apps stop launching cleanly. Set a calendar reminder for the renewal date.

**App-specific passwords expire** after a year of disuse. The release workflow will mysteriously start failing at the notarization step. Generate a new one at appleid.apple.com and update the secret.

These aren't failures of Loop or the build process — they're inherent to distributing signed Mac apps outside the App Store. The CONTRIBUTING.md should set expectations for anyone considering taking over maintenance.

---

## What's documented in the user-facing README vs. here

The repo's main `README.md` (the one users see on GitHub) should have:

- What Loop is (one paragraph)
- Screenshots
- "Download for macOS" link to the latest release
- Install instructions: download `.dmg`, drag to Applications, launch
- Quickstart: complete the onboarding, create a ticket, watch the loop work
- Link to the spec docs in the `docs/` folder for anyone who wants to understand how it works
- Link to CONTRIBUTING.md
- License (MIT)
- A note that Loop has no telemetry and never phones home

This release/distribution doc is a *developer* reference, not a user reference. It doesn't go in the README.
