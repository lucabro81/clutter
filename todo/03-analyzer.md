# Block 3: Analyzer

Legend: `[ ]` todo · `[x]` done · `[-]` skipped/deferred

---

## Context

The analyzer receives a `ProgramNode` (from the parser) and a `DesignTokens` value (loaded from
`tokens.json`). It walks the AST and validates:
- Every `PropValue::StringValue` against the design token set (CLT101–103)
- Every `PropValue::ExpressionValue` and `ExpressionNode` against declared identifiers (CLT104)

`tokens.json` lives at the workspace root. Unsafe validation (CLT105–106) is deferred — no
lexer/parser support yet (tracked in backlog).

Pipeline position: `Parser → **Analyzer** → Codegen`

---

## clutter-runtime — additions

- [x] `AnalyzerError { message: String, pos: Position }` — semantic error type
- [-] `ComponentKind` enum — kept internal to clutter-analyzer (not needed in runtime)
- [-] `PropDef { token_category: TokenCategory }` — superseded by `PropValidation` enum
- [-] `TokenCategory` enum in runtime — kept internal to clutter-analyzer

---

## Design token loading — `clutter-analyzer`

- [x] `DesignTokens` struct mirroring the `tokens.json` JSON shape
- [x] `DesignTokens::from_str(json: &str) -> Result<DesignTokens, serde_json::Error>`
- [x] `DesignTokens::valid_values(category: TokenCategory) -> &[String]`

---

## Prop map — closed vocabulary

- [x] `prop_map(component: &str, prop: &str) -> Option<PropValidation>`
  - `PropValidation::Tokens(TokenCategory)` — value checked against design tokens
  - `PropValidation::Enum(&[&str])` — value checked against a fixed set
  - `PropValidation::AnyValue` — prop valid, no value restriction
  - `None` for unknown component or unknown prop on known component

---

## clutter-analyzer — tests (written BEFORE implementation)

- [x] Valid prop value → no errors (`gap="md"` with `md` in spacing tokens)
- [x] Invalid prop value → `AnalyzerError` (CLT102) with message listing valid values
- [x] `ExpressionValue` prop with known identifier → no errors
- [x] `ExpressionValue` prop with unknown identifier → CLT104
- [x] Unknown component → `AnalyzerError` (CLT103)
- [x] Unknown prop on known component → `AnalyzerError` (CLT101)
- [x] Multiple errors in one file → all collected, not just the first
- [x] Nested component (child of `Column`) → props validated the same way
- [x] `<if>` / `<each>` children → their props are validated recursively
- [x] Empty template → no errors
- [x] `ExpressionNode` with known identifier → no errors (CLT104)
- [x] `ExpressionNode` with unknown identifier → CLT104
- [x] `<each>` alias in scope for children → no CLT104 false positives

---

## clutter-analyzer — implementation

- [x] `pub fn analyze(program: &ProgramNode, tokens: &DesignTokens) -> Vec<AnalyzerError>`
- [x] `analyze_nodes(nodes, tokens, identifiers, errors)` — recursive walker
- [x] `analyze_component(node, tokens, identifiers, errors)` — CLT103 + prop validation + recurse
- [x] `validate_prop(component, prop, tokens, identifiers) -> Vec<AnalyzerError>` — CLT101/102/104
- [x] `analyze_if` — CLT104 on condition, recurse into then/else children
- [x] `analyze_each` — CLT104 on collection, alias added to scope for children
- [x] `check_reference(name, pos, identifiers) -> Option<AnalyzerError>` — CLT104 helper
- [x] `extract_identifiers(logic_block) -> HashSet<String>` — shallow scan of logic block

---

## clutter-analyzer — integration tests

- [x] `fixtures/valid.clutter` → zero analyzer errors
- [x] `fixtures/invalid_token.clutter` → CLT102 on `xl2` and `huge`
- [x] `fixtures/complex.clutter` → zero errors

---

## Error format (target)

```
error — line 4, column 12
  Invalid value 'xl2' for prop 'gap' on 'Column'.
  Valid values: xs, sm, md, lg, xl, xxl
```

Full `miette` integration deferred to Block 5 (CLI). For now errors carry `message + pos`.
