# Architecture

Clutter is a Rust compiler for `.clutter`, a UI markup language with a closed vocabulary. This document explains the key architectural decisions made during the POC.

---

## Table of Contents

- [Pipeline](#pipeline)
- [Key decisions](#key-decisions)
  - [Closed vocabulary as a type system](#closed-vocabulary-as-a-type-system)
  - [Multi-component files](#multi-component-files)
  - [VocabularyMap — single source of truth](#vocabularymap--single-source-of-truth)
  - [Global CSS, no scoped styles](#global-css-no-scoped-styles)
  - [CSS variables in tokens.json](#css-variables-in-tokensjson)
  - [No runtime for Vue SFC](#no-runtime-for-vue-sfc)
  - [Unsafe as explicit debt](#unsafe-as-explicit-debt)
  - [Arena allocation for the AST](#arena-allocation-for-the-ast)
  - [Token auto-discovery](#token-auto-discovery)
  - [Distribution](#distribution)

---

## Pipeline

```
.clutter → Lexer → Parser → Analyzer → Codegen → Output
                                 ↑
                           tokens.json
```

Each stage is a separate crate, always passing read-only data to the next:

| Crate | Role |
|-------|------|
| `clutter-runtime` | Shared types: AST nodes, tokens, error types, `DesignTokens` |
| `clutter-lexer` | `String` → `Vec<Token>` |
| `clutter-parser` | `Vec<Token>` → AST (`FileNode`) |
| `clutter-analyzer` | Validates every prop value against `tokens.json` |
| `clutter-codegen` | AST → Vue SFC files + `clutter.css` |
| `clutter-cli` | Orchestrates the full pipeline; the `clutter` binary |

---

## Key decisions

### Closed vocabulary as a type system

The core idea: design tokens are not documentation — they are the type system. Every prop value in a template is validated against `tokens.json` at compile time. If a value is not in the token set, the build fails. This makes design system violations impossible to ship, not just unlikely.

The consequence is a **closed vocabulary**: only built-in components with known prop schemas are allowed. Custom HTML is not. This is deliberate — the compiler cannot validate arbitrary markup.

### Multi-component files

A single `.clutter` file can contain multiple component definitions. Each is emitted as a separate `.vue` file. The format:

```
component Card(props: CardProps) {
  [logic]
  ----
  [template]
}

component Feed(props: FeedProps) {
  ----
  [template]
}
```

The `----` separator (four dashes) divides the TypeScript logic block from the template. The logic block is treated as opaque — the compiler copies it verbatim into `<script setup>`.

This was introduced mid-development (replacing a single-component-per-file format) because related components belong together in source, the same way related functions belong in the same module.

### VocabularyMap — single source of truth

All component schemas and prop validation rules live in one place: `VocabularyMap` in `clutter-analyzer`. When adding or modifying a component, there is one file to change. The rest of the analyzer is unchanged.

Custom components defined within a `.clutter` file are recognized (so that `<Card />` doesn't produce "unknown component") but their props are not validated — that is deferred to post-POC, when a cross-file analysis pass is needed.

### Global CSS, no scoped styles

The original design had each generated Vue SFC include a `<style scoped>` block with all token utility classes. This was discarded:

- Every SFC had the same ~60 rules, duplicated N times for N components
- Scoped styles with `[data-v-xxx]` selectors add specificity noise
- A closed vocabulary means no component-specific CSS exists by definition — every rule is a utility class

The decision: one global `clutter.css`, generated alongside the Vue files, containing a `:root { }` block (CSS custom properties) followed by utility classes. This is the Tailwind pattern. Import it once in the app entry point.

### CSS variables in `tokens.json`

The utility classes reference CSS custom properties (`var(--spacing-md)`, etc.) rather than hardcoded values. The actual values are defined in an optional `"variables"` key in `tokens.json`:

```json
"variables": {
  "--spacing-md": "1rem",
  "--color-primary": "#3b82f6"
}
```

This keeps designers in control of the actual values without touching the compiler. The naming convention `--{category}-{value}` is implicit, not enforced — the compiler emits whatever names and values it finds.

### No runtime for Vue SFC

The Vue SFC target requires no Clutter runtime at all. The output is standard Vue — `<template>`, `<script setup>`, and class names that reference globally imported CSS. The Vue runtime is the runtime.

### Unsafe as explicit debt

`<unsafe>` exists for content that can't be expressed in the closed vocabulary (legacy third-party components, edge cases). It requires a mandatory `reason` attribute:

```
<unsafe reason="DatePicker has no Clutter wrapper yet">
  ...
</unsafe>
```

The compiler rejects `<unsafe>` without a reason. The intent is to make technical debt explicit and searchable — every use of `<unsafe>` is a documented exception, not a silent escape.

### Arena allocation for the AST

The parser uses `typed-arena` to allocate AST nodes. All nodes live in an arena owned by the parse call; references are valid for the lifetime of the arena. This avoids `Box<dyn Node>` overhead and keeps the AST layout cache-friendly.

### Token auto-discovery

The CLI walks up the directory tree from the source file looking for `tokens.json`. An explicit `--tokens` flag overrides this. The pattern matches how tools like ESLint and Prettier discover their config files.

### Distribution

The compiler is distributed as a pre-built binary via GitHub Releases. A `setup.sh` (generated by CI with the version baked in) handles everything in one command: platform detection, binary download, project scaffolding, `.clutter` compilation, and `npm install`. No Rust toolchain required.

Two targets are built on every tag push: `aarch64-apple-darwin` (macOS Apple Silicon) and `x86_64-unknown-linux-gnu` (Linux).
