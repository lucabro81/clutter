# Block 3: Analyzer

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## Context

The analyzer receives a `ProgramNode` (from the parser) and a path to `tokens.clutter` (JSON).
It walks the AST and validates every `PropValue::StringValue` against the design token set.
`PropValue::ExpressionValue` props are runtime references — passed through without validation.

Pipeline position: `Parser → **Analyzer** → Codegen`

---

## clutter-runtime — additions

- [ ] `AnalyzerError { message: String, pos: Position }` — semantic error type
- [ ] `ComponentKind` enum — closed vocabulary of known components:
  `Column | Row | Box | Text | Button | Input`
- [ ] `PropDef { token_category: TokenCategory }` — what token category a prop maps to
- [ ] `TokenCategory` enum — `Spacing | Color | FontSize | FontWeight | LineHeight | Radius | Shadow`

---

## Design token loading — `clutter-analyzer`

- [ ] `DesignTokens` struct mirroring the `tokens.clutter` JSON shape
- [ ] `DesignTokens::from_str(json: &str) -> Result<DesignTokens, _>` — deserialize with `serde_json`
- [ ] `DesignTokens::valid_values(category: TokenCategory) -> Vec<&str>` — return all valid values for a category

---

## Prop map — closed vocabulary

Hardcoded mapping: `(ComponentKind, prop_name) → TokenCategory`.
Examples: `(Column, "gap") → Spacing`, `(Text, "size") → FontSize`, `(Text, "color") → Color`.

- [ ] `prop_category(component: &str, prop: &str) -> Option<TokenCategory>`
  - Returns `None` for unknown component/prop pairs → separate "unknown prop" error
  - Returns `Some(category)` for known pairs → value validated against that category

---

## clutter-analyzer — tests (written BEFORE implementation)

All tests pass a `ProgramNode` built from tokens (same helper pattern as parser tests) + a
`DesignTokens` value. No filesystem access needed in unit tests.

- [ ] Valid prop value → no errors (`gap="md"` with `md` in spacing tokens)
- [ ] Invalid prop value → `AnalyzerError` with message listing valid values
- [ ] `ExpressionValue` prop → always passes validation (runtime reference)
- [ ] Unknown component → `AnalyzerError` ("unknown component: Foo")
- [ ] Unknown prop on known component → `AnalyzerError` ("unknown prop 'xyz' on Column")
- [ ] Multiple errors in one file → all collected, not just the first
- [ ] Nested component (child of `Column`) → props validated the same way
- [ ] `<if>` / `<each>` children → their props are validated recursively
- [ ] Empty template → no errors

---

## clutter-analyzer — implementation

- [ ] `pub fn analyze(program: &ProgramNode, tokens: &DesignTokens) -> Vec<AnalyzerError>`
  — public entry point, returns all errors (empty vec = valid)
- [ ] `analyze_nodes(nodes: &[Node], tokens: &DesignTokens, errors: &mut Vec<AnalyzerError>)`
  — recursive walker
- [ ] `analyze_component(node: &ComponentNode, tokens: &DesignTokens, errors: &mut Vec<AnalyzerError>)`
  — validates each prop, then recurses into children
- [ ] `validate_prop(component: &str, prop: &PropNode, tokens: &DesignTokens) -> Option<AnalyzerError>`
  — returns `Some(error)` if invalid, `None` if valid or expression

---

## clutter-analyzer — integration tests

- [ ] `fixtures/valid.clutter` → zero analyzer errors
- [ ] `fixtures/invalid_token.clutter` → at least one `AnalyzerError` with correct message
- [ ] `fixtures/complex.clutter` → zero errors (all prop values use real token names)

---

## Error format (target)

```
error — line 4, column 12
  Invalid value 'xl2' for prop 'gap' on 'Column'.
  Valid values: xs, sm, md, lg, xl, xxl
```

Full `miette` integration deferred to Block 5 (CLI). For now errors carry `message + pos`.
