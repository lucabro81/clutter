# CLUTTER — Block 5: Runtime

**Version**: 0.1.0-draft
**Status**: Exploration
**Author**: Luca

---

## Disambiguation: what "runtime" means here

The term "runtime" is ambiguous — it is worth clarifying what it means in this document.

**Execution environment** (runtime in the common sense) — Node, Bun, the browser, the JVM. It is the machine that executes the code. Clutter has no control over or responsibility for this. There will always be a browser processing JS, and probably a Node or Bun server during development.

**Clutter runtime** (the sense used in this document) — the code that must be present in the execution environment of the output for that output to work. It is not the environment itself, it is the library that the generated code assumes is available.

A direct analogy: when React generates the virtual DOM, it assumes `react-dom` exists in the environment. `react-dom` is React's runtime — not Node, not the browser, but the library that React assumes is present. For Clutter it is the same.

---

## What the Clutter Runtime is and why it exists

The Clutter Runtime is the code that the Code Generator assumes exists in the environment when it produces output. It is not part of the compiler — the compiler assumes it is present, it does not manage it.

The answer to "what is needed at runtime" depends entirely on the compilation target. There is no monolithic Clutter Runtime — there is the runtime required by each target.

---

## Guiding principle

> The runtime is whatever the target needs, nothing more.

A proprietary runtime only makes sense when existing runtimes do not satisfy concrete needs. For the POC — and probably for a long time after — those needs do not exist. Building a proprietary runtime before a real need emerges would be wasted work on a problem not yet defined.

---

## Runtime per target

### Vue SFC target

**Required runtime: none.**

The Vue SFC target generates native HTML in the template and valid TypeScript in `<script setup>`. The runtime is Vue itself — already present in any Vue/Nuxt application that consumes the generated file.

The CSS classes are included in the `<style scoped>` of the generated file. Nothing to install, nothing to import.

This is one of the advantages of choosing to generate native HTML instead of `<Column>`, `<Text>` etc. components: the Vue SFC target is completely self-contained.

### HTML target

**Required runtime for the POC: Alpine.js**

Alpine.js handles reactivity in the HTML target — state, bindings, conditionals, iterations. It is an external dependency loaded via CDN or installed as a package.

The Code Generator automatically includes the Alpine reference in the produced HTML. No configuration required from the developer.

**Runtime at scale: `@vue/reactivity`**

At scale, the HTML target will use `@vue/reactivity` — Vue's reactivity package, distributed separately and usable without the rest of the framework. This eliminates the dependency on Alpine and unifies the reactive model across targets.

`@vue/reactivity` will be bundled into the Clutter runtime for the HTML target — the developer does not install it explicitly, it is an internal dependency of Clutter.

---

## Target inconsistency (architectural note)

The "different runtime for different target" approach carries a risk: Alpine and `@vue/reactivity` do not behave identically in all edge cases. Code that works in the Vue target may have slightly different behaviour in the HTML target.

For the POC this inconsistency is acceptable — use cases are simple and well-defined. At scale, unifying the reactive runtime on `@vue/reactivity` for both targets solves the problem at its root.

---

## Dev server and hot reload

A development environment with a file watcher and hot reload is out of scope for the POC, but it is worth defining how it is structured architecturally to avoid making wrong decisions now.

The Rust compiler is a binary — it takes a `.clutter` file and produces output. The file watcher that detects changes and re-runs the compiler does not need to be written in Rust: it is simpler and more pragmatic to use an existing watcher (Bun, Node, or even a simple shell script) that monitors `.clutter` files and invokes the Rust binary on each change.

Browser-side hot reload (update without full refresh) depends on the target:
- Vue target: handled by Vite/Nuxt, already present in the host application
- HTML target: would require a dev server with WebSocket — to be built or delegated to an existing tool (Bun serve, Vite in standalone mode)

In both cases the Rust compiler knows nothing about the dev server — it is the external layer that handles orchestration.

---

## A note on the future: the logic section language

The logic section of a `.clutter` file is today TypeScript. This choice is pragmatic for the POC, not architectural.

If the compiler is in Rust and targets are interchangeable, the logic section could in the future be written in any language that:
- Has a sufficiently expressive type system
- Can be compiled or transpiled to the target's runtime
- Has or can have a reactivity system implementable in the target

Rust, Go, or a proprietary DSL are all theoretically valid candidates. This is a direction consistent with the project's vision — to keep as a side note, not as an immediate goal.

---

## What the Runtime is not

The Runtime is not the compiler. It does not participate in the Lexer → Parser → Semantic Analyzer → Code Generator pipeline. It is not invoked by `clutter build`.

The Runtime exists in the execution environment of the output — in the browser, in the Vue application, in the produced HTML file. The compiler assumes it is present, it does not manage it.

---

## Proprietary runtime: when and why

A proprietary runtime means writing and directly controlling the code that manages reactivity, component mounting, lifecycle, and DOM updates — instead of delegating to Vue or Alpine.

### What it would do concretely

- Manage reactivity with a custom implementation, optimised for the assumptions Clutter can make (closed vocabulary, predictable structure)
- Mount components in the DOM tree
- Manage lifecycle (created, mounted, destroyed, etc.)
- Optimise DOM updates in a way specific to the Clutter compiler's output

### The realistic sweet spot

Reinventing reactivity from scratch is a project within a project. The sweet spot is taking `@vue/reactivity` or React's core, modifying it where necessary, and distributing it as an internal Clutter package. The developer does not see it, does not install it, does not know it exists — it is an internal dependency bundled in the output.

This gives full control over behaviour, zero explicit dependency on Vue or React for the developer, and does not require reinventing years of work on the reactivity problem.

### WASM: the direction for compute-intensive use cases

If the compiler is in Rust, the runtime could be written in Rust and compiled to **WebAssembly**. Generated code would call the WASM runtime instead of a JS library — the browser executes native bytecode instead of interpreting JavaScript.

The advantage is not reactivity — for updating the DOM the difference is negligible. The advantage is for compute-intensive operations: data processing, simulations, scientific visualisations, algorithms on large datasets. Cases where JS is the bottleneck and WASM solves the problem structurally.

This is the long-term direction for Clutter as a platform for data-intensive applications — not an immediate goal, but an arrow consistent with the choice of Rust as the compiler language.

---

## References

- [`@vue/reactivity`](https://github.com/vuejs/core/tree/main/packages/reactivity) — standalone Vue reactivity package, foundation for the proprietary runtime
- [Alpine.js](https://alpinejs.dev) — lightweight reactive runtime for the HTML target in the POC
- [Bun](https://bun.sh) — candidate for file watcher and dev server in the orchestration layer
- [Leptos](https://leptos.dev) — Rust framework with WASM runtime, reference for how a Rust→WASM runtime works in the browser
- [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) — tool for calling browser APIs from Rust compiled to WASM

---

*End of Document*
