# Titen Web — Design Token Spec

> Single source of truth for colors, spacing, type, radii, shadows, motion, and z-index.
> Implementation: `src/app.css` (`@theme` block). The system is **Hallmark / Cobalt** — OKLCH, anchor hue **260**.

## Layer Map

```
PRIMITIVES (--color-paper, --color-rule, --color-ink, --color-accent, …)
    │   raw material — do NOT use directly in components
    ▼
SEMANTIC ALIASES (--color-bg, --color-border, --color-danger, --color-primary, …)
    │   the API pages/components consume
    ▼
SHADCN MAPPING (--background, --primary, --destructive, --ring, …)
    │   feeds ui/* primitives — keep in sync with aliases
    ▼
COMPONENT TOKENS (--form-*, --table-*, --sidebar-*, --z-*, --dur-*)
    tuned per component family
```

## OKLCH Recipe

- **Anchor hue: 260** for all neutral surfaces, ink, and accent.
- **Semantic hues:** success `145`, warning `85`, error `25`, info `260`.
- **Text/ink chroma ≤ 0.008** (near-neutral, tinted by anchor hue). Accent carries the saturation (`0.20`).
- **Dim variants** (`-dim`) are high-lightness low-chroma washes for badge/soft backgrounds; `-ink` variants are their foreground pairs.

## Rules

1. **No hex fallbacks.** `var(--token, #fff)` is banned — if the token is missing, fix the token layer, not the call site. (`grep -rn "var(--[a-zA-Z-]*, *#" src` must return 0.)
2. **No raw hex / Tailwind palette classes in markup** (`text-green-600`, `bg-white`, …). Use semantic aliases.
3. **Pages consume semantic aliases** (or shadcn utilities mapped to them). Primitives are only for composing new aliases/tokens.
4. **New tokens get defined in `@theme` first**, then consumed. No phantom `var()` references.
5. Full pills use **`--radius-pill`** (there is deliberately no `--radius-full`).

## Token Inventory (see `app.css @theme` for values)

| Family | Tokens |
|---|---|
| Surfaces | `--color-paper(-2/-3)`, `--surface-base/raised/sunken/overlay` |
| Text | `--color-ink(-2)`, `--color-muted`, `--color-neutral` |
| Rules/borders | `--color-rule(-2)`, `--rule-default/subtle/strong`, `--color-border(-hover)` |
| Accent | `--color-accent(-dim/-ink)`, `--color-focus`, `--color-accent-subtle` |
| Status | `--color-success/warning/error/info` + `-dim` + `-ink`, `--color-*-bg`, `--color-bg-subtle` |
| Radii | `--radius-2xs → --radius-pill` |
| Spacing | `--space-3xs → --space-3xl` (4pt base) |
| Type | `--font-display/body/mono`, `--text-2xs → --text-3xl` (1.25 ratio) |
| Shadows | `--shadow-whisper/raised/overlay` |
| Motion | `--dur-short/base/long`, `--ease-out/in` |
| Z-index | `--z-base → --z-tooltip` (6 named levels) |
| Components | `--form-*`, `--table-*`, `--sidebar-*` |

## Shadcn ↔ Token Mapping

| shadcn name | Titen token |
|---|---|
| `--background` / `--foreground` | `--color-paper` / `--color-ink` |
| `--card` | `--color-paper-2` |
| `--popover` | `--color-paper` |
| `--primary` / `--primary-foreground` | `--color-accent` / `--color-accent-ink` |
| `--secondary` | `--color-paper-3` |
| `--muted` / `--muted-foreground` | `--color-paper-3` / `--color-muted` |
| `--accent` | `--color-accent-dim` |
| `--destructive(-foreground)` | `--color-error(-ink)` |
| `--input` / `--border` | `--color-rule` |
| `--ring` | `--color-focus` |

Adding a new shadcn primitive requires BOTH the `:root` mapping and a `--color-<name>` entry in `@theme` (Tailwind v4 only emits utilities for `@theme` names).
