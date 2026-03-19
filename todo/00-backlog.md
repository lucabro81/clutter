# Backlog — miglioramenti trasversali

Idee emerse durante lo sviluppo che non appartengono a un blocco specifico.
Da affrontare quando il contesto è maturo, non necessariamente in ordine.

---

## Error handling

| Item | Dettaglio | Quando |
|------|-----------|--------|
| `emit` nel lexer | Il lexer fa `errors.push(LexError { … })` direttamente nei siti di errore; il parser ha `emit(&mut self, msg, pos)` centralizzato. Portare lo stesso pattern nel lexer per coerenza. | Qualsiasi momento |
| Modulo `clutter-diagnostics` (valutare) | `LexError`, `ParseError` e `AnalyzerError` hanno la stessa struttura `{ message, pos }`. Un crate/modulo condiviso con un trait `Diagnostic` + `emit` eviterebbe duplicazione e semplificherebbe l'integrazione `miette`. | Ora (Block 3 completo) |
| Codici errore strutturati | Aggiungere `code: &'static str` agli errori: `L001` unexpected char, `P001` missing separator, `P002` orphan else, `A101`–`A104` (CLT101–104). Permette test sul codice invece che sulla stringa, documentazione linkabile, soppressione selettiva. | Ora (Block 3 completo) |
| Unsafe validation (CLT105/106) — **priorità alta** | Selling point principale del POC. Manca il supporto nel lexer/parser: `<unsafe reason="...">` e `unsafe('val', 'reason')` non sono tokenizzati. Richiede un mini-blocco parser (UnsafeBlock + UnsafeValue nell'AST), poi CLT105/106 nell'analyzer. | Appena possibile — sblocca prima di Block 4 |
| Span multi-token (`start..end`) | `Position` tiene solo `{ line, col }` del token iniziale. Un `Span { start: Position, end: Position }` permetterebbe sottolineare range di testo negli errori (`miette` lo supporta nativamente). | Quando si integra `miette` (Block 5) |

---

## Lexer

| Item | Dettaglio | Quando |
|------|-----------|--------|
| `emit` nel lexer | Vedi sopra. | Qualsiasi momento |
| Test sui messaggi di errore esatti | I test del lexer verificano solo la presenza di errori, non il testo. Allineare allo stile del parser (es. `assert_eq!(errors[0].message, "…")`). | Prima di Block 4 |

---

## Parser

| Item | Dettaglio | Quando |
|------|-----------|--------|
| `expect_emit` helper | Oggi `expect` ritorna `Result`; i chiamanti fanno `if let Err(e) = … { self.emit(…) }`. Un `expect_emit` che emette e ritorna `Option<Token>` ridurrebbe il boilerplate nei casi in cui non si vuole propagare. | Qualsiasi momento |
| Recovery più robusta in `parse_props` | Il recovery su prop malformata avanza fino al prossimo `Whitespace`. Potrebbe essere più preciso: saltare fino al token che inizia sicuramente la prop successiva o la chiusura del tag. | Prima di Block 4 |

---

## Analyzer

| Item | Dettaglio | Quando |
|------|-----------|--------|
| Prop map dinamica / componenti custom | La prop map è hardcoded per il POC. Questioni aperte: dove si dichiarano nuovi componenti built-in? Come si mappa un componente custom (`component Card(props) {}`) alle token category? La mappa può essere caricata da file o rimane sempre codice Rust? Discutere prima di Block 4. | Prima di Block 4 |
| `extract_identifiers` — limitazione regex shallow | `extract_identifiers` scansiona il logic block con `split_whitespace` + pattern matching sul token precedente. Falsi negativi su: destructuring (`const { a, b } = …`), import (`import foo from …`), alias di tipo, variabili di closure. Limitazione nota, accettabile per il POC. | Quando si decide di supportare TypeScript più completo |

---

## Tooling / qualità

| Item | Dettaglio | Quando |
|------|-----------|--------|
| Integrazione `miette` | Prevista al Block 5 (CLI). Richiederà che `LexError`, `ParseError` e `AnalyzerError` implementino il trait `Diagnostic` di `miette`. | Block 5 |
| Fixture più ricchi | `fixtures/` copre i casi base. Prima del Block 4 aggiungere fixture per edge case reali: props con espressioni complesse, `<each>` annidato in `<if>`, logica TypeScript non vuota. | Prima di Block 4 |
| Benchmark con `criterion` | Nessuna misura di performance. Aggiungere un benchmark sul lexer prima di Block 5 per avere una baseline e accorgersi di regressioni. | Prima di Block 5 |
