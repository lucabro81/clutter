# CLUTTER — Block 1: Lexer

**Version**: 0.1.0-draft
**Status**: Exploration
**Author**: Luca

---

## What a Lexer is and why it exists

A compiler does not read text the way we do. For a machine, a file is a flat sequence of characters — it does not know where one word ends and a number begins, it does not know that `<Column` is a different entity from `gap="md"`.

The Lexer (also called a *tokenizer* or *scanner*) is the first stage: it reads the text character by character and transforms it into a sequence of **tokens** — units with a type and a value.

Example:

```
<Column gap="md" padding="lg">
```

Becomes:

```
TOKEN_OPEN_TAG     "<"
TOKEN_IDENTIFIER   "Column"
TOKEN_IDENTIFIER   "gap"
TOKEN_EQUALS       "="
TOKEN_STRING       "md"
TOKEN_IDENTIFIER   "padding"
TOKEN_EQUALS       "="
TOKEN_STRING       "lg"
TOKEN_CLOSE_TAG    ">"
```

The next block (Parser) receives this sequence and builds structure on top of it — but it no longer needs to worry about characters, spaces, or quotes. The Lexer has already done that work.

---

## Why separate Lexer and Parser

This is a well-established design choice in all real compilers (GCC, Clang, Babel, TypeScript itself). The reason is simple: they are two different problems.

The Lexer answers: *"What are these characters?"*
The Parser answers: *"What does this sequence of tokens mean?"*

Keeping them separate makes both easier to write, test, and modify. If the syntax of a string changes (e.g. backtick support), you only modify the Lexer. If the grammar changes (e.g. a new `<if>` construct), you only modify the Parser.

---

## Clutter's tokens

For the POC, the Lexer must recognise these token types:

### Structural

| Token | Example | Description |
|---|---|---|
| `SECTION_SEPARATOR` | `---` | Separator between logic section and template |
| `OPEN_TAG` | `<Column` | Component tag opening |
| `CLOSE_TAG` | `>` | Closing of opening tag |
| `SELF_CLOSE_TAG` | `/>` | Self-closing tag close |
| `CLOSE_OPEN_TAG` | `</Column>` | Component closing tag |

### Props and values

| Token | Example | Description |
|---|---|---|
| `IDENTIFIER` | `Column`, `gap`, `title` | Component name or prop name |
| `EQUALS` | `=` | Prop assignment |
| `STRING` | `"md"`, `"primary"` | String literal value |
| `EXPRESSION` | `{title}`, `{count}` | Reference to a variable from the logic section |

### Control flow

| Token | Example | Description |
|---|---|---|
| `IF_OPEN` | `<if` | Conditional block opening |
| `ELSE_OPEN` | `<else>` | Alternative block |
| `EACH_OPEN` | `<each` | Iteration opening |

### Content

| Token | Example | Description |
|---|---|---|
| `TEXT` | `Hello world` | Static text between tags |
| `WHITESPACE` | ` `, `\n` | Spaces and newlines (often ignored) |
| `EOF` | — | End of file |

### Logic section

The logic section (TypeScript) is treated as an opaque block — the Lexer does not analyse it in detail, it collects it whole as a single `LOGIC_BLOCK` token. TypeScript type checking is not the responsibility of the Lexer or the Clutter Parser.

---

## How it works internally

The Lexer maintains a **current state** as it scans the text. The questions it asks at each character are always the same:

1. Am I in the logic section or in the template?
2. Does the current character start a new token?
3. Has the current token ended?

### The state machine

A Lexer is formally a *finite state machine*. You don't need to know the formal theory — the practical idea is this: the Lexer always knows which "mode" it is in, and each character can either confirm the current mode or cause a transition to another.

Main states for Clutter:

```
LOGIC       — reading the TypeScript section
TEMPLATE    — reading the template
IN_TAG      — inside an open tag (<Column ...)
IN_STRING   — inside a string "..."
IN_EXPR     — inside an expression {...}
```

