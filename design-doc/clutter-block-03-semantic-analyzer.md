# CLUTTER — Block 3: Semantic Analyzer

**Version**: 0.1.0-draft
**Status**: Exploration
**Author**: Luca

---

## What the Semantic Analyzer is and why it exists

The Lexer has recognised the characters. The Parser has verified that the syntactic structure is correct — tags are balanced, props have the right form. But neither of them yet knows whether what is written actually makes sense.

The Semantic Analyzer is the block that answers the question: **does this syntactically correct code mean something valid?**

Concrete example: this template passes the Lexer and the Parser without issues —

```
<Column gap="xl2" color="banana">
  <Text>{pippo}</Text>
</Column>
```

It is syntactically perfect. But:
- `xl2` does not exist in the design system's spacing
- `color` is not a prop of `Column`
- `banana` does not exist in the design system's colors
- `pippo` is not a variable declared in the logic section

None of these errors is detectable by the Parser. They are errors of **meaning**, not of structure. The Semantic Analyzer finds all of them.

This is the block that demonstrates the value of the project. Zero CSS, zero configuration, zero conventions to remember — if you write something invalid, the compiler tells you before the code exists.

---

## Semantic Analyzer responsibilities

The Semantic Analyzer has three separate responsibilities, which it is useful to keep conceptually distinct:

**1. Prop type checking** — verifies that every prop of every component receives a value present in the design system (`tokens.clutter`)

**2. Reference checking** — verifies that every `{variable}` expression in the template corresponds to an identifier declared in the logic section

**3. Unsafe validation** — verifies that every use of `unsafe` is accompanied by a mandatory non-empty comment

These are three distinct problems with different sources of truth: the design system tokens for the first, the logic section for the second, Clutter's syntactic rules for the third.

---

## Source 1: tokens.clutter

`tokens.clutter` is the design system — the single source of truth for valid values. The Semantic Analyzer loads it before analysing any file.

Expected structure (format defined in the stack doc; JSON used here as reference):

```json
{
  "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
  "colors":     ["primary", "secondary", "danger", "surface", "background"],
  "typography": {
    "sizes":    ["xs", "sm", "base", "lg", "xl", "xxl"],
    "weights":  ["normal", "medium", "semibold", "bold"]
  },
  "radii":      ["none", "sm", "md", "lg", "full"],
  "shadows":    ["sm", "md", "lg"]
}
```

From `tokens.clutter` the Semantic Analyzer builds an internal map that associates each prop of each component with the set of valid values. This map is the basis of type checking.

---

## The props → tokens map

For each built-in component, the Semantic Analyzer knows which props exist and which token category they belong to.

Example for the POC:

```
Column:
  gap      → spacing
  padding  → spacing
  mainAxis → ["start", "end", "center", "spaceBetween", "spaceAround", "spaceEvenly"]
  crossAxis → ["start", "end", "center", "stretch"]

Text:
  size     → typography.sizes
  weight   → typography.weights
  color    → colors
  align    → ["left", "center", "right"]

Button:
  variant  → ["primary", "secondary", "outline", "ghost", "danger"]
  size     → ["sm", "md", "lg"]
  disabled → boolean

Box:
  bg       → colors
  padding  → spacing
  margin   → spacing
  radius   → radii
  shadow   → shadows
```

This map is hardcoded for built-in components in the POC. In the future, with custom components, it will be generated dynamically from the component definition.

---

## How it works internally

The Semantic Analyzer traverses the AST with a recursive visit — starting from the root (`ProgramNode`) and descending toward the leaves, analysing each node.

### Visit pseudocode

```
function analyzeProgram(node: ProgramNode):
  identifiers = extractIdentifiers(node.logicBlock)
  analyzeTemplate(node.template, identifiers)

function analyzeTemplate(node, identifiers):
  for each child of node:
    if it is a ComponentNode:
      analyzeComponent(node, identifiers)
    if it is an ExpressionNode:
      analyzeExpression(node, identifiers)
    if it is an IfNode or EachNode:
      analyzeControlFlow(node, identifiers)
    if it is an UnsafeBlockNode:
      analyzeUnsafeBlock(node)

function analyzeComponent(node: ComponentNode, identifiers):
  verify that node.name is a built-in or imported component

  for each prop in node.props:
    verify that prop.name exists in the map for node.name

    if prop.value is StringValue:
      verify that prop.value.value is in the valid value set for that prop

    if prop.value is ExpressionValue:
      verify that the name in the expression exists in identifiers

    if prop.value is UnsafeValue:
      analyzeUnsafeValue(prop.value)

  for each child in node.children:
    analyzeTemplate(child, identifiers)

function analyzeExpression(node: ExpressionNode, identifiers):
  verify that node.name exists in identifiers

function analyzeUnsafeBlock(node: UnsafeBlockNode):
  if node.reason is absent or empty string:
    emit error CLT105
  — the block content is not analysed

function analyzeUnsafeValue(node: UnsafeValue):
  if node.reason is absent or empty string:
    emit error CLT106
  — the custom value is not validated against tokens
```

### Extracting identifiers from the logic section

The logic section is raw TypeScript — it is not fully parsed. For the POC, a shallow analysis is sufficient: collect all names that follow `const`, `let`, `var`, `function`, and the custom keyword `component`.

