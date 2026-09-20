# Titen Web — Component Standards

> What is canonical, what is deprecated, and the contract every page must follow.
> Token vocabulary: see [TOKENS.md](./TOKENS.md).

## Canonical Components

| Concern | Use | Notes |
|---|---|---|
| Buttons | `ui/button` `<Button>` | variants: `default`, `secondary`, `destructive`, `outline`, `ghost`; success = `default` + `class="bg-[var(--color-success)]"` |
| Forms | `ui/field` (`Field`, `FieldLabel`, `FieldDescription`, `FieldError`) + `ui/input`, `ui/select`, `ui/switch`, `ui/textarea` | every input has a label; validation errors link via `aria-describedby` + `aria-invalid` |
| Dialogs | `ui/dialog`, `ui/alert-dialog` | long content scrolls (`.detail-body`); full-bleed ≤30rem; destructive actions state the consequence in copy |
| Status display | `StatusBadge` | the only badge renderer — do not hand-roll status colors |
| Empty states | `EmptyState` | everywhere: tables, cards, calendar, dashboard lists |
| Tables | `DataTable` | provides skeleton loading, sort, expandable rows, keyboard row activation, `hideOnMobile` |
| Layout | `PageHeader` | renders the page `h1` + description + action slot |
| Media preview | `MediaLightbox` | Escape/backdrop/X close + body scroll lock — reuse this pattern for other overlays |
| Confirm flows | `ConfirmDialog` | destructive + irreversible actions (delete, HITL gates) |

**Deprecated:** the legacy `@layer components` classes (`.btn-*`, `.form-*`) were removed. shadcn primitives are the only component system. Raw `<button>`/`<select>`/`<table>` in pages are review blockers.

## Contracts

### Toast (feedback)
- Container: `role="status"` `aria-live="polite"`; error items `role="alert"`.
- Every toast carries a type icon (non-color cue). Success and error must be distinguishable without color.
- Every async action surfaces a toast on success AND failure. Error copy is human; raw `e.message` goes to console.

### Loading
- Any wait >150ms renders a **skeleton** shaped like the real content (`StatSkeleton`, `Skeleton` rows in `DataTable`, calendar cell grid). Bare "Loading…" text is not acceptable.
- Keep previous data visible during refetch where possible.

### Empty
- Zero-state uses `EmptyState` with a concrete next action ("Add your first account…"). Distinguish "no data" from "no results for filters" in copy.

### Destructive / HITL actions
- Irreversible or publish-triggering actions require two-stage confirmation (arm → confirm within 3s, `aria-pressed`) or `ConfirmDialog`.
- Reject-style flows collect a reason; approve-style flows state the publish consequence.
- Optimistic updates must revert on error and surface a failure toast.

### Icons
- Lucide only (`@lucide/svelte/icons/*`). No emoji in chrome. Distinct icons per nav item.

### Focus & Motion
- Global instant `:focus-visible` ring (`--color-focus`) — never remove, never animate.
- Durations from `--dur-*`, easing `--ease-out`; nothing above 400ms; `prefers-reduced-motion` honored globally.
- Interactive targets ≥40px on touch (`app.css` table rules handle this — keep them).

### Dark-mode readiness
- Only token references (`var(--…)`) in component CSS. Any new hardcoded color is a review blocker (the palette is light-theme today; tokens are the migration path).
