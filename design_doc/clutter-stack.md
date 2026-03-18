# CLUTTER — Stack Tecnologico

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Panoramica

Il compilatore Clutter è scritto in Rust. La scelta non è estetica: un compilatore è manipolazione di stringhe, trasformazione di strutture dati ad albero, e produzione di output deterministico — esattamente il dominio in cui Rust eccelle. Ownership chiara, zero garbage collector, performance nativa, e un ecosistema di librerie mature per questo tipo di problemi.

---

## Struttura del progetto

Workspace Cargo con crate separati per ogni blocco funzionale. Ogni crate è compilabile, testabile e sviluppabile in isolamento.

```
clutter/
├── Cargo.toml              ← workspace root
├── crates/
│   ├── clutter-lexer/      ← Lexer
│   ├── clutter-parser/     ← Parser + definizione AST
│   ├── clutter-analyzer/   ← Semantic Analyzer
│   ├── clutter-codegen/    ← Code Generator + target
│   ├── clutter-runtime/    ← definizioni runtime (tipi condivisi)
│   └── clutter-cli/        ← CLI, entry point del binario
├── tests/                  ← integration tests end-to-end
└── fixtures/               ← file .clutter di esempio per i test
```

### Dipendenze tra crate

```
clutter-cli
    ↓
clutter-codegen  ←  clutter-analyzer  ←  clutter-parser  ←  clutter-lexer
                                      ↑
                               clutter-runtime
                          (tipi condivisi: token, errori, posizioni)
```

`clutter-runtime` non è il runtime dell'output — è il crate che contiene i tipi condivisi tra i blocchi: la definizione dei token di design system, la struttura degli errori, le posizioni nel sorgente. Il nome è da rivedere per evitare ambiguità con il "runtime" dell'output discusso nel documento Block 5.

### TDD di default

Rust rende il TDD naturale: ogni crate ha il suo modulo `tests` interno, i test si scrivono nello stesso file del codice che testano, e `cargo test` li esegue tutti in parallelo senza configurazione aggiuntiva.

```rust
// Dentro clutter-lexer/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizza_open_tag() {
        let tokens = lex("<Column gap=\"md\">");
        assert_eq!(tokens[0].kind, TokenKind::OpenTag);
        assert_eq!(tokens[0].value, "Column");
    }
}
```

Ogni blocco si sviluppa test-first: si scrive il test che descrive il comportamento atteso, poi l'implementazione che lo fa passare. I test di integrazione in `tests/` esercitano la pipeline completa con file `.clutter` reali.

---

## Rappresentazione dell'AST: Arena Allocation

### Il problema degli alberi in Rust

In un linguaggio con garbage collector (JS, Java, Go) un nodo dell'AST può contenere riferimenti ai suoi figli senza problemi — il GC si occupa di gestire la memoria. In Rust, ogni valore ha un proprietario unico. Un nodo che contiene altri nodi, che a loro volta contengono altri nodi, crea strutture ricorsive che il compilatore Rust fatica a gestire con puntatori diretti.

La soluzione naive — `Box<Node>` o `Rc<RefCell<Node>>` — funziona ma è verbosa, lenta su alberi grandi, e produce codice difficile da leggere.

### Arena allocation

Un'**arena** è un blocco di memoria preallocato in cui tutti i nodi dell'AST vengono allocati sequenzialmente. Invece di puntatori tra nodi, si usano **indici** — un nodo "figlio" è semplicemente il numero della sua posizione nell'arena.

```
Arena: [Node0, Node1, Node2, Node3, Node4, ...]
                                        ↑
         Node1.children = [2, 4]  ←  indici, non puntatori
```

Vantaggi:
- **Performance**: tutti i nodi sono contigui in memoria, il cache del processore li legge in sequenza senza salti
- **Ownership semplice**: l'arena è l'unico proprietario di tutti i nodi — nessun problema di lifetime o riferimenti circolari
- **Allocazione e deallocazione in blocco**: quando la compilazione finisce, si libera l'intera arena in un'operazione sola
- **Standard nei compilatori**: è l'approccio usato da `rustc` stesso, da `rust-analyzer`, e da tutti i compilatori Rust seri