```
const title = "Hello"      → identifiers: ["title"]
let count = 0              → identifiers: ["title", "count"]
function handleClick() {}  → identifiers: ["title", "count", "handleClick"]
component Card(props) {}   → identifiers: ["title", "count", "handleClick", "Card"]
```

This approach has false negatives (e.g. destructuring, imports) — acceptable for the POC. Documented as a known limitation.

---

## Semantic errors

Errors are the primary product of the Semantic Analyzer — not just the fact that something is wrong, but why and how to fix it.

### Error types

**Unknown prop**
```
Error [CLT101] — line 3, column 5
Prop 'color' does not exist on component 'Column'.
```

**Value not in design system**
```
Error [CLT102] — line 4, column 12
Value 'xl2' is not valid for prop 'gap' of 'Column'.
Valid values: xs, sm, md, lg, xl, xxl
```

**Unknown component**
```
Error [CLT103] — line 7, column 3
Component 'Grid' is not recognised.
Available components: Column, Row, Box, Text, Button, Input
```

**Reference to undeclared variable**
```
Error [CLT104] — line 9, column 8
'{foo}' is not declared in the logic section.
```

**Unsafe block without reason**
```
Error [CLT105] — line 12, column 3
<unsafe> requires a non-empty 'reason' attribute.
Example: <unsafe reason="legacy component, replace with Wrapper in v2">
```

**Unsafe value without reason**
```
Error [CLT106] — line 15, column 10
unsafe() requires a second argument with the justification.
Example: unsafe('16px', 'non-standard spacing, add token print-spacing')
```

### Error structure

```
ErrorNode
  code:     string     — error code (CLT101, CLT102, ...)
  message:  string     — human-readable description
  hint:     string?    — optional suggestion ("Valid values: ...")
  position: Position   — line and column in the source
  severity: "error"    — for the POC all errors block compilation
           | "warning" — future: non-blocking warnings
```

### Error collection

Like the Lexer and the Parser, the Semantic Analyzer does not stop at the first error. It continues the AST traversal collecting all errors found, then reports them all together at the end.

The only exception: if a component is unknown (CLT103), there is no point analysing its props — we do not know which are valid. In this case the prop analysis for that node is skipped and traversal continues with siblings.

---

## Warnings vs Errors

For the POC, everything is a blocking error — if the Semantic Analyzer finds problems, the Code Generator is not executed.

In the future it makes sense to distinguish:

- **Error**: value not in design system, unknown component, undeclared variable → blocks compilation
- **Warning**: redundant prop (e.g. `gap` on a component with no children), discouraged pattern → compiles but warns

This distinction is out of scope for now.

---

## Block input and output

**Input**: AST produced by the Parser + `tokens.clutter` file

**Output**: one of two possible cases

*Case 1 — no errors*: the original AST is passed to the Code Generator unchanged (or annotated with additional information collected during analysis)

*Case 2 — errors found*: list of `ErrorNode` with code, message, hint, and position. Compilation stops here; the Code Generator is not executed.

```
// Case 1
{
  success: true,
  ast: ProgramNode { ... }
}

// Case 2
{
  success: false,
  errors: [
    { code: "CLT102", message: "Value 'xl2' is not valid...", hint: "Valid values: xs, sm, ...", position: { line: 4, col: 12 } },
    { code: "CLT104", message: "'{foo}' is not declared...", position: { line: 9, col: 8 } }
  ]
}
```

---

## How to test the Semantic Analyzer

The Semantic Analyzer is tested by providing an AST directly — there is no need to re-run the Lexer and Parser every time. This makes tests fast and precise.

Cases to cover:

**Valid (no errors expected)**
- Component with all correct props
- Component without props
- Expression referencing a declared variable
- All-valid nested components

**Expected errors**
- Prop with a value not in the design system → CLT102
- Unknown prop for that component → CLT101
- Unrecognised component → CLT103
- Expression with undeclared variable → CLT104
- `<unsafe>` block without `reason` attribute → CLT105
- `<unsafe>` block with empty `reason` → CLT105
- `unsafe()` value without second argument → CLT106
- `unsafe()` value with empty second argument → CLT106
- File with multiple errors → all reported, not just the first

**Valid unsafe (no errors expected)**
- `<unsafe reason="...">` with non-empty reason → compiles, content ignored
- `unsafe('16px', 'justification...')` with both arguments → compiles, value passed verbatim

---

## A note on the design system as a type system

It is worth pausing on this for a moment.

In a normal application, the design system is documentation — a list of values to follow by convention. If you write `gap: 17px` instead of `gap: var(--spacing-md)`, nothing prevents it. You discover it in code review, or you never discover it.

In Clutter, the design system is the type system. `gap="xl2"` is not a violated convention — it is a compilation error, exactly like passing a string where a compiler expects a number.

This is the conceptual leap that justifies all the complexity of the project. The Semantic Analyzer is the point where that leap becomes concrete.

---

## References

- [Crafting Interpreters](https://craftinginterpreters.com) — Ch. 11 (Resolving and Binding) — scope management and reference resolution
- [Writing a Compiler in Go](https://compilerbook.com) — Ch. 4 (Symbol Table) — identifier management and scopes
- TypeScript source → `src/compiler/checker.ts` — TypeScript's type checker, useful reference for understanding the scale of the problem (not to read in full, but useful to see how a real semantic analyzer is structured)

---

*End of Document*
