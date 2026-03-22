# CLUTTER — Block 2: Parser

**Version**: 0.1.0-draft
**Status**: Exploration
**Author**: Luca

---

## What a Parser is and why it exists

The Lexer has transformed the raw text into a flat sequence of tokens. The Parser takes that sequence and builds a tree structure on top of it — the **AST** (Abstract Syntax Tree).

The fundamental difference is this: the Lexer reads characters in sequence without ever looking back or looking far ahead. The Parser, on the other hand, must understand the relationships between tokens — which are children of which, where a block starts and ends, whether a sequence is valid according to the language's grammar.

Example: the Lexer sees these tokens one after another and classifies them:

```
OPEN_TAG    "Column"
IDENTIFIER  "gap"
EQUALS      "="
STRING      "md"
CLOSE_TAG   ">"
OPEN_TAG    "Text"
...
CLOSE_OPEN_TAG "Column"
```

The Parser understands that `Text` is a child of `Column`, that `gap="md"` is a prop of `Column`, and that everything ends when it encounters `</Column>`.

---

## The AST — Abstract Syntax Tree

The AST is the representation of a `.clutter` file as a data structure. It is a tree because components nest — each node can have children, and children can have children.

"Abstract" means the tree no longer contains the details of the source text (quotes, angle brackets, spaces) — it contains only the structure and meaning extracted from that text.

Example: this template:

```
<Column gap="md">
  <Text size="lg">Hello</Text>
  <Button variant="primary">OK</Button>
</Column>
```

Produces this AST:

```
ComponentNode "Column"
  props:
    gap = "md"
  children:
    ComponentNode "Text"
      props:
        size = "lg"
      children:
        TextNode "Hello"
    ComponentNode "Button"
      props:
        variant = "primary"
      children:
        TextNode "OK"
```

This is the object that the Semantic Analyzer and the Code Generator receive and work with. No more strings, no more tokens — a navigable structure.

---

## Clutter's AST nodes

For the POC, the AST must represent these node types:

### Root node

> ⚠️ Superseded by [`clutter-block4a.md`](clutter-block4a.md). `ProgramNode` is replaced by `FileNode { components: Vec<ComponentDef> }`, where each `ComponentDef` carries its own `name`, `props_raw`, `logic_block`, and `template`. Content below kept for historical reference.

> ```
> ProgramNode
>   logicBlock: string       — the raw TypeScript content of the logic section
>   template: TemplateNode   — the template root
> ```
>
> The `ProgramNode` is always the root of the tree. It contains the logic block (treated as an opaque string) and the template.

### Template nodes

| Node | Description |
|---|---|
| `ComponentNode` | A component (`<Column>`, `<Text>`, etc.) with props and children |
| `TextNode` | Static text between tags |
| `ExpressionNode` | Variable reference `{title}` |
| `IfNode` | Conditional block `<if condition={...}>` |
| `EachNode` | Iteration `<each item={...} as="...">` |

### ComponentNode structure

```
ComponentNode
  name: string             — component name ("Column", "Text", ...)
  props: PropNode[]        — list of props
  children: Node[]         — child nodes
  position: Position       — line and column in the source

PropNode
  name: string             — prop name ("gap", "variant", ...)
  value: StringValue       — string literal value ("md", "primary")
        | ExpressionValue  — variable reference ({myVar})
  position: Position
```

---

## How it works internally

The Parser consumes tokens one at a time, keeping track of where it is in the structure. The technique used for Clutter is the **Recursive Descent Parser** — the most common approach for languages with relatively simple syntax, used by Babel, TypeScript, and the Vue compiler.

### Why Recursive Descent

Other approaches exist (table-driven parsers, parsers automatically generated from a grammar). Recursive Descent is written by hand, is simple to understand and debug, and lends itself well to producing quality errors. For a new language with custom syntax it is the natural choice.

### The principle

Each construct of the language has a dedicated function in the Parser. That function knows which tokens to expect, consumes them in order, and recursively calls the functions for nested constructs.

Pseudocode:

