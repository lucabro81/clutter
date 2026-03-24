# Clutter Language Reference

Clutter is a UI markup language with a **closed vocabulary**. Every component, every prop, and every value must exist in the design system — if it doesn't compile, it doesn't ship.

---

## Table of Contents

- [File structure](#file-structure)
- [Component declaration](#component-declaration)
- [Logic section](#logic-section)
- [Template section](#template-section)
- [Built-in components](#built-in-components)
  - [Column](#column)
  - [Row](#row)
  - [Box](#box)
  - [Text](#text)
  - [Button](#button)
  - [Input](#input)
- [Control flow](#control-flow)
- [Unsafe escape hatch](#unsafe-escape-hatch)
- [Design tokens (tokens.json)](#design-tokens-tokensjson)
- [Output](#output)
- [Error codes](#error-codes)

---

## File structure

A `.clutter` file contains one or more component definitions. Each component is emitted as a separate output file.

```
component ComponentName(props: PropsType) {
  [logic section — TypeScript]
  ----
  [template section]
}

component AnotherComponent(props: OtherProps) {
  ----
  [template section]
}
```

- Every component is wrapped in `component Name(...) { }`
- `----` (exactly four dashes) separates the logic section from the template
- The separator is required even when the logic section is empty

---

## Component declaration

```
component Greeting(props: GreetingProps) {
```

- `component` — keyword
- `Greeting` — component name; becomes the output file name (`Greeting.vue`)
- `(props: GreetingProps)` — TypeScript props signature, treated as opaque by the compiler

The compiler does not parse or validate the props signature — it is passed verbatim into the generated `<script setup>` block.

---

## Logic section

The logic section is standard TypeScript. The compiler treats it as an opaque block and copies it verbatim into `<script setup lang="ts">`.

```
component Card(props: CardProps) {
const title = "Hello from Clutter";
const isVisible = props.show ?? true;
const items = ["design tokens", "type safety", "Vue SFC"];
----
```

Variables declared in the logic section can be referenced in the template with `{varName}`.

---

## Template section

The template is JSX-like but restricted to the closed vocabulary. Only built-in components are allowed — arbitrary HTML is not.

### String props

```
<Text value="Hello" size="lg" weight="bold" />
```

String prop values must be valid design token values (enforced at compile time).

### Expression props

```
<Text value={title} size="lg" />
<if condition={isVisible}>
```

`{varName}` references a variable from the logic section. The compiler validates that the identifier was declared there (error CLT104 if not).

### Boolean shorthand

```
<Button disabled />
```

Equivalent to `disabled={true}`.

### Self-closing and block elements

```
<Text value="Hello" />          <!-- self-closing -->

<Column gap="md">
  <Text value="Hello" />
</Column>
```

---

## Built-in components

### `Column`

Flex column layout.

| Prop | Valid values |
|------|-------------|
| `gap` | spacing tokens |
| `padding` | spacing tokens |
| `mainAxis` | `start` `end` `center` `spaceBetween` `spaceAround` `spaceEvenly` |
| `crossAxis` | `start` `end` `center` `stretch` |

### `Row`

Flex row layout.

| Prop | Valid values |
|------|-------------|
| `gap` | spacing tokens |
| `padding` | spacing tokens |
| `mainAxis` | `start` `end` `center` `spaceBetween` `spaceAround` `spaceEvenly` |
| `crossAxis` | `start` `end` `center` `stretch` |

### `Box`

Generic container.

| Prop | Valid values |
|------|-------------|
| `bg` | color tokens |
| `padding` | spacing tokens |
| `margin` | spacing tokens |
| `radius` | radius tokens |
| `shadow` | shadow tokens |

### `Text`

Typographic element.

| Prop | Valid values |
|------|-------------|
| `value` | any string or `{expr}` |
| `size` | typography size tokens |
| `weight` | typography weight tokens |
| `color` | color tokens |
| `align` | `left` `center` `right` |

### `Button`

Interactive action.

| Prop | Valid values |
|------|-------------|
| `variant` | `primary` `secondary` `outline` `ghost` `danger` |
| `size` | `sm` `md` `lg` |
| `disabled` | any (boolean shorthand supported) |

### `Input`

Text input field.

| Prop | Valid values |
|------|-------------|
| `placeholder` | any string or `{expr}` |
| `value` | any string or `{expr}` |
| `type` | `text` `email` `password` `number` |

---

## Control flow

### Conditional: `<if>`

```
<if condition={isVisible}>
  <Text value="Shown when true" />
</if>
```

### Conditional with else: `<if>` / `<else>`

```
<if condition={isLoggedIn}>
  <Text value="Welcome back" />
<else>
  <Button variant="primary">Log in</Button>
</else>
</if>
```

`<else>` is a sibling of the content inside `<if>`, not a child. The closing `</else>` and `</if>` are both required.

### List rendering: `<each>`

```
<each collection={items} as="item">
  <Text value={item} size="sm" />
</each>
```

| Attribute | Description |
|-----------|-------------|
| `collection` | expression referencing an array from the logic section |
| `as` | string literal — the item variable name inside the block |

---

## Unsafe escape hatch

For content that falls outside the closed vocabulary.

```
<unsafe reason="third-party DatePicker, no Clutter wrapper yet">
  <div class="legacy-datepicker">...</div>
</unsafe>
```

- The `reason` attribute is **required** — the compiler rejects `<unsafe>` without one (error CLT105)
- Content inside `<unsafe>` is passed through verbatim; no prop validation applies
- Using `<unsafe>` emits a warning but does not fail the build

---

## Design tokens (`tokens.json`)

The compiler reads `tokens.json` from the project root (or the path provided via `--tokens`).

```json
{
  "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
  "colors":     ["primary", "secondary", "danger", "surface", "background"],
  "typography": {
    "sizes":    ["xs", "sm", "base", "lg", "xl", "xxl"],
    "weights":  ["normal", "medium", "semibold", "bold"]
  },
  "radii":      ["none", "sm", "md", "lg", "full"],
  "shadows":    ["sm", "md", "lg"],
  "variables": {
    "--spacing-md":   "1rem",
    "--color-primary": "#3b82f6"
  }
}
```

### Token arrays

Define the valid vocabulary. Any prop value not in the relevant array is a compile error (CLT102).

### `variables` (optional)

Maps CSS custom property names to their values. When present, the compiler emits a `:root { }` block at the top of `clutter.css` so the generated utility classes resolve correctly.

Convention: variable names follow `--{category}-{token-name}` (e.g. `--spacing-md`, `--color-primary`) to match what the utility classes reference. Designers can also add arbitrary variables (`--brand-font-family`, etc.) that are emitted but not used by any utility class.

---

## Output

Running `clutter src/clutter/Greeting.clutter --tokens tokens.json --out src/components/` produces:

```
src/components/
├── Greeting.vue     ← one file per component definition
└── clutter.css      ← global: :root variables + utility classes
```

`clutter.css` is global and shared across all components. Import it once in your app entry point:

```ts
import './components/clutter.css'
```

---

## Error codes

| Code | Meaning |
|------|---------|
| CLT101 | Unknown prop on a built-in component |
| CLT102 | Invalid token value for a prop |
| CLT103 | Unknown component name |
| CLT104 | Undeclared identifier referenced in template |
| CLT105 | `<unsafe>` block missing `reason` attribute |
| CLT106 | `<unsafe>` used as a prop value without `reason` |
| CLT107 | Complex expression in prop (only simple identifiers allowed) |
