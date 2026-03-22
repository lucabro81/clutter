# Clutter — Block 4A: Multi-Component Format & Vocabulary Architecture

**Version**: 0.1.0-draft
**Status**: Decided
**Date**: 2026-03-21

> Architecture decision record for the discussion held before Block 4 (Codegen).
> Supersedes the file format and vocabulary sections in `clutter-spec-compiler.md`,
> `clutter-block-01-lexer.md`, and `clutter-block-02-parser.md`.

---

## Context

Before implementing the code generator two structural questions had to be resolved:

1. Can a `.clutter` file contain more than one component?
2. How does the compiler manage the vocabulary of known components and their props?

Both questions affect every stage of the pipeline and cannot be deferred without
locking in assumptions that would be expensive to undo later.

---

## Decision 1 — Multi-component file format

### What changed

The previous format assumed a single unnamed component per file:

```
[TypeScript logic]

---

[template]
```

The new format wraps **every** component — including the root — in an explicit named
block. A file can contain N components, each delimited by its own curly braces:

```
component MainComponent(props: MainProps) {
    [TypeScript logic — opaque]
    ----
    [template]
}

component Card(props: CardProps) {
    [TypeScript logic — opaque]
    ----
    [template]
}

component Other(props: OtherProps) { ... }
```

No explicit inter-component separator is needed: the closing `}` of each block
unambiguously delimits it. The parser loops looking for `ComponentOpen` tokens.

### Separators

| Separator | Meaning | Previous |
|-----------|---------|----------|
| `----` (4 dashes) | Logic / template boundary inside a component block | `---` (3 dashes) |
| `component Name(...) { }` | Component block wrapper | *(did not exist)* |

The change from 3 to 4 dashes eliminates potential ambiguity with the `---` operator
in TypeScript or CSS contexts inside the logic block.

### Rationale

- Uniform syntax: the root component has no special treatment versus sub-components.
- Mirrors TypeScript's model of multiple named exports per file.
- `component Name(...) {` is a Clutter construct — the compiler controls it entirely;
  it is not TypeScript.
- Each component's TypeScript logic block remains isolated and opaque to the compiler.
- The `{ }` braces already delimit each block unambiguously — no additional separator
  token (`====` or similar) is needed.

---

## Decision 2 — VocabularyMap

### Problem: two structures, one concept

The analyzer currently maintains two independent structures for the same concept
("what components exist and what props do they accept"):

```rust
// Can diverge from prop_map silently — adding a component requires two edits
const KNOWN_COMPONENTS: &[&str] = &["Column", "Row", "Box", "Text", "Button", "Input"];

fn prop_map(component: &str, prop: &str) -> Option<PropValidation> {
    match (component, prop) { ... } // 30+ line hardcoded match
}
```

### Solution: single struct

`VocabularyMap` is the sole source of truth for the built-in vocabulary:

```rust
struct ComponentSchema {
    props: HashMap<&'static str, PropValidation>,
}

struct VocabularyMap {
    components: HashMap<&'static str, ComponentSchema>,
}

impl VocabularyMap {
    /// Replaces the KNOWN_COMPONENTS check (CLT103)
    fn contains(&self, name: &str) -> bool { ... }

    /// Replaces prop_map() (CLT101, CLT102)
    fn prop(&self, component: &str, prop: &str) -> Option<&PropValidation> { ... }
}
```

### Scope for the POC

- Built-in components (`Column`, `Row`, `Box`, `Text`, `Button`, `Input`) remain
  hardcoded Rust — no external file format is introduced.
- `VocabularyMap` is constructed once at the start of `analyze()`.
- The public signature of `analyze()` does not change for callers.
- This is a **pure internal refactor**: same errors emitted, same validation logic,
  no new features.

### Extension point

When custom component schemas or a file-based vocabulary become necessary, the
extension point is `VocabularyMap::new()` — the rest of the analyzer is unchanged.

---

## Decision 3 — `ComponentOpen` token (lexer)

`component Name(props_signature) {` is emitted as a **single lexer token**:

```rust
ComponentOpen { name: String, props_raw: String }
```

`props_raw` captures everything between `(` and `)` as a raw string. It is
TypeScript and is treated as opaque by the compiler — exactly like the logic block.

The closing `}` at the component level is a separate `ComponentClose` token.

---

## Decision 4 — Props signature: opaque for the POC

```
component Card(title: string, size: SpacingToken) { ... }
```

The type annotations are TypeScript. The compiler does not parse them.

Consequences:
- Custom component props are validated as `AnyValue` (CLT101/CLT102 are suppressed
  for custom components).
- A future typing system could extract prop types and cross-validate them against
  design tokens — this is a post-POC concern.
- Vue/TS runtime handles type-checking of component usage at runtime; building a
  parallel type system is explicitly out of scope.

---

## Decision 5 — Custom components in the POC

Components defined in the same `.clutter` file are **recognised** (no CLT103 error)
but their props are **not validated against the design system** (all `AnyValue`).

The `VocabularyMap` is the future extension point: when full prop validation for
custom components is needed, the compiler can populate the map with schemas derived
from the props signature or an external manifest.

Full prop validation for custom components is explicitly deferred.

---

## AST impact

```
Before                              After
──────────────────────────────────  ──────────────────────────────────────
ProgramNode                         FileNode
  logic_block: String                 components: Vec<ComponentDef>
  template: Vec<Node>
                                    ComponentDef
                                      name: String
                                      props_raw: String   ← opaque TS
                                      logic_block: String ← opaque TS
                                      template: Vec<Node>
```

`parse_program()` becomes `parse_file()`. Internal parsing functions
(`parse_nodes`, `parse_component`, `parse_props`, etc.) are unchanged.

---

## Pipeline impact summary

| Crate | Change |
|-------|--------|
| `clutter-lexer` | New tokens: `ComponentOpen { name, props_raw }`, `ComponentClose`. `SectionSeparator` changes from `---` to `----`. |
| `clutter-parser` | New root node: `FileNode { components: Vec<ComponentDef> }`. `parse_program` → `parse_file`. |
| `clutter-analyzer` | `analyze()` iterates over `Vec<ComponentDef>`. `VocabularyMap` replaces `KNOWN_COMPONENTS` + `prop_map`. Components in the same file are recognised (no CLT103). |
| `clutter-runtime` | AST types updated: `ProgramNode` → `FileNode`, add `ComponentDef`. |
| Fixtures | All `.clutter` fixtures rewritten in the new format. |

---

## What is explicitly deferred

| Item | Deferred to |
|------|-------------|
| Props signature parsing / TypeScript type extraction | Post-POC |
| File-based schema for built-in or custom components | Post-POC |
| Cross-file component imports | Post-POC |
| Full prop validation for custom components | Post-POC |
| Typing system coherent with Vue/TS | Long-term roadmap |

---

*End of Document*
