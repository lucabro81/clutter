# CLUTTER — Compiler Approach Specification

**Version**: 0.1.0-draft
**Status**: Exploration
**Author**: Luca

---

## Executive Summary

Clutter is a UI markup language with a dedicated compiler. `.clutter` files are transpiled to a configurable target (Vue SFC, vanilla JS/HTML, others). The developer does not write CSS, does not configure toolchains, does not manage ecosystem dependencies — they write structure and logic, the compiler does the rest.

End goal: an ideal format for human and LLM-first development, with a closed vocabulary and zero ambiguity.

---

## Table of Contents

1. [Rationale](#rationale)
2. [File Format](#file-format)
3. [Template Syntax](#template-syntax)
4. [Type System & Design Tokens](#type-system--design-tokens)
5. [Component Model](#component-model)
6. [Compiler Architecture](#compiler-architecture)
7. [Runtime](#runtime)
8. [Compilation Targets](#compilation-targets)
9. [Unsafe Escape Hatch](#unsafe-escape-hatch)
10. [Tooling](#tooling)
11. [Known Limitations & Tradeoffs](#known-limitations--tradeoffs)
12. [Future Extensions](#future-extensions)

---

## Rationale

### Why not a Vue library

The "Vue components with typed props" approach solves the manual CSS problem, but leaves everything else intact:

- The developer is still inside the Vue ecosystem with its rules
- TypeScript is mandatory and brings with it tsconfig, strict mode, versioning
- Errors are TypeScript errors, not Clutter errors
- Adding features (accessibility plugin, design-by-testing, etc.) requires workarounds on the existing ecosystem
- The LLM must know Vue, TypeScript, Vite, and Clutter — too broad a vocabulary

### Why a custom compiler

- **Semantic errors**: "colour 'blue' does not exist in the design system" instead of generic type errors
- **Closed vocabulary**: the template only accepts Clutter constructs — nothing to invent, nothing to forget
- **Native plugin architecture**: accessibility, test ids, linting are compiler hooks, not ecosystem patches
- **LLM-first**: predictable format, fixed structure, zero implicit configuration — trainable with very few examples
- **Multiple targets**: the same `.clutter` can compile to Vue for existing apps or vanilla for new projects

### What you lose

- Immediate compatibility with Vue tooling (Volar, Vue DevTools, etc.)
- Ecosystem of third-party Vue components
- Team familiarity with the format

These are real costs, not ignorable — see the [Known Limitations](#known-limitations--tradeoffs) section.

---

## File Format

> ⚠️ Superseded by [`clutter-block4a.md`](clutter-block4a.md). Content below kept for historical reference.

> A `.clutter` file is composed of two sections separated by an explicit delimiter.
>
> ```
> [logic section — standard TypeScript]
>
> ---
>
> [template section — JSX-like syntax with closed vocabulary]
> ```
>
> ### Rules
>
> - The logic section is valid TypeScript, no custom syntax
> - The `---` separator is mandatory even if the logic section is empty
> - The template section does not accept arbitrary TypeScript — only references to variables defined in the logic section
> - A `.clutter` file defines exactly one root component
>
> ### Minimal example
>
> ```
> const title = "Hello"
> const handleClick = () => console.log("clicked")
>
> ---
>
> <Column gap="md" padding="lg">
>   <Text size="xl" weight="bold">{title}</Text>
>   <Button variant="primary" onClick={handleClick}>Click</Button>
> </Column>
> ```

---

## Template Syntax

### Principles

- JSX-like syntax: tags, props, children
- Only built-in components or explicitly imported `.clutter` components
- Props only accept values from the design system or references to variables from the logic section
- No arbitrary JS expressions in the template — complex expressions must be calculated in the logic section and passed as variables

### Built-in components

Available without import, part of the language:

| Component | Purpose |
|---|---|
| `Column` | Flex column |
| `Row` | Flex row |
| `Box` | Generic container |
| `Text` | Typographic text |
| `Button` | Interactive action |
| `Input` | Input field |
| `Image` | Image with token-based dimensions |

### Props

Props accept:
- Literal values from the design system: `gap="md"`, `color="primary"`
- Variable references from the logic section: `{myVariable}`
- Boolean shorthand: `disabled` is equivalent to `disabled={true}`

Props **do not** accept:
- Arbitrary values not present in the tokens: `gap="17px"` → compilation error
- Inline JS expressions: `gap={isLarge ? "lg" : "sm"}` → must be calculated in the logic section

### Conditional rendering and lists

```
// Conditional — dedicated keyword, not a JS expression
<Column>
  <if condition={isLoggedIn}>
    <Text>Welcome</Text>
  </if>
  <else>
    <Button variant="primary">Login</Button>
  </else>
</Column>

// Lists
<Column gap="sm">
  <each item={products} as="product">
    <ProductCard product={product} />
  </each>
</Column>
```

### Local components

> ⚠️ Superseded by [`clutter-block4a.md`](clutter-block4a.md). Content below kept for historical reference.

> Sub-components can be defined in the same file, before the `---` separator:
>
> ```
> component ProductCard(product: Product) {
>   <Box bg="surface" padding="md" radius="md">
>     <Column gap="sm">
>       <Text weight="bold">{product.name}</Text>
>       <Text color="secondary">{product.price}</Text>
>     </Column>
>   </Box>
> }
>
> ---
>
> <Column gap="md">
>   <each item={products} as="product">
>     <ProductCard product={product} />
>   </each>
> </Column>
> ```

---

## Type System & Design Tokens

### tokens.clutter

Design system configuration file, single source of truth. Format to be defined (JSON, TOML, or custom DSL) — priority: readable by humans and LLMs, not necessarily TypeScript.

**Categories**:
- `colors` — semantic scale and neutrals
- `spacing` — dimensional scale
- `typography` — sizes, weights, lineHeights
- `radii` — border radius
- `shadows` — shadow presets
- `breakpoints` — responsive values

### Enforcement

The compiler reads `tokens.clutter` and:
- Generates valid types for each prop
- Produces explicit errors for values not present in the tokens
- Errors report the used value and the available valid values

### Semantic errors (example)

```
Error [CLT001] — line 4, column 12
Value 'xl2' does not exist for prop 'gap'.
Valid values: xs, sm, md, lg, xl, xxl
```

---

## Component Model

### Importing external components

```
import ProductCard from "./ProductCard.clutter"
import { Modal, Drawer } from "./overlays.clutter"
```

Only `.clutter` files — no importing Vue components or arbitrary JS (except via `unsafe`).

### Component props

> ⚠️ Superseded by [`clutter-block4a.md`](clutter-block4a.md). Content below kept for historical reference.

> Defined in the logic section with standard TypeScript syntax:
>
> ```
> interface Props {
>   title: string
>   variant?: "primary" | "secondary"
>   onClick: () => void
> }
>
> const props = defineProps<Props>()
>
> ---
>
> <Button variant={props.variant} onClick={props.onClick}>
>   {props.title}
> </Button>
> ```

### Local state

```
import { reactive } from "clutter"

const count = reactive(0)
const increment = () => count.value++

---

<Row gap="sm" crossAxis="center">
  <Text>{count.value}</Text>
  <Button variant="primary" onClick={increment}>+</Button>
</Row>
```

`clutter` exposes a minimal reactive API — not the entire Vue or React API.

---

## Compiler Architecture

### Pipeline

```
.clutter file
     ↓
  Lexer
     ↓
  Parser → AST
     ↓
  Semantic Analyzer
  (validates tokens, props, references)
     ↓
  Plugin hooks
  (accessibility, test ids, etc.)
     ↓
  Code Generator
     ↓
  Output (Vue SFC | Vanilla | ...)
```

### Main phases

**Lexer**: tokenises the file, distinguishes the two sections, identifies tags, props, expressions

**Parser**: builds the template AST; the logic section is treated as opaque TypeScript and passed to the code generator unchanged (or nearly so)

**Semantic Analyzer**:
- Verifies that every prop receives a valid value from the design system
- Verifies that `{variable}` references exist in the logic section
- Verifies that used components are built-in or explicitly imported

**Plugin hooks**: extension points in the pipeline, before code generation

**Code Generator**: traverses the AST and produces the selected target

### Recommended stack for the compiler

To be evaluated — main options:

| Option | Pros | Cons |
|---|---|---|
| TypeScript | Ecosystem, ease of npm distribution | Performance on large files |
| Rust | Performance, Luca is learning it, WASM-friendly | Longer initial development time |
| Go | Performance, fast compilation | More complex npm ecosystem |

Rust is the most ambitious option and most consistent with long-term goals (WASM, tool distribution), but requires more initial time.

---

## Runtime

The built-in components (`Column`, `Row`, `Box`, etc.) are implemented in the Clutter runtime — compiled JS/TS code, opaque to the user, distributed as part of the package.

### Runtime principles

- No external dependencies at runtime (no Vue, no React)
- Minimal size — only what the built-in components need
- Minimal reactive API exposed to the logic section via `import { reactive, computed } from "clutter"`
- Compatibility with supported compilation targets

### When the target is Vue

The runtime is not needed — built-in components are compiled as valid Vue components, and the Vue runtime handles reactivity. The logic section uses the Vue API directly (`ref`, `computed`, etc.).

---

## Compilation Targets

### Vue SFC

Output: valid `.vue` files, integrable into any existing Vue/Nuxt app.

Use case: gradual migration of an existing Vue app.

```
// Input: ProductCard.clutter
// Output: ProductCard.vue (valid Vue SFC)
```

### Vanilla JS + HTML

Output: standard Web Components or plain HTML/JS, with no framework dependencies.

Use case: new projects, embedding in non-Vue contexts.

### Other targets (future)

Plugin architecture allows additional targets without modifying the core compiler.

---

## Unsafe Escape Hatch

For inevitable edge cases — third-party components, legacy integrations, cases not covered by the DSL.

### In the template

```
<Row gap="md">
  <Text>Normal content</Text>
  <unsafe>
    <div style="some-legacy-thing: value">...</div>
  </unsafe>
</Row>
```

### In the logic section

Arbitrary JS/TS code is already permitted in the logic section by definition — `unsafe` in the template explicitly signals that you are exiting the closed vocabulary.

### Principles

- `unsafe` is visible and deliberate — it is not a silent workaround
- It appears in compiler reports ("N unsafe blocks in the project")
- It is a signal for code review, not an error
- It does not disable checking on the surrounding parts

---

## Tooling

### Indispensable before day-1

- **CLI**: `clutter build`, `clutter watch`, `clutter check` (validation only without output)
- **Language Server (LSP)**: without autocomplete in the editor, the DX is worse than writing Vue by hand — it is not optional
- **VS Code extension**: LSP consumer, syntax highlighting, inline errors

### Useful post-MVP

- **Formatter**: automatic consistent style
- **Accessibility plugin**: compile-time warnings for non-accessible patterns
- **Test plugin**: automatic generation of test ids and basic assertions
- **Figma plugin**: generate `.clutter` from design

---

## Known Limitations & Tradeoffs

### Real costs

| Problem | Impact |
|---|---|
| No Vue DevTools | Harder runtime debugging |
| No third-party component ecosystem | Everything must be wrapped via `unsafe` or reimplemented |
| LSP to build from scratch | Without it the DX is poor — it is a prerequisite, not a nice-to-have |
| Parser/compiler to maintain | Additional bug surface compared to the library approach |
| Team learning curve | New format, new errors, new mental model |

### Compiler scope

The parser handles the `.clutter` template. The logic section is treated as opaque TypeScript — the compiler does not perform TypeScript type checking, it only verifies that references used in the template exist as identifiers in the logic section.

For full type checking of the logic section, TypeScript is needed — which remains optional and configurable, but not enforced.

---

## Future Extensions

- **React target** — if needed, addable as a target plugin
- **Hot module replacement** — for a smooth dev experience
- **Source maps** — for debugging compiled code
- **WASM build of the compiler** — zero-dependency distribution, maximum performance (consistent with the Rust path)
- **LLM integration** — the closed and predictable format is ideal for fine-tuning on UI generation tasks

---

*End of Specification*
