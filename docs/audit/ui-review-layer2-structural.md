# Titen Web UI Review — Layer 2 (Structural)

**Scope:** `routes/+layout.svelte`, `routes/+page.svelte`, `routes/login`, `routes/auth/callback`, `routes/admin/+layout.svelte`, all 9 `routes/admin/*/+page.svelte`, shared `lib/components/{ConfirmDialog,DataTable,EmptyState,Icon,PageHeader,PostDetail,ScheduleDetail,StatSkeleton,StatusBadge}.svelte`
**Baseline:** `src/app.css` (Hallmark OKLCH token system) — read first, treated as source of truth.
**Stack:** SvelteKit (Svelte 5 runes) + shadcn-svelte + Tailwind v4. Stock `lib/components/ui/*` excluded from findings.

---

## 🔴 CRITICAL

1. **`transition: all 0.2s ease` in media dropzone** (`routes/admin/media/+page.svelte:398`). The one `transition:all` in app code — repaints every animatable property (including layout-ish ones), and the bare `ease` keyword overrides the token curve. Also pairs with a `scale(1.01)` drag state (minor, but transition:all makes it janky).
2. **Settings page bypasses shadcn entirely.** `routes/admin/settings/+page.svelte` builds tabs, text inputs, number inputs, checkbox toggles, and badges from raw `<button>`/`<input>` + ~230 lines of hand-rolled scoped CSS (`settings-tab`, `form-input`, `form-toggle`…), while `ui/tabs`, `ui/input`, `ui/switch`, `ui/badge` are all installed. Two parallel design systems on one page → focus rings, keyboard nav, and dark tokens will diverge from every other page. Same story for the custom tab strip vs `ui/tabs`.
3. **Hand-rolled modal pattern instead of Dialog/AlertDialog** in `routes/admin/accounts` (`.confirm-overlay`, line 294), `schedules` (3 modals: 483, 616, 669), `mentions` (`.reply-overlay`, 177), and `lib/components/PostDetail`/`ScheduleDetail` (`.detail-overlay`). `role="dialog" aria-modal="true"` is set, but there's **no focus trap, no focus restore, no scroll lock** — a modal that doesn't trap focus is an a11y failure and inconsistent with Media page which correctly uses `ui/dialog` + `ConfirmDialog`.
4. **Dead shadcn inventory: 60 component dirs installed, ~16 used.** Unused: accordion, aspect-ratio, avatar, breadcrumb, button-group, calendar, card, carousel, chart, collapsible, command, context-menu, drawer, dropdown-menu, field, form, hover-card, input-otp, item, kbd, menubar, navigation-menu, pagination, popover, progress, radio-group, range-calendar, resizable, scroll-area, sidebar, slider, sonner, spinner, tooltip (only referenced inside other ui/ files), toggle/toggle-group (ui-internal), etc. ~44 orphan dirs bloat the repo, confuse contributors ("is pagination ours?"), and Media page even hand-rolls prev/next pagination while `ui/pagination` sits unused.

## 🟡 MEDIUM

5. **Phantom-token fallback hex values** — `var(--color-warning-bg, #fef3c7)` in `schedules:739` and `comments:516-522`, `var(--color-bg-elevated, #f3f4f6)` (`schedules:810`), `var(--color-warning-border, #fcd34d)` (`app.css:1082`), `var(--color-accent, #3b82f6)` (`PostDetail:256`). All these tokens **are** actually defined in app.css (compat alias block, lines 51–64), so the fallbacks are dead weight — but they're light-mode-only hexes that would silently break dark mode if the alias ever gets removed. Delete fallbacks or delete the aliases.
6. **Icon-only buttons without aria-label** — Media delete `<Trash2>` button (`media:293-299`) has no aria-label and no title (its sibling Copy button has `title="Copy URL"` but not `aria-label`; title is a weak substitute). All other pages use text buttons, so this is contained, but the pattern will spread.
7. **No responsive handling on dense table pages.** `accounts` (9 cols), `posts` (9), `schedules` (8), `mentions` (7), `media` (6): zero `md:`/`lg:` utilities and zero `@media` queries. They rely solely on the global `.data-table-wrap { overflow-x: auto }` (app.css:550) — horizontal scroll is the *floor*, not a solution: on mobile the primary content (post caption, username) is off-screen. Analytics and dashboard are the only pages with real `@media (max-width: 48rem)` blocks. Comments gets credit for a mobile card-ish layout (min-width: 12rem) but still no breakpoint.
8. **Settings danger-zone + purge/delete calls raw `fetch()`** (`settings:146,159`) instead of the `$lib/api` client — no ApiError handling parity, and the "endpoint may not be available yet" copy leaks implementation doubt into the UI.
9. **Cross-page template repetition (anti-slop):** all 9 admin pages share the identical `PageHeader → error/toast → skeleton table → DataTable/empty-state` rhythm, which is good consistency — but the empty states are near-identical generic copy ("No X yet" + "X will appear once…"), and dashboard/posts/mentions reuse the same `.detail-overlay` dialog CSS duplicated in PostDetail and ScheduleDetail (~200 lines duplicated, including the same media-onerror handler inline 3×). Extract a shared DetailDialog/Modal shell.
10. **`ScheduleDetail` uses raw `<textarea>`** (line 213) despite `ui/textarea` being installed and used elsewhere (4 usages of ui/textarea across routes). Also `svelte:window on:keydown` (deprecated `on:` syntax) in both detail components; Svelte 5 pages elsewhere use `onclick=`.
11. **Settings loading state is text-only** (`Checking…`, `Saving…` buttons, no skeletons) — acceptable for a form, but the tab-switch content pops in with no transition and no skeleton, the weakest loading story of the 9 pages.
12. **`Icon.svelte` duplicates lucide** — hand-copies 9 lucide paths into snippets while `@lucide/svelte` is already a dependency and used directly in EmptyState/media. Two icon systems; the lucide direct imports should win.