```
function parseComponent():
  consume OPEN_TAG → get the component name
  while the next token is IDENTIFIER:
    call parseProp() → add the prop to the node
  consume CLOSE_TAG
  while the next token is not CLOSE_OPEN_TAG:
    call parseNode() → add the child to the node
  consume CLOSE_OPEN_TAG
  return the complete ComponentNode

function parseProp():
  consume IDENTIFIER → get the prop name
  consume EQUALS
  if the next token is STRING:
    consume STRING → get the value
  if the next token is EXPRESSION:
    consume EXPRESSION → get the reference
  return PropNode

function parseNode():
  if the next token is OPEN_TAG:
    call parseComponent()
  if the next token is TEXT:
    return TextNode
  if the next token is EXPRESSION:
    return ExpressionNode
  if the next token is IF_OPEN:
    call parseIf()
  if the next token is EACH_OPEN:
    call parseEach()
```

Recursion occurs because `parseComponent` calls `parseNode`, and `parseNode` can call `parseComponent` — this is what allows arbitrary nesting of components.

### Lookahead

The Parser needs to "look ahead" by one token to decide which function to call — this technique is called **lookahead**. For Clutter, a lookahead of 1 is sufficient (you only look at the next token, not two or three ahead). This greatly simplifies the implementation.

---

## Handling the logic section

The logic section (TypeScript) is not analysed in detail by the Clutter Parser. It is collected as a raw string and inserted into the `ProgramNode` as `logicBlock`.

The reason is pragmatic: correctly parsing TypeScript is a problem already solved by the TypeScript compiler. Doing it again in Clutter would be enormous work for marginal benefit in the POC.

The only thing the Parser must do with the logic section is identify the names of declared identifiers — to allow the Semantic Analyzer to verify that `{variable}` references in the template exist. This can be done with a shallow analysis (collecting all words after `const`, `let`, `function`, `component`) without a full TypeScript parser.

---

## Error handling in the Parser

As with the Lexer, the goal is to collect as many errors as possible before stopping. The standard technique is **panic mode recovery**: when the Parser encounters an unexpected token, it reports the error and advances to a known synchronisation point (typically the end of a tag or the file), then resumes analysis from there.

Typical errors to handle:

- Open tag without a matching close
- Close tag without an open (`</Column>` without `<Column>`)
- Prop without `=` or without a value
- `{` expression without a closing `}`
- File without `---` separator

For each error: type, human-readable message, position (from the `position` of the involved token).

---

## Block input and output

**Input**: array of tokens produced by the Lexer

**Output**: AST — a `FileNode` object representing the entire file (see [`clutter-block4a.md`](clutter-block4a.md))

> ⚠️ Example below uses the old `ProgramNode` structure. Superseded by [`clutter-block4a.md`](clutter-block4a.md).

> ```
> ProgramNode {
>   logicBlock: "const title = 'Hello'\nconst handleClick = () => ...",
>   template: ComponentNode {
>     name: "Column",
>     props: [
>       PropNode { name: "gap", value: StringValue { value: "md" } }
>     ],
>     children: [
>       ComponentNode {
>         name: "Text",
>         props: [
>           PropNode { name: "size", value: StringValue { value: "lg" } }
>         ],
>         children: [
>           TextNode { value: "Hello" }
>         ]
>       }
>     ]
>   }
> }
> ```

---

## How to test the Parser

The Parser is tested in isolation starting from already-produced tokens — there is no need to re-run the Lexer every time.

Cases to cover:

- Template with a single component and no props
- Template with a component and string props
- Template with an expression prop `{var}`
- Two-level nesting
- Deep nesting (3+ levels)
- Self-closing component (`<Text />`)
- `<if>` block with and without `<else>`
- `<each>` block
- Open tag without close → error
- Prop without value → error
- File without `---` → error

---

## What the Parser does not do

The Parser does not know whether `gap="xl2"` is a valid value for that prop. It does not know the design system tokens. It does not know whether `{foo}` is a variable that actually exists in the logic section.

These are the responsibilities of the Semantic Analyzer.

The Parser only knows that the syntactic structure is correct — tags are balanced, props have the right form, expressions are well-formed.

---

## References

- [Crafting Interpreters](https://craftinginterpreters.com) — Ch. 5 (Representing Code) and Ch. 6 (Parsing Expressions) — AST construction and Recursive Descent
- [AST Explorer](https://astexplorer.net) — interactive tool, paste JS/Vue code and see the produced AST in real time. Useful for understanding what real-world ASTs look like
- `@vue/compiler-core` source → `packages/compiler-core/src/parse.ts` — parser for Vue templates, syntax very similar to Clutter's

---

*End of Document*