### Crate di riferimento

Per il POC si usa **`typed-arena`** — una arena tipizzata semplice, stabile, con ottima documentazione.

Per un'implementazione più matura (post-POC), **`rowan`** è la libreria usata da `rust-analyzer` e dalla maggior parte dei language server Rust. Offre un sistema di nodi tipizzati, green tree immutabili, e supporto nativo per operazioni incrementali (utile quando arriverà il LSP).

### Approfondimenti

- [Rust Book — Smart Pointers](https://doc.rust-lang.org/book/ch15-00-smart-pointers.html) — prerequisito per capire perché l'arena risolve un problema reale
- [typed-arena](https://docs.rs/typed-arena) — crate usato nel POC
- [rowan](https://github.com/rust-analyzer/rowan) — crate usato da rust-analyzer, riferimento per implementazione matura
- [Rustc Dev Guide — The HIR](https://rustc-dev-guide.rust-lang.org/hir.html) — come il compilatore Rust stesso gestisce il suo AST interno

---

## Formato tokens.clutter

Il design system è definito in un file `tokens.clutter` in formato **JSON**.

### Motivazioni

- Universale — tutti gli strumenti lo leggono e lo scrivono
- Figma può esportare JSON direttamente, con plugin o via API
- Script di migrazione da design system esistenti sono banali da scrivere
- Supporto Rust eccellente via `serde_json`

### Struttura

```json
{
  "spacing": {
    "xs":  4,
    "sm":  8,
    "md":  16,
    "lg":  24,
    "xl":  32,
    "xxl": 48
  },
  "colors": {
    "primary":    "#007AFF",
    "secondary":  "#5856D6",
    "danger":     "#FF3B30",
    "surface":    "#F2F2F7",
    "background": "#FFFFFF",
    "text": {
      "primary":   "#000000",
      "secondary": "#3C3C43",
      "tertiary":  "#8E8E93"
    }
  },
  "typography": {
    "sizes":   { "xs": 12, "sm": 14, "base": 16, "lg": 18, "xl": 24, "xxl": 32 },
    "weights": { "normal": 400, "medium": 500, "semibold": 600, "bold": 700 },
    "lineHeights": { "tight": 1.2, "normal": 1.5, "relaxed": 1.75 }
  },
  "radii": {
    "none": 0, "sm": 4, "md": 8, "lg": 16, "full": 9999
  },
  "shadows": {
    "sm": "0 1px 2px 0 rgb(0 0 0 / 0.05)",
    "md": "0 4px 6px -1px rgb(0 0 0 / 0.1)",
    "lg": "0 10px 15px -3px rgb(0 0 0 / 0.1)"
  },
  "breakpoints": {
    "mobile": 640, "tablet": 768, "desktop": 1024, "wide": 1280
  }
}
```

Il Semantic Analyzer carica questo file all'avvio e costruisce la mappa interna `prop → valori validi`. Se il file è malformato o mancante, la CLI produce un errore esplicito prima di tentare qualsiasi compilazione.

---

## unsafe nel template

### Motivazione

La rigidità del vocabolario chiuso è il valore principale di Clutter — ma imporre rigidità assoluta senza via d'uscita è una scelta che rallenta l'adozione. Integrazioni con componenti di terze parti, casi edge del design system, codice legacy da wrappare: situazioni reali che esistono in qualsiasi progetto.

La soluzione non è ammorbidire le regole — è rendere le eccezioni **esplicite, visibili, e documentate**.

`unsafe` nel template di Clutter funziona come `unsafe` in Rust: non è un errore, è una dichiarazione consapevole che si sta uscendo dalle garanzie del sistema. Il codice compila, ma il developer — e chiunque faccia code review — vede immediatamente che lì le regole non si applicano, e perché.

### Due forme di unsafe

**Blocco unsafe** — per markup arbitrario fuori dal vocabolario Clutter.

Il tag `<unsafe>` richiede un attributo `reason` obbligatorio. Se manca, errore di compilazione — non warning, errore. Un `unsafe` senza spiegazione è peggio di nessun `unsafe`: dà falsa sicurezza senza lasciare traccia del debito.

```
<Column gap="md">
  <Text size="lg">Contenuto normale</Text>

  <unsafe reason="il componente DatePicker di terze parti non ha ancora un wrapper Clutter">
    <div class="legacy-datepicker">
      ...
    </div>
  </unsafe>

  <Button variant="primary">OK</Button>
</Column>
```

**Valore unsafe** — per valori custom su props che si aspettano token del design system.

La funzione `unsafe()` accetta il valore custom come primo argomento e un commento obbligatorio come secondo. Se il secondo argomento manca, errore di compilazione.

```
<Column gap={unsafe('16px', 'spaziatura non standard richiesta dal layout di stampa, risolvere con token print-spacing in v2')}>
  ...
</Column>
```

### Comportamento del compilatore

- Il Semantic Analyzer ignora il contenuto di `<unsafe>` e i valori `unsafe()` — non valida, non controlla
- Il Semantic Analyzer **verifica** che `reason` e il secondo argomento di `unsafe()` siano presenti e non vuoti — errore di compilazione altrimenti
- Il Code Generator copia il contenuto di `<unsafe>` verbatim nell'output, e inserisce il valore di `unsafe()` direttamente nella prop
- La CLI riporta tutti i blocchi unsafe con i loro commenti:

```
✓ Card.clutter → Card.vue (12ms)

  2 blocchi unsafe:
  - riga 8  [tag]   "il componente DatePicker non ha ancora un wrapper Clutter"
  - riga 23 [valore] "spaziatura non standard per layout di stampa, risolvere con token print-spacing in v2"
```

- In futuro: un flag `--no-unsafe` che fa fallire la compilazione se sono presenti blocchi `<unsafe>` o valori `unsafe()`, utile per CI/CD su branch di produzione

### unsafe nella sezione logica

La sezione logica è già TypeScript arbitrario — per definizione non ha le restrizioni del template. Non serve una keyword `unsafe` lì: il developer scrive quello che vuole, e il Code Generator la inserisce invariata nell'output.

### Selling point

`unsafe` trasforma il debito tecnico da implicito a esplicito e documentato. In un codebase Clutter:

- `grep unsafe` mostra esattamente dove e quante volte il design system è stato aggirato
- Ogni occorrenza porta con sé la spiegazione del perché e — idealmente — cosa serve per risolverla
- Il report della CLI a ogni build tiene il team consapevole del debito esistente

Questo è un vantaggio strutturale rispetto a Tailwind o CSS puro, dove le eccezioni si nascondono silenziosamente tra centinaia di classi o in file `.css` sparsi senza lasciare traccia.

---

## Dipendenze Rust

| Crate | Versione | Uso |
|---|---|---|
| `clap` | 4.x | CLI — argument parsing |
| `miette` | 5.x | Error reporting con highlight del sorgente |
| `serde` + `serde_json` | 1.x | Deserializzazione di `tokens.clutter` |
| `typed-arena` | 2.x | Arena allocation per l'AST |
| `rowan` | 0.15.x | Post-POC — AST per il LSP |

Nessuna dipendenza per il Lexer, il Parser, il Semantic Analyzer, e il Code Generator — sono algoritmi puri su strutture dati, zero librerie esterne necessarie.

---

## Dipendenze output (non Rust)

| Dipendenza | Target | Uso |
|---|---|---|
| Alpine.js | HTML (POC) | Runtime reattivo |
| `@vue/reactivity` | HTML (a regime) | Runtime reattivo proprietario |
| Vue / Nuxt | Vue SFC | Runtime dell'applicazione host |

---

## Riferimenti

- [The Rust Programming Language](https://doc.rust-lang.org/book/) — riferimento primario
- [Rust Book — Workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) — struttura multi-crate
- [clap](https://docs.rs/clap) — CLI
- [miette](https://docs.rs/miette) — error reporting
- [serde_json](https://docs.rs/serde_json) — parsing JSON
- [typed-arena](https://docs.rs/typed-arena) — arena allocation POC
- [rowan](https://github.com/rust-analyzer/rowan) — arena + AST per LSP
- [Crafting Interpreters](https://craftinginterpreters.com) — riferimento generale compilatori

---

*End of Document*