Example transitions:

```
state: TEMPLATE
char: "<"
  → enter IN_TAG, start collecting the tag name

state: IN_TAG
char: " " (space)
  → emit IDENTIFIER token with the collected name, stay in IN_TAG

state: IN_TAG
char: "="
  → emit EQUALS, stay in IN_TAG

state: IN_TAG
char: '"'
  → enter IN_STRING

state: IN_STRING
char: '"' (second)
  → emit STRING with the collected value, return to IN_TAG
```

---

## Handling the `---` separator

The separator is the most peculiar case in Clutter — it does not exist in other markup languages.

The Lexer starts in `LOGIC` mode. When it encounters a line that contains exactly `---` (and nothing else), it emits `SECTION_SEPARATOR` and switches to `TEMPLATE` mode. From that point on, everything is read as template.

This means `---` in the TypeScript logic section would be a problem. Simple solution for the POC: document it as a reserved value not supported in the logic section. If needed in the future (e.g. decrement `x---`), it can be handled with more sophisticated context.

---

## Position information

Each token must carry its position in the source file:

```
{
  type: "STRING",
  value: "xl2",
  line: 4,
  column: 12
}
```

This is essential for producing useful errors in later stages:

```
Error [CLT001] — line 4, column 12
Value 'xl2' does not exist for prop 'gap'.
```

Without position, an error has no coordinates — useless in practice. All position information is collected during the lexing phase, because that is the only moment when the relationship between characters and source file lines is known.

---

## What the Lexer does not do

Clarifying boundaries is as useful as defining responsibilities.

The Lexer **does not** verify whether the syntax is correct — it can emit tokens from a malformed sequence without knowing it. It does not know whether `<Column` has a closing `>`. It does not know whether `gap="xl2"` is a valid value. It does not know the design system tokens.

These are the responsibilities of the Parser and the Semantic Analyzer.

---

## Error handling in the Lexer

The Lexer may encounter characters it cannot classify. The standard strategy is:

1. Emit an `UNKNOWN` token with the unrecognised character
2. Continue reading (do not stop at the first error)
3. Collect all errors, not just the first

The reason is practical: if the compiler stops at the first error, the developer must fix it, recompile, discover the second error, recompile. Collecting all possible errors in a single pass is much more useful.

---

## Block input and output

**Input**: text string — the raw content of a `.clutter` file

**Output**: array of tokens, each with type, value, and position

```
[
  { type: "LOGIC_BLOCK",       value: "const title = ...", line: 1,  col: 1  },
  { type: "SECTION_SEPARATOR", value: "---",               line: 5,  col: 1  },
  { type: "OPEN_TAG",          value: "Column",            line: 7,  col: 1  },
  { type: "IDENTIFIER",        value: "gap",               line: 7,  col: 8  },
  { type: "EQUALS",            value: "=",                 line: 7,  col: 11 },
  { type: "STRING",            value: "md",                line: 7,  col: 12 },
  ...
  { type: "EOF",               value: "",                  line: 12, col: 1  }
]
```

---

## How to test the Lexer

The Lexer is the simplest block to test in isolation — given a text input, the output is deterministic and verifiable.

Cases to cover:

- Minimal file (only the `---` separator, empty template)
- Component without props
- Component with a string prop
- Component with an expression prop `{var}`
- Nested components
- Logic section with real TypeScript code
- Unrecognised character → `UNKNOWN` token
- File without `---` separator → explicit error

---

## References

- [Crafting Interpreters](https://craftinginterpreters.com) — Ch. 3 (Scanning) — practical reference for implementing a lexer from scratch, language-agnostic
- `@vue/compiler-core` source → `packages/compiler-core/src/tokenizer.ts` — real-world example of a tokenizer for JSX-like syntax
- Babel source → `@babel/parser/src/tokenizer` — reference for state handling and positions

---

*End of Document*