## 🟢 GOOD patterns

- **Zero `window.confirm()`** anywhere — all destructive actions go through `ConfirmDialog` (AlertDialog-based), with type-to-confirm for the extreme danger zone. Excellent.
- **Token discipline is strong:** zero hardcoded hex outside fallback args, zero Tailwind palette classes (`text-green-600`, `bg-white`, `gray-*`: 0 hits) in app code. Pages consistently use `var(--color-*)`, `--space-*`, `--radius-*`, `--text-*` from app.css.
- **Loading states are skeleton-first:** every data page uses `Skeleton` rows inside the real table header (layout-shift-free: dashboard, accounts, posts, schedules, comments, media, mentions, analytics all skeletal; StatSkeleton for stat cards). Analytics trend chart has its own skeleton block.
- **Motion is restrained:** durations are `--dur-short`/`--dur-base` tokens, `--ease-out` standard, no `scale(0)` entrances, no >300ms entrances (the 1.5s `pulse`/`skeleton-pulse` loops are loading indicators — correct usage). Reduced-motion handled in app.css.
- **`DataTable` is a genuinely good abstraction** — shadcn Table + built-in sort, skeleton, and empty state, used consistently by 5 pages. `EmptyState` wraps `ui/empty` with lucide icons. `StatusBadge` maps statuses to token classes (`bg-[var(--color-success)]`), not palette colors.
- **Dark mode structurally safe:** tokens are OKLCH with `.dark` variants; pages reference semantic tokens, not raw values. Analytics has real mobile handling (stacked filter bar, hidden bar labels).

---

## 📊 Scorecard

| Area | Grade | Notes |
|---|---|---|
| A. shadcn adoption | **C+** | Buttons/Select/Table/Dialog used well; Settings page + hand-rolled modals + ScheduleDetail textarea bypass it |
| B. Component dead-code | **D** | 60 installed / ~16 used; Icon.svelte duplicates lucide; ui/pagination unused while Media hand-rolls pagination |
| C. Loading states | **A-** | Skeletons everywhere with correct table shape; Settings is text-only (minor) |
| D. Dark-mode safety | **B+** | Clean token usage; dock phantom-fallback hexes in schedules/comments/PostDetail |
| E. Responsive | **C-** | Global overflow-x crutch only; 5 dense table pages have zero breakpoints |
| F. A11y | **B-** | No window.confirm, aria-labels on dialogs/selects; but no focus trap in 6+ hand-rolled modals; 1 unlabeled icon button |
| G. Anti-slop | **B** | Consistent rhythm (good); generic empty-state copy, duplicated overlay CSS (~200 lines ×2) |
| H. Motion | **A-** | One `transition:all` (media dropzone); everything else tokenized and ≤ base duration |

**Overall: B−** — strong token and loading foundations; the debt is concentrated in the Settings page, hand-rolled modals, and the unused shadcn inventory.

## 🛠️ Fix priority

| # | Fix | Effort | Impact | Files |
|---|---|---|---|---|
| 1 | Replace hand-rolled overlays with `ui/dialog`/`ui/alert-dialog` (focus trap + scroll lock for free) | M | High | accounts, schedules×3, mentions, PostDetail, ScheduleDetail |
| 2 | Rebuild Settings tabs/inputs/toggle on `ui/tabs`, `ui/input`, `ui/switch`, `ui/badge`; delete ~230 lines scoped CSS | M | High | routes/admin/settings/+page.svelte |
| 3 | Delete unused `ui/*` dirs (keep list: button, select, table, dialog, alert-dialog, textarea, input, label, skeleton, separator, empty, badge, sheet, input-group, tooltip if adopted); keep components.json for re-adds | S | Med (repo hygiene) | lib/components/ui/* |
| 4 | Remove phantom-fallback hexes; rely on real tokens (or remove legacy aliases after audit) | S | Med | schedules, comments, PostDetail, app.css:1082 |
| 5 | `transition: all 0.2s ease` → `transition: border-color/background/transform var(--dur-base) var(--ease-out)` | S | Med | routes/admin/media/+page.svelte:398 |
| 6 | Add `md:`/`@media` handling to dense tables: hide secondary cols on mobile or switch to stacked cards (pattern already exists in comments) | M | High (mobile) | accounts, posts, schedules, mentions, media |
| 7 | aria-label + Tooltip on icon-only buttons (Media copy/delete); adopt ui/tooltip (2 uses already internal) | S | Med | routes/admin/media/+page.svelte |
| 8 | Swap raw `<textarea>` → `ui/textarea`; `on:keydown` → `onkeydown` | S | Low | ScheduleDetail.svelte |
| 9 | Extract shared DetailDialog shell from PostDetail/ScheduleDetail (dedupe ~200 lines + triple onerror handler) | M | Med | lib/components/PostDetail, ScheduleDetail |
| 10 | Replace Icon.svelte lucide-copies with `@lucide/svelte` imports | S | Low | lib/components/Icon.svelte, admin/+layout.svelte |
| 11 | Route danger-zone `fetch()` through `$lib/api`; drop "may not be available yet" copy | S | Low | routes/admin/settings/+page.svelte |
| 12 | Use `ui/pagination` on Media page instead of hand-rolled Prev/Next | S | Low | routes/admin/media/+page.svelte |
