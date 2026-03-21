# Clutter

A Rust compiler for `.clutter`, a UI markup language with a closed vocabulary that enforces design system compliance at compile time. Write structure and logic — the compiler handles styling.

---

## Table of Contents

- [Overview](#overview)
- [How it works](#how-it-works)
- [File format](#file-format)
- [Design tokens](#design-tokens)
- [Stack](#stack)
- [Project structure](#project-structure)
- [CLI](#cli)
- [Development](#development)

---

## Overview

Clutter solves a specific problem: CSS gives developers too much freedom. Arbitrary values, no compile-time enforcement, design system violations caught only in review — or never.

Clutter replaces that with a **closed vocabulary**. Components accept only props that map to design tokens. If a value is not in the design system, it does not compile. The type system *is* the design system.

```
<Column gap="md" padding="lg">
  <Text size="xl" weight="bold" color="primary">Hello</Text>
  <Button variant="primary">Click</Button>
</Column>
```

No CSS. No class names to remember. No design system violations possible.

---

## How it works

```
.clutter file → Lexer → Parser → Semantic Analyzer → Code Generator → Output
                                         ↑
                                   tokens.json
```

1. **Lexer** — tokenizes the source file
2. **Parser** — builds an AST from the token stream
3. **Semantic Analyzer** — validates every prop value against `tokens.json`; produces typed error messages if anything is invalid
4. **Code Generator** — walks the validated AST and emits the target output

Output targets: **Vue SFC** (`.vue`) and **static HTML**.

---

## File format

A `.clutter` file has two sections separated by `---`:

```
const title = "Hello"
const handleClick = () => console.log("clicked")

---

<Column gap="md" padding="lg">
  <Text size="xl" weight="bold">{title}</Text>
  <Button variant="primary" onClick={handleClick}>Click</Button>
</Column>
```

- **Logic section** — standard TypeScript; the compiler treats it as an opaque block
- **`---`** — required separator, even if the logic section is empty
- **Template section** — JSX-like syntax with a closed vocabulary of built-in components

### Template rules

- Props accept only values present in `tokens.json`: `gap="md"` ✓ · `gap="17px"` ✗
- Variable references from the logic section: `{title}`, `{handleClick}`
- No inline expressions — compute values in the logic section, reference them in the template
- Boolean shorthand: `disabled` equals `disabled={true}`

### Built-in components

| Component | Purpose |
|-----------|---------|
| `Column`  | Flex column layout |
| `Row`     | Flex row layout |
| `Box`     | Generic container |
| `Text`    | Typographic element |
| `Button`  | Interactive action |
| `Input`   | Text input field |

### Control flow

```
<if condition={isLoggedIn}>
  <Text>Welcome</Text>
</if>
<else>
  <Button variant="primary">Log in</Button>
</else>

<each item={products} as="product">
  <Text>{product.name}</Text>
</each>
```

### Escape hatch

For legacy integrations or edge cases, `<unsafe>` exits the closed vocabulary. A `reason` attribute is required — the compiler rejects unsafe blocks without one.

```
<unsafe reason="third-party DatePicker, no Clutter wrapper yet">
  <div class="legacy-datepicker">...</div>
</unsafe>
```

---

## Design tokens

`tokens.json` is the single source of truth for the design system. It is a JSON file placed at the project root.

```json
{
  "spacing":    { "xs": 4, "sm": 8, "md": 16, "lg": 24, "xl": 32, "xxl": 48 },
  "colors":     { "primary": "#007AFF", "secondary": "#5856D6", "danger": "#FF3B30", "surface": "#F2F2F7", "background": "#FFFFFF" },
  "typography": {
    "sizes":      { "xs": 12, "sm": 14, "base": 16, "lg": 18, "xl": 24, "xxl": 32 },
    "weights":    { "normal": 400, "medium": 500, "semibold": 600, "bold": 700 },
    "lineHeights":{ "tight": 1.2, "normal": 1.5, "relaxed": 1.75 }
  },
  "radii":   { "none": 0, "sm": 4, "md": 8, "lg": 16, "full": 9999 },
  "shadows": { "sm": "0 1px 2px 0 rgb(0 0 0 / 0.05)", "md": "0 4px 6px -1px rgb(0 0 0 / 0.1)", "lg": "0 10px 15px -3px rgb(0 0 0 / 0.1)" }
}
```

When a prop value is not in the token set, the compiler produces a typed error:

```
error[CLT102] — line 4, column 12
  Invalid value 'xl2' for prop 'gap' on 'Column'.
  Valid values: xs, sm, md, lg, xl, xxl

  4 │ <Column gap="xl2">
                   ^^^
```

---

## Stack

**Compiler** — Rust

| Crate | Role |
|-------|------|
| `clutter-runtime` | Shared types: `Token`, `Position`, AST nodes, error types |
| `clutter-lexer`   | Tokenizer: `String` → `Vec<Token>` |
| `clutter-parser`  | Parser: `Vec<Token>` → AST (arena-allocated with `typed-arena`) |
| `clutter-analyzer`| Semantic analyzer: validates props against `tokens.json` |
| `clutter-codegen` | Code generator: AST → Vue SFC or HTML string |
| `clutter-cli`     | CLI binary: orchestrates the full pipeline |

**External dependencies**

| Crate | Version | Use |
|-------|---------|-----|
| `clap` | 4 | CLI argument parsing |
| `miette` | 5 | Error reporting with source highlighting |
| `serde` + `serde_json` | 1 | `tokens.json` deserialization |
| `typed-arena` | 2 | Arena allocation for AST nodes |

---

## Project structure

```
clutter/
├── Cargo.toml          — workspace root
├── crates/
│   ├── clutter-runtime/
│   ├── clutter-lexer/
│   ├── clutter-parser/
│   ├── clutter-analyzer/
│   ├── clutter-codegen/
│   └── clutter-cli/
├── tests/              — end-to-end integration tests
├── fixtures/           — sample .clutter files used by tests
└── todo/               — block-by-block development checklist
```

---

## CLI

```
clutter build <file> [--target <vue|html>] [--out <dir>]
```

| Argument | Required | Default | Description |
|----------|----------|---------|-------------|
| `<file>` | yes | — | Path to the `.clutter` file to compile |
| `--target` | no | `vue` | Output target: `vue` or `html` |
| `--out` | no | source directory | Output directory |

**Examples**

```bash
# Compile to Vue SFC (default)
clutter build src/components/Card.clutter

# Compile to static HTML
clutter build src/components/Card.clutter --target html

# Write output to a specific directory
clutter build src/components/Card.clutter --out dist/
```

`tokens.json` is discovered automatically by walking up the directory tree from the source file — no explicit path needed.

**Exit codes**: `0` on success, `1` on any error.

---

## Development

```bash
cargo check                   # type check the workspace
cargo test                    # run all tests
cargo test -p clutter-lexer   # run tests for a single crate
cargo build --release         # build the clutter binary
```

All development follows TDD: tests are written before implementation. Unit tests live in `src/tests.rs` per crate; integration tests in `tests/` use real `.clutter` files from `fixtures/`.
