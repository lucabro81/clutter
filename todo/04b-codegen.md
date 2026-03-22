# Block 4B: Codegen — Vue SFC Target

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## Context

The code generator is the final stage of the compilation pipeline. It receives a
validated `FileNode` from the analyzer and produces one Vue SFC (`.vue` file) per
`ComponentDef`. The HTML/Alpine.js target is deferred — see `todo/00-backlog.md`.

Output type: `Vec<GeneratedFile { name: String, content: String }>`. The CLI writes
each entry to `{name}.vue`.

Pipeline position:

```
.clutter → Lexer → Parser → Analyzer → **Codegen** → .vue files
```

### Node → HTML mapping (Vue target)

| Clutter component | Vue template HTML        |
|-------------------|--------------------------|
| `Column`          | `<div class="clutter-column …">` |
| `Row`             | `<div class="clutter-row …">` |
| `Box`             | `<div class="clutter-box …">` |
| `Text`            | `<p class="clutter-text …">` |
| `Button`          | `<button class="clutter-button …">` |
| `Input`           | `<input class="clutter-input …" />` |
| custom component  | `<ComponentName prop="val" />` (passed through as-is) |

### Prop → output mapping

| Prop value kind   | Vue output                         |
|-------------------|------------------------------------|
| `StringValue`     | CSS class: `clutter-{prop}-{val}`  |
| `ExpressionValue` | Vue binding: `:{prop}="{expr}"`    |
| `UnsafeValue`     | raw string value, no CSS class     |

Special case — `Text value` prop:
- `value="Hello"` → element text content: `<p …>Hello</p>`
- `value={expr}` → Vue interpolation: `<p …>{{ expr }}</p>`

### `IfNode` / `EachNode` children

- Single child → `v-if` / `v-for` placed directly on that element.
- Multiple children → wrapped in `<template v-if="…">` / `<template v-for="…">`.

### `UnsafeNode`

Transparent in output: children are rendered normally. `<unsafe>` is a compiler
concept only. `UnsafeValue` props output the raw value string.

### CSS generation

One CSS class per token value, included in every SFC's `<style scoped>` block.

```css
.clutter-column  { display: flex; flex-direction: column; }
.clutter-row     { display: flex; flex-direction: row; }
.clutter-box     { box-sizing: border-box; }
.clutter-text    { }
.clutter-button  { cursor: pointer; }
.clutter-input   { }

.clutter-gap-xs  { gap: 4px; }   /* one per spacing token */
.clutter-gap-sm  { gap: 8px; }
/* … */
.clutter-bg-primary   { background-color: var(--color-primary); }
/* … */
```

Actual pixel/colour values come from `DesignTokens`. For the POC, the CSS uses
CSS custom properties named after the token values; resolving those to real values
is a CLI/runtime concern deferred to Block 5.

---

## clutter-codegen — module setup

- [ ] Remove placeholder `add` function and stub test from `lib.rs`
- [ ] Add `mod vue;` and `mod css;` submodules
- [ ] Define `pub struct GeneratedFile { pub name: String, pub content: String }` in `lib.rs`
- [ ] Define `pub fn generate_vue(file: &FileNode, tokens: &DesignTokens) -> Vec<GeneratedFile>` public entry point
- [ ] Add `clutter-runtime` and `clutter-analyzer` (for `DesignTokens`) to `Cargo.toml` dependencies

---

## clutter-codegen — CSS generation: tests (written BEFORE implementation)

- [ ] `generate_css` output contains `.clutter-column { display: flex; flex-direction: column; }`
- [ ] Output contains `.clutter-row { display: flex; flex-direction: row; }`
- [ ] Output contains one `.clutter-gap-{val}` class per spacing token value
- [ ] Output contains one `.clutter-bg-{val}` class per color token value
- [ ] Output contains `.clutter-size-{val}` for each font size token
- [ ] Output contains `.clutter-weight-{val}` for each font weight token
- [ ] Output contains `.clutter-radius-{val}` for each radius token
- [ ] Output contains `.clutter-shadow-{val}` for each shadow token

---

## clutter-codegen — CSS generation: implementation

