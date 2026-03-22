# CLUTTER — Block 4: Code Generator

**Version**: 0.2.0
**Status**: Active — Vue SFC target in progress
**Author**: Luca

> **POC scope**: only the Vue SFC target is implemented for the POC.
> The HTML/Alpine.js target is deferred — see `todo/00-backlog.md` (Block 4: Codegen).
> References to `ProgramNode` in this document are superseded by `FileNode` /
> `ComponentDef` — see `design-doc/clutter-block4a.md`.

---

## What the Code Generator is and why it exists

The Code Generator is the last stage of the compilation pipeline. It receives the AST validated by the Semantic Analyzer and transforms it into code in the target language — for the POC, Vue SFC or static HTML.

It is the block that closes the loop: from a `.clutter` file written by the developer, you get working code that a browser or a Vue application can execute.

The Code Generator decides nothing — everything has already been decided. The Lexer recognised, the Parser structured, the Semantic Analyzer validated. The Code Generator simply translates the AST faithfully into the target's syntax.

This separation is intentional: keeping the validation logic separate from the generation logic allows adding targets without touching anything already built.

---

## The concept of a target

A target is a specific code generation strategy. The same AST can produce different output depending on the selected target.

For the POC:

**Vue SFC target** — produces a valid `.vue` file, integrable into any existing Vue/Nuxt application. It is the primary target because it allows gradual adoption on real projects.

**Static HTML target** — produces HTML and inline CSS, with no dependencies. Useful for validating the concept in isolation, without needing a Vue application.

In the future each target is a separate module — the Code Generator core traverses the AST, each target knows how to translate each node type. Adding React as a target means adding a module, not modifying the compiler.

---

## How it works internally

The Code Generator visits the AST using a technique called the **Visitor Pattern** — for each AST node type there is a function that knows how to translate it into the current target.

### The Visitor Pattern

The Visitor Pattern is the standard way compilers traverse an AST to produce output. The idea is simple: instead of putting the generation logic inside the nodes themselves, you define an external object (the visitor) that has a function for each node type.

```
Visitor {
  visitFileNode(node)        → iterates ComponentDefs, produces Vec<GeneratedFile>
  visitComponentDef(node)    → generates a single .vue file (template + script + style)
  visitComponentNode(node)   → generates a component tag mapped to native HTML
  visitTextNode(node)        → generates static text
  visitExpressionNode(node)  → generates the variable reference
  visitIfNode(node)          → generates the conditional construct
  visitEachNode(node)        → generates the iteration
  visitUnsafeNode(node)      → passes children through unchanged
}
```

When the Code Generator encounters a `ComponentNode`, it calls `visitComponentNode`. That function generates the opening tag, then recursively calls the visitor on children, then generates the closing tag.

The advantage over a large `if/else` or `switch` is that each target implements its own visitor — same AST, different translation for each target.

---

## Vue SFC target

A Vue SFC (Single File Component) has this structure:

```
<template>
  ...
</template>

<script setup lang="ts">
  ...
</script>
```

The Code Generator for Vue must produce exactly this.

### Design choice: native HTML in the template

Clutter's built-in components are expanded to native HTML in the Vue template — not as `<Column>`, `<Text>` etc. components. The reason is pragmatic: a `.vue` file with standard HTML works in any Vue application without installing anything. No runtime dependencies, no additional imports.

Clutter's semantic props (`gap="md"`, `variant="primary"`) are translated into CSS classes generated from `tokens.clutter`. The corresponding `<style>` block is included in the produced `.vue` file.

### Node mapping

**ComponentDef → complete .vue file**

```
<template>
  [template output — native HTML]
</template>

<script setup lang="ts">
  [logicBlock unchanged]
</script>

<style scoped>
  [CSS classes generated from tokens.clutter]
</style>
```

**ComponentNode Column → flex column div**

```
// AST input
ComponentNode { name: "Column", props: [{ name: "gap", value: "md" }], children: [...] }

// Vue output
<div class="clutter-column clutter-gap-md">
  [children output]
</div>
```

**ComponentNode Row → flex row div**

```
<div class="clutter-row clutter-gap-md">
  [children output]
</div>
```

**ComponentNode Text → typographic element**

```
// Input: <Text size="lg" weight="bold">Hello</Text>
// Output:
<p class="clutter-text clutter-size-lg clutter-weight-bold">Hello</p>
```

**ComponentNode Button → native button**

```
// Input: <Button variant="primary">OK</Button>
// Output:
<button class="clutter-button clutter-variant-primary">OK</button>
```

