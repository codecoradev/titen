# Titen Web Dashboard

SvelteKit 5 + shadcn-svelte + Tailwind v4 frontend for the Titen Threads API manager (proxied to the Rust API by `src/hooks.server.ts`).

## Design System

- **[design/TOKENS.md](./design/TOKENS.md)** — design-token spec (Hallmark/Cobalt, OKLCH, hue 260). The single source of truth for color, spacing, type, radii, motion, and z-index.
- **[design/COMPONENT-STANDARDS.md](./design/COMPONENT-STANDARDS.md)** — canonical components (shadcn primitives + shared components) and the contracts every page must follow (toast, loading, empty, destructive actions, icons, focus/motion).

Rules in short: consume semantic token aliases (no hex, no palette classes, no fallbacks), use shadcn `Button`/`Field`/`Dialog` (the legacy `.btn-*`/`.form-*` layer is removed), render feedback via the live-region toast, skeletons for any wait >150ms, shared `EmptyState` for zero states, two-stage confirmation for publish-triggering (HITL) actions.

## Developing

```sh
bun install
bun run dev -- --open
```

## Building

```sh
bun run build
bun run preview
```

Type checking:

```sh
npx svelte-check --threshold error
```
