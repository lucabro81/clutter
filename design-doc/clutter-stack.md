# CLUTTER — Technology Stack

**Version**: 0.1.0-draft
**Status**: Exploration
**Author**: Luca

---

## Overview

The Clutter compiler is written in Rust. The choice is not aesthetic: a compiler is string manipulation, tree data structure transformation, and deterministic output production — exactly the domain where Rust excels. Clear ownership, zero garbage collector, native performance, and a mature library ecosystem for this class of problems.

---

## Project structure

Cargo workspace with separate crates for each functional block. Each crate is independently compilable, testable, and developable.

```
clutter/
├── Cargo.toml              ← workspace root
├── crates/
│   ├── clutter-lexer/      ← Lexer
│   ├── clutter-parser/     ← Parser + AST definition
│   ├── clutter-analyzer/   ← Semantic Analyzer
│   ├── clutter-codegen/    ← Code Generator + targets
│   ├── clutter-runtime/    ← runtime definitions (shared types)
│   └── clutter-cli/        ← CLI, binary entry point
├── tests/                  ← end-to-end integration tests
└── fixtures/               ← sample .clutter files for tests
```

### Crate dependency graph

```
clutter-cli
    ↓
clutter-codegen  ←  clutter-analyzer  ←  clutter-parser  ←  clutter-lexer
                                      ↑
                               clutter-runtime
                          (shared types: tokens, errors, positions)
```

`clutter-runtime` is not the output runtime — it is the crate that contains the types shared between blocks: design system token definitions, error structures, source positions. The name may be worth revisiting to avoid ambiguity with the "runtime" of the output discussed in the Block 5 document.

### TDD by default

Rust makes TDD natural: each crate has its own internal `tests` module, tests are written in the same file as the code they test, and `cargo test` runs them all in parallel with no additional configuration.

```rust
// Inside clutter-lexer/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenises_open_tag() {
        let tokens = lex("<Column gap=\"md\">");
        assert_eq!(tokens[0].kind, TokenKind::OpenTag);
        assert_eq!(tokens[0].value, "Column");
    }
}
```

Each block is developed test-first: you write the test that describes the expected behaviour, then the implementation that makes it pass. The integration tests in `tests/` exercise the complete pipeline with real `.clutter` files.

---

## AST representation: Arena Allocation

### The problem with trees in Rust

In a garbage-collected language (JS, Java, Go) an AST node can contain references to its children without issue — the GC handles memory management. In Rust, every value has a unique owner. A node containing other nodes, which in turn contain other nodes, creates recursive structures that the Rust compiler struggles to manage with direct pointers.

The naive solution — `Box<Node>` or `Rc<RefCell<Node>>` — works but is verbose, slow on large trees, and produces hard-to-read code.

### Arena allocation

An **arena** is a pre-allocated block of memory in which all AST nodes are allocated sequentially. Instead of pointers between nodes, **indices** are used — a "child" node is simply the number of its position in the arena.

```
Arena: [Node0, Node1, Node2, Node3, Node4, ...]
                                        ↑
         Node1.children = [2, 4]  ←  indices, not pointers
```

Advantages:
- **Performance**: all nodes are contiguous in memory, the processor cache reads them in sequence without jumps
- **Simple ownership**: the arena is the sole owner of all nodes — no lifetime problems or circular references
- **Bulk allocation and deallocation**: when compilation finishes, the entire arena is freed in one operation
- **Standard in compilers**: this is the approach used by `rustc` itself, by `rust-analyzer`, and by all serious Rust compilers

### Reference crate

For the POC, **`typed-arena`** is used — a simple, stable, well-documented typed arena.

For a more mature implementation (post-POC), **`rowan`** is the library used by `rust-analyzer` and most Rust language servers. It offers a typed node system, immutable green trees, and native support for incremental operations (useful when the LSP arrives).

### Further reading