**ComponentNode Box → generic div**

```
// Input: <Box bg="surface" padding="md" radius="lg">...</Box>
// Output:
<div class="clutter-box clutter-bg-surface clutter-padding-md clutter-radius-lg">
  [children output]
</div>
```

**Props with expression → Vue binding**

Props that receive an `{variable}` expression become Vue bindings with `:`.

```
// Input: <Button disabled={isLoading}>
// Output:
<button :disabled="isLoading" class="clutter-button">
```

**TextNode → text in template**

```
TextNode { value: "Hello" }  →  Hello
```

**ExpressionNode → Vue interpolation**

```
ExpressionNode { name: "title" }  →  {{ title }}
```

**IfNode → v-if / v-else**

```
// Clutter input
<if condition={isLoggedIn}>
  <Text>Welcome</Text>
</if>
<else>
  <Button variant="primary">Login</Button>
</else>

// Vue output
<p v-if="isLoggedIn" class="clutter-text">Welcome</p>
<button v-else class="clutter-button clutter-variant-primary">Login</button>
```

**EachNode → v-for**

```
// Clutter input
<each item={products} as="product">
  <Box padding="md">...</Box>
</each>

// Vue output
<div
  v-for="product in products"
  :key="product"
  class="clutter-box clutter-padding-md"
>
  [children output]
</div>
```

### The generated style block

The CSS classes used in the template are defined in the `<style scoped>` of the `.vue` file, generated from `tokens.clutter`:

```css
.clutter-column { display: flex; flex-direction: column; }
.clutter-row    { display: flex; flex-direction: row; }
.clutter-box    { box-sizing: border-box; }

.clutter-gap-xs  { gap: 4px; }
.clutter-gap-sm  { gap: 8px; }
.clutter-gap-md  { gap: 16px; }
/* ... one class for each value of each token category */

.clutter-bg-primary  { background-color: #007AFF; }
.clutter-bg-surface  { background-color: #F2F2F7; }
/* ... */
```

This block is generated automatically — it is never written by hand.

### The logic section

The TypeScript logic section is inserted into `<script setup>` unchanged. The Code Generator does not touch it — it is already valid TypeScript, Vue knows how to handle it.

---

## HTML target *(deferred — post-POC)*

> This section describes the planned HTML target. It is **not implemented in the POC**.
> Deferred reasons: requires TypeScript → JS transpilation (esbuild/tsc) as a build
> dependency, and adds Alpine.js as a runtime dependency. See `todo/00-backlog.md`.



The HTML target produces a standalone `.html` file — no dependency on Vue or a build step. It is the target that demonstrates the portability of the compiler: the same `.clutter` source can also run outside a Vue ecosystem.

### The reactivity problem

An honest HTML target must execute the logic section of the source — if the developer has written state and handlers, they must work in the output. Ignoring the logic section would produce static HTML that does not honour the promises of the source.

The problem is that reactivity (state that automatically updates the DOM) is not free in plain HTML — it requires a runtime.

### POC: Alpine.js as a temporary runtime

For the POC, **Alpine.js** is used as the reactive runtime. Alpine is essentially "Vue without a build step": it declares state and bindings directly in HTML attributes, weighs ~15KB, and requires no configuration.

The Code Generator for the HTML target translates:
- The logic section → Alpine `x-data` object
- `{variable}` expressions → `x-text="variable"`
- Prop bindings → Alpine attributes (`:class`, `:disabled`, etc.)
- `<if condition={...}>` → `x-show` or `x-if`
- `<each item={...}>` → `x-for`

```html
<!-- HTML output with Alpine -->
<!DOCTYPE html>
<html>
<head>
  <style>
    /* CSS generated from tokens.clutter */
    .clutter-column { display: flex; flex-direction: column; }
    .clutter-gap-md  { gap: 16px; }
    /* ... */
  </style>
  <script src="https://cdn.jsdelivr.net/npm/alpinejs@3/dist/cdn.min.js" defer></script>
</head>
<body>
  <div x-data="{ title: 'Hello', count: 0 }">
    <div class="clutter-column clutter-gap-md">
      <p class="clutter-text clutter-size-lg" x-text="title"></p>
      <button class="clutter-button clutter-variant-primary" @click="count++">
        Click
      </button>
    </div>
  </div>
</body>
</html>
```

The TypeScript logic section is transpiled to vanilla JS (via `esbuild` or `tsc`) and injected into the `x-data` object.

### At scale: @vue/reactivity as the proper runtime

