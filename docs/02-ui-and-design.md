# Loop — UI and Design

How Loop should look and feel.

The high-level reference: **Loop should feel like Linear.** Dense, fast, keyboard-first, sharp typography, a distinctive dark-leaning palette, thin borders, small icons, and an "engineered" feel that respects the user's time and attention. Loop's users are developers managing many concurrent tickets — they want to see a lot of state at once, move fast, and use the keyboard for everything.

This is a deliberate shift away from a "calm chat interface" direction. Loop is a board with a terminal attached, not a chat with a board attached. The board, the cards, and the right column should all feel dense and information-rich. The middle column terminal is the one place that gets to breathe, because terminals are terminals.

---

## Stack

- **shadcn/ui** as the primary component library. Built on Radix primitives, themeable via Tailwind, components are owned in-tree so they can be modified freely. Do not pull in a different general-purpose component library.
- **React Aria** as an escape hatch for specific primitives where it's clearly better than shadcn's equivalent. The first place this applies is the **Cmd+K command palette / ticket switcher**, where React Aria's combobox primitive is more battle-tested. Use React Aria sparingly, only where the win is obvious.
- **Tailwind CSS** for styling, configured with CSS variables for theming.
- **Lucide React** for icons (shadcn's default).
- **Inter** as the UI font, **JetBrains Mono** for the terminal and any code surfaces.
- **xterm.js** for the Claude Code terminal in the middle column. Theme it to match the active app theme.

---

## Visual language

### Tone

- **Dense.** Tight spacing throughout. More information per square inch than a typical app. The user should be able to see 15 to 20 ticket cards at once on the board without scrolling.
- **Sharp typography.** Inter at smaller sizes than usual: `text-xs` (12px) and `text-[13px]` are common, `text-sm` (14px) is the default body size, anything larger is reserved for actual emphasis. Line-height tight (`leading-tight` or `leading-snug`). Letter-spacing slightly negative on headings (`tracking-tight`).
- **Thin borders as structure.** A 1px border in a low-contrast color is the primary way of separating regions. Borders are part of the architecture of the screen, not just dividers. Avoid heavy shadows or large gaps where a thin border would do.
- **Confident accent color.** Loop has one accent color used consistently for: the active ticket highlight, primary buttons, focus rings, the unread output dot, the current branch indicator. Don't be shy with it but don't sprinkle it on everything either. Suggested default: a desaturated indigo-violet (similar in spirit to Linear's purple but not a copy). Define it as `--primary` in the theme tokens.
- **Speed as a design value.** Transitions are 50 to 100ms, not 150 to 300ms. Hover states snap. Loading spinners are rare, optimistic UI is the default. The user should never wait for the UI to catch up with their click.

### Typography scale

| Use | Class | Size |
|---|---|---|
| Tiny labels, column headers | `text-[11px] uppercase tracking-wide font-medium` | 11px |
| Card metadata (ID, badges) | `text-xs` | 12px |
| Card titles, body text | `text-[13px]` | 13px |
| Default body | `text-sm` | 14px |
| Editor (plan view) | `text-[15px]` leading-relaxed | 15px |
| Section headings (rare) | `text-base font-semibold tracking-tight` | 16px |

The plan editor is the only place text gets bigger than default. It's where the user does sustained reading and writing, so it earns the size.

### Color and meaning

A small palette, used with intent:

- **Background, surface, surface-elevated, foreground, muted-foreground, border, border-strong** are the neutral structure of the app. Defined as CSS variables, themed per mode.
- **Primary** (the accent) for active states, primary buttons, focus rings, unread indicators
- **Success** (green) for Ready to merge column dots, passing CI states
- **Warning** (amber) for Attention required column dots, review needed states
- **Destructive** (red) for error states, kill-session button, failed CI
- **Info** (blue) for In review column dots, neutral informational states

Status colors live as small dots (not full badges) on cards. The column the card is in already tells you the status, the dot is reinforcement, not the primary signal. This means color is never the only way to read the board, which keeps it accessible.

---

## Theming

Loop supports **dark mode (default), light mode, and system mode** at v1, with the architecture set up so additional themes can be added later without touching components.

**Dark mode is the default**, not light. Linear is a dark-mode-first product and Loop's audience expects the same. Light mode is a deliberate, well-supported alternative for users who want it, not an afterthought.

### How theming works

- All colors are defined as **CSS variables** in `:root[data-theme="dark"]` (default) and `:root[data-theme="light"]`. Components reference these variables via Tailwind classes (`bg-background`, `text-foreground`, `border-border`, etc.), never hardcoded color values.
- Theme switching writes a `data-theme` attribute on `<html>`. The `dark` class is also toggled for compatibility with Tailwind's dark variant utilities.
- The user's choice (dark, light, system) is stored in the `Settings` table and applied on app startup. System mode listens to `prefers-color-scheme` and updates live when the OS changes.
- The Settings panel has a theme picker as a segmented control: System / Light / Dark.

### Adding themes later

The architecture supports adding new named themes (e.g. high contrast, midnight, solarized) without rewriting components:

- Define each theme as a named CSS variable set: `:root[data-theme="midnight"] { --background: ...; --foreground: ...; }`
- The theme picker reads from a registered list of themes; switching writes the corresponding `data-theme` attribute
- Components never reference theme names directly. They reference CSS variables. If the variables are defined, the theme works.

This is standard shadcn/Tailwind practice. The point of calling it out is so the v1 build doesn't accidentally hardcode `bg-zinc-900` or `text-white` anywhere. **Every color must come from a token.**

### Suggested dark mode palette (starting point, tune visually)

These are reference values, not commandments. The theme tokens should be tuned in the actual app, with real content, until they look right. But this gives a starting direction:

- `--background`: very dark, slightly cool. Around `oklch(0.15 0.005 270)`, close to black but not pure
- `--surface`: one notch lighter, used for cards and panels
- `--surface-elevated`: one notch lighter again, used for popovers, dropdowns, modals
- `--foreground`: not pure white. Around `oklch(0.95 0 0)`, soft, easy on the eyes
- `--muted-foreground`: around `oklch(0.65 0 0)` for secondary text
- `--border`: very subtle, around `oklch(0.25 0.005 270)`
- `--border-strong`: slightly more visible, for the few places a stronger separator is needed
- `--primary`: a confident accent color. Indigo-violet range, around `oklch(0.65 0.2 280)`
- `--primary-foreground`: white-ish for text on primary

The light mode palette mirrors this structure with inverted lightness values, but should be checked visually rather than mathematically inverted.

### Terminal theming

`xterm.js` accepts a theme object with foreground, background, cursor, selection, and ANSI colors. Loop should:

- Read the current app theme on startup and pass a matching theme object to xterm.js
- Re-theme xterm.js when the user changes the app theme (no restart)
- Define ANSI colors as part of each theme's tokens (16 colors: 8 normal + 8 bright)
- Use JetBrains Mono in the terminal at the user's selected font size
- Match the terminal background to `--surface` (not `--background`) so the terminal feels like a panel inside the app, not a hole in it

---

## Density and font size

The user can adjust both **density** (how much breathing room) and **font size** (the base text size of the whole app), independently. Stored in `Settings`.

Note: even at the most spacious density setting, Loop is denser than a typical app. The density slider tunes within Loop's range, not relative to the whole web.

### Density

Three options: **Compact, Comfortable (default), Spacious.**

Implementation: density maps to a CSS variable `--density-scale` (e.g. `0.85`, `1.0`, `1.15`) that multiplies the spacing tokens used in components. Practically:

- Define semantic spacing tokens at the root: `--space-card-padding`, `--space-card-gap`, `--space-list-padding`, `--space-button-y`, etc.
- Each token's value is a base value times `--density-scale`
- Components use these tokens, not raw Tailwind spacing

The simplest concrete approach: define a few base custom utility classes (`p-card`, `gap-list`, etc.) backed by these CSS variables. Pick whichever is cleanest. The goal is that flipping a single setting visibly changes breathing room throughout the app without touching component code.

### Font size

Three options: **Small, Medium (default), Large.**

Implementation: maps to a `font-size` on the `:root` element (e.g. `13px`, `14px`, `15px`). All other text in the app is sized in `rem` units, so they all scale together. shadcn components use `rem` by default, do not override with fixed `px` values.

Note the smaller defaults compared to a typical app: medium is 14px, not 16px. This is the Linear-density direction. Users who want larger text can pick Large (15px) or override with their OS-level zoom.

The terminal in the middle column also respects this setting, mapped to fixed font-sizes in `xterm.js` config: small = 12px, medium = 13px, large = 15px.

### Why both axes

Density and font size are different. A user might want compact spacing with large text (accessibility) or comfortable spacing with small text (information density). Treating them as one slider would be wrong.

### Where the controls live

- **In Settings**, as two segmented controls
- **A small dropdown in the app footer** for quick access to theme, density, and font size. This is more important in a Linear-style dense UI than it would be otherwise, because users will want to tune their setup live

---

## Layout

Three columns, full window height. **No header bar above them**, the app uses native macOS window chrome (traffic lights only). A thin **footer** runs the full width at the bottom for status info and quick controls.

### Column proportions

Default proportions, resizable by dragging the column dividers:

- **Left (Board):** 260px fixed-min, 300px default. Tighter than a typical sidebar.
- **Middle (Plan or Session):** flexible, takes remaining space. The focus area.
- **Right (Local):** 380px fixed-min, 440px default. Can be hidden via a footer toggle.

Resize handles between columns are 1px borders that highlight on hover. Use shadcn's `Resizable` component (it wraps `react-resizable-panels`).

Column widths are persisted in `Settings`.

### Footer

A 28px-tall footer running the full width of the window, divided into sections:

- **Left:** active session count ("4 sessions running"), rate limit indicator if available
- **Center:** any active background process status (polling indicator that's silent unless something is happening)
- **Right:** quick controls — theme toggle, density toggle, font size, hide-right-column toggle, settings gear

Footer text is `text-[11px] text-muted-foreground`. The whole footer is one line, no wrapping.

### The board (left column)

- **Top bar:** "Loop" wordmark on the left in small caps, **+** button (new ticket) on the right. Single line, ~32px tall.
- **Below:** vertical scrollable stack of column groups. Each is one of the nine status columns from the product spec.
- Each group has a header: column name in `text-[11px] uppercase tracking-wide font-medium text-muted-foreground` on the left, count badge on the right. Single line, no separator below the header (the cards' top border does that work).
- Tickets within a group are cards stacked vertically with a small `gap-1` (4px) between them. Tight.
- The active ticket card has a left accent border (2px, primary color) and a slightly elevated background (`--surface-elevated`).
- Empty columns show a thin "—" placeholder in `text-muted-foreground/50`. Not "No tickets," just a dash. Quieter.

### Ticket cards

Compact and information-dense. Roughly 56 to 72px tall depending on whether badges wrap. Layout:

- **Row 1** (single line): ticket ID in mono on the left (`text-[11px] text-muted-foreground`), priority icon and any state badges right-aligned
- **Row 2** (one or two lines max): title in `text-[13px]`, truncated with ellipsis after 2 lines
- **Bottom-right corner:** unread output dot when applicable, 6px filled circle in `--primary`

Hover state: subtle background shift (`--surface` to `--surface-elevated`).
Active state: left border + elevated background (described above).
Click target: the entire card.

No card padding wider than `px-3 py-2`. Tight.

### The middle column

- **Top toolbar** (32px tall): ticket ID in mono + title on the left, mode-specific actions on the right (Enhance/Save in Plan mode, PR overlay toggle + Kill in Session mode). Single line, separated from the content below by a 1px border.
- **Below the toolbar:** the editor or terminal, filling the remaining space.
- **Padding:** 
  - Plan mode: `p-8` for generous reading space (the one place Loop is *not* dense; sustained reading deserves room)
  - Session mode: `p-2` to `p-3`, the terminal wants to be close to the edges so xterm.js has maximum width for output
- **Empty state** (no active ticket): centered icon + one sentence in `text-muted-foreground`. Quiet. Don't show tutorials, don't show a list of features.

### The right column

- **Top:** branch context bar, ~28px, single line. Branch name in mono, ticket ID in muted text. 1px border below.
- **Middle:** service runner panel. Each detected script as a row: checkbox on the left, script name in mono in the middle, status indicator (running/stopped/errored dot) on the right. Below the list: a single primary button — **Run** when nothing is running, **Stop all** when services are running. Clicking a script name expands a drawer below it with that service's PTY output.
- **Bottom (largest area):** browser preview as a Tauri webview, taking remaining vertical space. Address bar above the webview is hidden by default — the URL is fixed to `localhost:<port>` and a small badge in the corner shows the port.

When no ticket is active or the right column is hidden, the column collapses out of the layout entirely.

---

## Interaction patterns

### Keyboard-first

This is a Linear-style app, which means the keyboard is a first-class input. Mouse works for everything but the user should rarely need it.

Required shortcuts for v1:

| Shortcut | Action |
|---|---|
| `Cmd+K` | Open command palette (ticket switcher + actions). The most important shortcut, must feel instant. |
| `Cmd+,` | Open Settings |
| `Cmd+N` | New ticket |
| `Cmd+1` / `Cmd+2` / `Cmd+3` | Focus left / middle / right column |
| `Cmd+B` | Toggle right column visibility |
| `Cmd+/` | Toggle PR overlay (when in Session mode) |
| `j` / `k` | Move down / up through tickets in the board (when board is focused) |
| `Enter` | Activate the focused ticket |
| `Cmd+Enter` | In Plan mode: save plan to Linear |
| `Esc` | Close any open modal / dialog / palette |

The command palette (`Cmd+K`) is the heart of keyboard navigation. It should:
- Open instantly with no animation (or a 50ms fade)
- Show all tickets first, fuzzy-searchable by ID, title, or column
- Also expose actions: "Create ticket", "Open settings", "Switch theme", "Toggle right column", etc.
- Use React Aria's combobox primitive for the input. This is one of the few places React Aria buys us a real win over shadcn.

### Click is the secondary action

- Click a card → make it active
- Click a service checkbox → toggle run state
- Click a button → do the thing
- No drag-and-drop in v1

### Confirmations

shadcn's `AlertDialog` for destructive or context-changing actions:
- Switching the local to a different ticket (will stop running services)
- Killing a Claude session
- Closing/cleaning up a ticket

shadcn's `Sonner` (toast) for non-blocking feedback. Toasts in Loop are *small* and *fast*: bottom-right corner, single line, auto-dismiss in 2 seconds:
- "Plan saved"
- "Branch created"
- "Session handed off"

### Notifications

Native macOS notifications (via Tauri) for events that happen while the app is in the background:
- New CodeRabbit comment
- New human review
- Handoff fired
- Session errored

In-app, the same events also produce a subtle visual update on the relevant card.

---

## Empty states, loading states, errors

### Empty states

Quiet and small. Linear-style. A small icon, one short sentence in `text-muted-foreground`, no call-to-action button unless there's literally nothing else the user can do. Examples:

- No active ticket → "Pick a ticket to get started."
- No tickets at all → "No tickets. Press Cmd+N to create one."
- Empty board column → just a "—" line

Notice the second example shows the keyboard shortcut directly, not a button. That's the Linear way: teach the shortcut, don't hide it behind a click.

### Loading states

- **Initial app load:** show the layout immediately with skeleton cards in the board (shadcn `Skeleton`), terminal placeholder in the middle, empty service list on the right.
- **Polling refreshes:** silent. No spinner. The user does not need to know polling is happening.
- **User-triggered actions:** small inline spinner inside the button, never a global loading bar.

### Errors

- **API errors:** small toast with the error and a "Retry" or "Open settings" link.
- **Session errors:** the ticket card shows a red dot; the middle column shows the error in the terminal area with a "Restart session" button below.
- **Branch creation errors:** modal with the git error verbatim and a "Copy" button. Don't try to recover automatically.

---

## Accessibility

- All interactive elements reachable by keyboard
- Focus visible at all times. Keep shadcn's focus rings, do not remove them. Linear-density doesn't mean removing affordances, it means making them efficient.
- Color is never the only signal (status dots reinforce the column position; icons accompany color where space allows)
- Respect `prefers-reduced-motion` for animations
- Font size scaling is itself an accessibility feature

The combination of dense + small text + dark mode is risky for accessibility if done badly. The font-size and density controls exist partly for users who can't comfortably use the defaults. Make sure they actually work: test the app at "Spacious + Large" and make sure it's pleasant.

---

## Animation

Minimal, fast, functional.

- **Use:** hover transitions (50 to 80ms), modal/popover entrances (shadcn defaults trimmed to 100ms), the active-card highlight sliding when active ticket changes (100ms), command palette fade-in (50ms or none)
- **Do not use:** bouncing, scaling, parallax, decorative motion, anything that draws the eye

If in doubt, leave it static. Speed feels better than motion.

---

## Reference

When making styling decisions, the visual reference is **Linear**, not ChatGPT, not VS Code, not Notion. Specifically:

- Linear's sidebar density and the way it stacks ticket info
- Linear's command palette (`Cmd+K`), the gold standard for this pattern
- Linear's typography: small, sharp, confident
- Linear's use of one accent color throughout
- Linear's keyboard-first ethos: every action has a shortcut, shortcuts are visible in tooltips and menus
- Linear's "feels engineered" quality: tight, considered, fast

Loop is not a Linear clone and shouldn't try to be. But if a styling decision feels uncertain, "what would Linear do?" is the right question to ask.