- [Rust Book — Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html) — prerequisite for understanding why arena allocation solves a real problem
- [typed-arena](https://docs.rs/typed-arena) — crate used in the POC
- [rowan](https://github.com/rust-analyzer/rowan) — crate used by rust-analyzer, reference for mature implementation
- [Rustc Dev Guide — The HIR](https://rustc-dev-guide.rust-lang.org/hir.html) — how the Rust compiler itself manages its internal AST

---

## tokens.clutter format

The design system is defined in a `tokens.clutter` file in **JSON** format.

### Rationale

- Universal — all tools read and write it
- Figma can export JSON directly, with plugins or via API
- Migration scripts from existing design systems are trivial to write
- Excellent Rust support via `serde_json`

### Structure

```json
{
  "spacing": {
    "xs":  4,
    "sm":  8,
    "md":  16,
    "lg":  24,
    "xl":  32,
    "xxl": 48
  },
  "colors": {
    "primary":    "#007AFF",
    "secondary":  "#5856D6",
    "danger":     "#FF3B30",
    "surface":    "#F2F2F7",
    "background": "#FFFFFF",
    "text": {
      "primary":   "#000000",
      "secondary": "#3C3C43",
      "tertiary":  "#8E8E93"
    }
  },
  "typography": {
    "sizes":   { "xs": 12, "sm": 14, "base": 16, "lg": 18, "xl": 24, "xxl": 32 },
    "weights": { "normal": 400, "medium": 500, "semibold": 600, "bold": 700 },
    "lineHeights": { "tight": 1.2, "normal": 1.5, "relaxed": 1.75 }
  },
  "radii": {
    "none": 0, "sm": 4, "md": 8, "lg": 16, "full": 9999
  },
  "shadows": {
    "sm": "0 1px 2px 0 rgb(0 0 0 / 0.05)",
    "md": "0 4px 6px -1px rgb(0 0 0 / 0.1)",
    "lg": "0 10px 15px -3px rgb(0 0 0 / 0.1)"
  },
  "breakpoints": {
    "mobile": 640, "tablet": 768, "desktop": 1024, "wide": 1280
  }
}
```

The Semantic Analyzer loads this file at startup and builds the internal `prop → valid values` map. If the file is malformed or missing, the CLI produces an explicit error before attempting any compilation.

---

## unsafe in the template

### Motivation

The rigidity of the closed vocabulary is Clutter's primary value — but imposing absolute rigidity without an escape hatch is a choice that slows adoption. Third-party component integrations, design system edge cases, legacy code to wrap: real situations that exist in any project.

The solution is not to soften the rules — it is to make exceptions **explicit, visible, and documented**.

`unsafe` in the Clutter template works like `unsafe` in Rust: it is not an error, it is a conscious declaration that you are stepping outside the system's guarantees. The code compiles, but the developer — and anyone doing code review — immediately sees that the rules do not apply there, and why.

### Two forms of unsafe

**Unsafe block** — for arbitrary markup outside the Clutter vocabulary.

The `<unsafe>` tag requires a mandatory `reason` attribute. If it is missing, compilation error — not a warning, an error. An `unsafe` without an explanation is worse than no `unsafe`: it gives false security without leaving a trace of the debt.

```
<Column gap="md">
  <Text size="lg">Normal content</Text>

  <unsafe reason="the third-party DatePicker component does not yet have a Clutter wrapper">
    <div class="legacy-datepicker">
      ...
    </div>
  </unsafe>

  <Button variant="primary">OK</Button>
</Column>
```

**Unsafe value** — for custom values on props that expect design system tokens.

The `unsafe()` function accepts the custom value as the first argument and a mandatory comment as the second. If the second argument is missing, compilation error.

```
<Column gap={unsafe('16px', 'non-standard spacing required by the print layout, fix with token print-spacing in v2')}>
  ...
</Column>
```

### Compiler behaviour

- The Semantic Analyzer ignores the content of `<unsafe>` and `unsafe()` values — it does not validate or check them
- The Semantic Analyzer **does verify** that `reason` and the second argument of `unsafe()` are present and non-empty — compilation error otherwise
- The Code Generator copies the content of `<unsafe>` verbatim into the output, and inserts the `unsafe()` value directly into the prop
- The CLI reports all unsafe blocks with their comments:

```
✓ Card.clutter → Card.vue (12ms)

  2 unsafe blocks:
  - line 8  [tag]   "the DatePicker component does not yet have a Clutter wrapper"
  - line 23 [value] "non-standard spacing for print layout, fix with token print-spacing in v2"
```

- In the future: a `--no-unsafe` flag that fails compilation if `<unsafe>` blocks or `unsafe()` values are present, useful for CI/CD on production branches

### unsafe in the logic section

The logic section is already arbitrary TypeScript — by definition it has none of the template's restrictions. No `unsafe` keyword is needed there: the developer writes whatever they want, and the Code Generator inserts it unchanged into the output.

### Selling point

`unsafe` transforms technical debt from implicit to explicit and documented. In a Clutter codebase:

- `grep unsafe` shows exactly where and how many times the design system was bypassed
- Each occurrence carries with it the explanation of why and — ideally — what is needed to resolve it
- The CLI report at every build keeps the team aware of existing debt

This is a structural advantage over Tailwind or plain CSS, where exceptions hide silently among hundreds of classes or in scattered `.css` files without leaving a trace.

---

## Rust dependencies

| Crate | Version | Use |
|---|---|---|
| `clap` | 4.x | CLI — argument parsing |
| `miette` | 5.x | Error reporting with source highlighting |
| `serde` + `serde_json` | 1.x | `tokens.clutter` deserialisation |
| `typed-arena` | 2.x | Arena allocation for the AST |
| `rowan` | 0.15.x | Post-POC — AST for the LSP |

No dependencies for the Lexer, Parser, Semantic Analyzer, and Code Generator — they are pure algorithms on data structures, no external libraries needed.

---

## Output dependencies (non-Rust)

| Dependency | Target | Use |
|---|---|---|
| Alpine.js | HTML (POC) | Reactive runtime |
| `@vue/reactivity` | HTML (at scale) | Proprietary reactive runtime |
| Vue / Nuxt | Vue SFC | Host application runtime |

---

## References

- [The Rust Programming Language](https://doc.rust-lang.org/book/) — primary reference
- [Rust Book — Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) — multi-crate structure
- [clap](https://docs.rs/clap) — CLI
- [miette](https://docs.rs/miette) — error reporting
- [serde_json](https://docs.rs/serde_json) — JSON parsing
- [typed-arena](https://docs.rs/typed-arena) — arena allocation POC
- [rowan](https://github.com/rust-analyzer/rowan) — arena + AST for LSP
- [Crafting Interpreters](https://craftinginterpreters.com) — general compiler reference

---

*End of Document*