- [ ] `css::generate_css(tokens: &DesignTokens) -> String`
- [ ] Base component classes (Column, Row, Box, Text, Button, Input)
- [ ] Token value classes for all six categories (spacing, color, font-size, font-weight, radius, shadow)

---

## clutter-codegen — Vue target: tests (written BEFORE implementation)

All tests construct the AST directly — no lexer/parser round-trip needed.

### Template node generation

- [ ] `ComponentNode` Column, no props → `<div class="clutter-column">`
- [ ] `ComponentNode` Column, `gap="md"` → `<div class="clutter-column clutter-gap-md">`
- [ ] `ComponentNode` Column, `gap={size}` (expression) → `<div :gap="size" class="clutter-column">`
- [ ] `ComponentNode` Text, `value="Hello"` → `<p class="clutter-text">Hello</p>`
- [ ] `ComponentNode` Text, `value={title}` → `<p class="clutter-text">{{ title }}</p>`
- [ ] `ComponentNode` Button, `variant="primary"`, `disabled={loading}` → correct classes + binding
- [ ] `ComponentNode` Input → `<input … />` (self-closing)
- [ ] `ComponentNode` unknown/custom name → passed through as `<Name prop="val" />`
- [ ] `TextNode` → plain text verbatim
- [ ] `ExpressionNode` → `{{ expr }}`
- [ ] Nesting two levels deep → correct 2-space indentation per level
- [ ] `IfNode`, single child, no else → `v-if` attribute on child element
- [ ] `IfNode`, single child, with else → `v-if` on then-child, `v-else` on else-child
- [ ] `IfNode`, multiple then-children → `<template v-if="…">` wrapper
- [ ] `EachNode`, single child → `v-for="alias in collection" :key="alias"` on child
- [ ] `EachNode`, multiple children → `<template v-for="…" :key="…">` wrapper
- [ ] `UnsafeNode` → children rendered normally, no wrapper emitted
- [ ] `UnsafeValue` prop → raw string value, no CSS class generated

### Full SFC generation

- [ ] `ComponentDef` with empty template → valid SFC with empty `<template>` block
- [ ] `ComponentDef` with logic block → logic block verbatim in `<script setup lang="ts">`
- [ ] `ComponentDef` with empty logic block → `<script setup lang="ts">` block still present but empty
- [ ] `<style scoped>` block present and non-empty in every generated SFC
- [ ] `FileNode` with one component → `Vec` with one `GeneratedFile`, name matches component name
- [ ] `FileNode` with two components → two `GeneratedFile`s, independent content

---

## clutter-codegen — Vue target: implementation

- [ ] `vue::generate_sfc(comp: &ComponentDef, tokens: &DesignTokens) -> String`
- [ ] `vue::generate_template(nodes: &[Node], depth: usize) -> String`
- [ ] `vue::generate_node(node: &Node, depth: usize) -> String`
- [ ] `vue::generate_component_node(node: &ComponentNode, depth: usize) -> String`
  - built-in → mapped HTML element with CSS classes
  - custom → passed through as `<Name …>`
- [ ] `vue::generate_props(props: &[PropNode]) -> (String, String)` → `(class_attr, bindings)`
- [ ] `vue::generate_if(node: &IfNode, depth: usize) -> String`
- [ ] `vue::generate_each(node: &EachNode, depth: usize) -> String`
- [ ] `vue::generate_unsafe(node: &UnsafeNode, depth: usize) -> String`
- [ ] Wire everything through `generate_vue(file, tokens)` in `lib.rs`

---

## Integration tests

- [ ] `valid.clutter` fixture → generates valid `.vue` file (contains `<template>`, `<script setup>`, `<style scoped>`)
- [ ] `logic_block.clutter` fixture → logic block appears verbatim in `<script setup>`
- [ ] `if_else.clutter` fixture → output contains `v-if` and `v-else`
- [ ] `nesting.clutter` fixture → output is correctly indented

---

## Final check

- [ ] `cargo test` — full workspace green
- [ ] `cargo build --workspace` — zero warnings
- [ ] Mark `Block 4: Codegen` row in `CLAUDE.md` status table as ✅ complete
- [ ] Update `todo/00-backlog.md` if any items were resolved incidentally
