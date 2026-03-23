# Clutter

> **This is a proof of concept.** The compiler is functional end-to-end but the CLI, error reporting, and output targets are still in development. Not ready for production use.

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

A `.clutter` file wraps each component in a `component Name(...) { }` block. Logic and template are separated by `----` (four dashes):

```
component MainComponent(props: MainProps) {
const title = "Hello"
const handleClick = () => console.log("clicked")
----
<Column gap="md" padding="lg">
  <Text size="xl" weight="bold" value={title} />
  <Button variant="primary">Click</Button>
</Column>
}
```

- **Logic section** — standard TypeScript; the compiler treats it as an opaque block
- **`----`** — required separator (4 dashes), even if the logic section is empty
- **Template section** — JSX-like syntax with a closed vocabulary of built-in components
- A file can define multiple components; each is emitted as a separate output file

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
  <Text value="Welcome" />
<else>
  <Button variant="primary" value="Log in" />
</else>
</if>

<each collection={products} as="product">
  <Text value={product} />
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
  "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
  "colors":     ["primary", "secondary", "danger", "surface", "background"],
  "typography": {
    "sizes":   ["xs", "sm", "base", "lg", "xl", "xxl"],
    "weights": ["normal", "medium", "semibold", "bold"]
  },
  "radii":   ["none", "sm", "md", "lg", "full"],
  "shadows": ["sm", "md", "lg"]
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
├── fixtures/           — sample .clutter files used by integration tests
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

Unit tests live in `src/tests.rs` per crate; integration tests in `tests/` use real `.clutter` files from `fixtures/`.