Alpine is an acceptable temporary solution for the POC. At scale, Clutter will have its own runtime based on `@vue/reactivity` — the package that manages reactivity in Vue, distributed separately and usable independently.

This allows:
- Eliminating the dependency on Alpine
- Full control over reactive behaviour
- Keeping the Clutter source syntax consistent between Vue and HTML targets
- Not reinventing reactivity from scratch — `@vue/reactivity` is battle-tested

At scale, the Code Generator produces HTML + a small script that initialises the Clutter runtime, mounts the component, and connects reactive state to the DOM.

---

## Code generation as strings

Concretely, the Code Generator builds the target code as a text string. Each `visit*` function returns a string that is concatenated with strings from neighbouring nodes.

### Indentation

Generated code must be readable — not a blob of text on one line. The Code Generator tracks the current nesting level and indents accordingly.

Pseudocode:

```
function visitComponentNode(node, depth):
  indent = "  ".repeat(depth)
  output = indent + "<" + node.name

  for each prop in node.props:
    output += " " + visitProp(prop)

  if node.children is empty:
    output += " />"
    return output

  output += ">\n"

  for each child in node.children:
    output += visitNode(child, depth + 1) + "\n"

  output += indent + "</" + node.name + ">"
  return output
```

### Source maps (out of scope for the POC)

In real compilers, the code generator also produces **source maps** — files that map each line of the generated code to the corresponding line in the original source. This allows the debugger to show the original `.clutter` file instead of the generated `.vue`.

Source maps are out of scope for the POC, but it is useful to know they exist and that they are produced at this stage.

---

## Block input and output

**Input**: validated `FileNode` (output of the parser/analyzer) + `DesignTokens`

**Output**: one `GeneratedFile { name: String, content: String }` per `ComponentDef`
in the `FileNode`. A single `.clutter` file with N components produces N `.vue` files.

```
// Input
FileNode {
    components: [
        ComponentDef { name: "MainComponent", ... },
        ComponentDef { name: "Card", ... },
    ]
}

// Output
[
    GeneratedFile { name: "MainComponent", content: "<template>…</template>…" },
    GeneratedFile { name: "Card",          content: "<template>…</template>…" },
]
```

The CLI writes each `GeneratedFile` to disk as `{name}.vue`.

> Note: the previous version of this section referenced `ProgramNode` and a single
> output string. Updated to reflect the multi-component format introduced in Block 4A.

---

## How to test the Code Generator

The Code Generator is tested by providing an AST directly — there is no need to re-run the entire pipeline every time.

Cases to cover:

**Vue target**
- Component without props → Vue tag without attributes
- Component with string prop → Vue attribute
- Component with expression prop → Vue binding with `:`
- TextNode → text in template
- ExpressionNode → `{{ }}` interpolation
- Nesting → correct indentation
- IfNode → `v-if` / `v-else`
- EachNode → `v-for`
- Logic section → inserted unchanged in `<script setup>`

**HTML target**
- Column → div with flex column and CSS classes
- Row → div with flex row and CSS classes
- Text with static content → p with CSS classes
- ExpressionNode → `x-text` Alpine attribute
- IfNode → `x-show` Alpine
- EachNode → `x-for` Alpine
- Logic section → Alpine `x-data` object
- `<style>` block generated from tokens.clutter present in the file

**General**
- Output is syntactically valid code (verifiable by parsing the result)
- Consistent indentation
- No unclosed tags

---

## What the Code Generator does not do

The Code Generator validates nothing — that responsibility belongs to the Semantic Analyzer. If it receives a valid AST, it produces valid output. It does no additional checks, does not transform the logic, does not optimise.

It does not write the file to disk — that is the CLI's job. The Code Generator produces a string; whoever calls it decides what to do with it.

---

## References

- [Crafting Interpreters](https://craftinginterpreters.com) — Ch. 8 (Statements and State) and Ch. 23 (Jumping Back and Forth) — code generation and visitor pattern
- [Design Patterns — Visitor](https://refactoring.guru/design-patterns/visitor) — clear explanation of the Visitor Pattern with practical examples
- `@vue/compiler-core` source → `packages/compiler-core/src/codegen.ts` — Vue compiler's code generator, reference for the Vue SFC target
- [Alpine.js](https://alpinejs.dev) — lightweight reactive runtime used in the HTML target for the POC
- [`@vue/reactivity`](https://github.com/vuejs/core/tree/main/packages/reactivity) — Vue's standalone reactivity package, target of the Clutter runtime at scale

---

*End of Document*
