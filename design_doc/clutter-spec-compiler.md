# CLUTTER — Compiler Approach Specification

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Executive Summary

Clutter è un linguaggio di markup per UI con compilatore dedicato. I file `.clutter` vengono transpilati verso un target configurabile (Vue SFC, vanilla JS/HTML, altri). Il developer non scrive CSS, non configura toolchain, non gestisce dipendenze di ecosistema — scrive struttura e logica, il compilatore fa il resto.

Obiettivo finale: formato ideale per sviluppo human e LLM-first, con vocabolario chiuso e zero ambiguità.

---

## Table of Contents

1. [Rationale](#rationale)
2. [File Format](#file-format)
3. [Template Syntax](#template-syntax)
4. [Type System & Design Tokens](#type-system--design-tokens)
5. [Component Model](#component-model)
6. [Compiler Architecture](#compiler-architecture)
7. [Runtime](#runtime)
8. [Compilation Targets](#compilation-targets)
9. [Unsafe Escape Hatch](#unsafe-escape-hatch)
10. [Tooling](#tooling)
11. [Known Limitations & Tradeoffs](#known-limitations--tradeoffs)
12. [Future Extensions](#future-extensions)

---

## Rationale

### Perché non una libreria Vue

L'approccio "componenti Vue con props tipizzate" risolve il problema del CSS manuale, ma lascia intatto tutto il resto:

- Il developer è ancora dentro l'ecosistema Vue con le sue regole
- TypeScript è obbligatorio e porta con sé tsconfig, strict mode, versioning
- Gli errori sono errori TypeScript, non errori Clutter
- Aggiungere feature (plugin accessibilità, test by design, ecc.) richiede workaround sull'ecosistema esistente
- L'LLM deve conoscere Vue, TypeScript, Vite, e Clutter — vocabolario troppo ampio

### Perché un compilatore custom

- **Errori semantici**: "colore 'blu' non esiste nel design system" invece di type errors generici
- **Vocabolario chiuso**: il template accetta solo costrutti Clutter — niente da inventare, niente da dimenticare
- **Plugin architecture nativa**: accessibilità, test id, linting sono hook del compilatore, non patch sull'ecosistema
- **LLM-first**: formato prevedibile, struttura fissa, zero configurazione implicita — addestrabile con pochissimi esempi
- **Target multipli**: lo stesso `.clutter` può compilare a Vue per app esistenti o vanilla per progetti nuovi

### Cosa si perde

- Compatibilità immediata con tooling Vue (Volar, Vue DevTools, ecc.)
- Ecosistema di componenti Vue di terze parti
- Familiarità del team con il formato

Questi sono costi reali, non ignorabili — vedi sezione [Known Limitations](#known-limitations--tradeoffs).

---

## File Format

Un file `.clutter` è composto da due sezioni separate da un delimitatore esplicito.

```
[sezione logica — TypeScript standard]

---

[sezione template — sintassi JSX-like con vocabolario chiuso]
```

### Regole

- La sezione logica è TypeScript valido, nessuna sintassi custom
- Il separatore `---` è obbligatorio anche se la sezione logica è vuota
- La sezione template non accetta TypeScript arbitrario — solo riferimenti a variabili definite nella sezione logica
- Un file `.clutter` definisce esattamente un componente radice

### Esempio minimo

```
const title = "Ciao"
const handleClick = () => console.log("clicked")

---

<Column gap="md" padding="lg">
  <Text size="xl" weight="bold">{title}</Text>
  <Button variant="primary" onClick={handleClick}>Clicca</Button>
</Column>
```

---

## Template Syntax

### Principi

- Sintassi JSX-like: tag, props, children
- Solo componenti built-in o componenti `.clutter` importati esplicitamente
- Props accettano solo valori dal design system o riferimenti a variabili della sezione logica
- Nessuna espressione JS arbitraria nel template — le espressioni complesse vanno calcolate nella sezione logica e passate come variabili

### Componenti built-in

Disponibili senza import, fanno parte del linguaggio:

| Componente | Scopo |
|---|---|
| `Column` | Flex column |
| `Row` | Flex row |
| `Box` | Contenitore generico |
| `Text` | Testo tipografato |
| `Button` | Azione interattiva |
| `Input` | Campo di input |
| `Image` | Immagine con dimensioni token-based |

### Props

Le props accettano:
- Valori letterali dal design system: `gap="md"`, `color="primary"`
- Riferimenti a variabili dalla sezione logica: `{myVariable}`
- Boolean shorthand: `disabled` equivale a `disabled={true}`

Le props **non** accettano:
- Valori arbitrari non presenti nei token: `gap="17px"` → errore di compilazione
- Espressioni JS inline: `gap={isLarge ? "lg" : "sm"}` → va calcolato nella sezione logica

### Rendering condizionale e liste

```
// Condizionale — parola chiave dedicata, non espressione JS
<Column>
  <if condition={isLoggedIn}>
    <Text>Benvenuto</Text>
  </if>
  <else>
    <Button variant="primary">Login</Button>
  </else>
</Column>

// Liste
<Column gap="sm">
  <each item={products} as="product">
    <ProductCard product={product} />
  </each>
</Column>
```

### Componenti locali

È possibile definire sotto-componenti nello stesso file, prima del separatore `---`:

```
component ProductCard(product: Product) {
  <Box bg="surface" padding="md" radius="md">
    <Column gap="sm">
      <Text weight="bold">{product.name}</Text>
      <Text color="secondary">{product.price}</Text>
    </Column>
  </Box>
}

---

<Column gap="md">
  <each item={products} as="product">
    <ProductCard product={product} />
  </each>
</Column>
```

---

## Type System & Design Tokens

### tokens.clutter

File di configurazione del design system, unica sorgente di verità. Formato da definire (JSON, TOML, o DSL custom) — priorità: leggibile da umano e da LLM, non necessariamente TypeScript.

**Categorie**:
- `colors` — scala semantica e neutri
- `spacing` — scala dimensionale
- `typography` — sizes, weights, lineHeights
- `radii` — border radius
- `shadows` — presets ombra
- `breakpoints` — valori responsive

### Enforcement

Il compilatore legge `tokens.clutter` e:
- Genera i tipi validi per ogni prop
- Produce errori espliciti per valori non presenti nei token
- Gli errori riportano il valore usato e i valori validi disponibili

### Errori semantici (esempio)

```
Errore [CLT001] — riga 4, colonna 12
Valore 'xl2' non esiste per la prop 'gap'.
Valori validi: xs, sm, md, lg, xl, xxl
```

---

## Component Model

### Importazione componenti esterni

```
import ProductCard from "./ProductCard.clutter"
import { Modal, Drawer } from "./overlays.clutter"
```

Solo file `.clutter` — nessun import di componenti Vue o JS arbitrari (eccetto via `unsafe`).

### Props di un componente

Definite nella sezione logica con sintassi TypeScript standard:

```
interface Props {
  title: string
  variant?: "primary" | "secondary"
  onClick: () => void
}

const props = defineProps<Props>()

---

<Button variant={props.variant} onClick={props.onClick}>
  {props.title}
</Button>
```

### Stato locale

```
import { reactive } from "clutter"

const count = reactive(0)
const increment = () => count.value++

---

<Row gap="sm" crossAxis="center">
  <Text>{count.value}</Text>
  <Button variant="primary" onClick={increment}>+</Button>
</Row>
```

`clutter` espone una API reattiva minima — non l'intera API di Vue o React.

---

## Compiler Architecture

### Pipeline

```
File .clutter
     ↓
  Lexer
     ↓
  Parser → AST
     ↓
  Semantic Analyzer
  (valida token, props, riferimenti)
     ↓
  Plugin hooks
  (accessibilità, test id, ecc.)
     ↓
  Code Generator
     ↓
  Output (Vue SFC | Vanilla | ...)
```

### Fasi principali

**Lexer**: tokenizza il file, distingue le due sezioni, identifica tag, props, espressioni

**Parser**: costruisce l'AST del template; la sezione logica viene trattata come TypeScript opaco e passata al code generator invariata (o quasi)

**Semantic Analyzer**:
- Verifica che ogni prop riceva un valore valido dal design system
- Verifica che i riferimenti `{variabile}` esistano nella sezione logica
- Verifica che i componenti usati siano built-in o importati esplicitamente

**Plugin hooks**: punti di estensione nella pipeline, prima della generazione del codice

**Code Generator**: percorre l'AST e produce il target selezionato

### Stack consigliato per il compilatore

Da valutare — opzioni principali:

| Opzione | Pro | Contro |
|---|---|---|
| TypeScript | Ecosistema, facilità di distribuzione npm | Performance su file grandi |
| Rust | Performance, Luca lo sta imparando, WASM-friendly | Tempi di sviluppo iniziali più lunghi |
| Go | Performance, compilazione veloce | Ecosistema npm più complesso |

Rust è l'opzione più ambiziosa e coerente con gli obiettivi a lungo termine (WASM, tool distribuzione), ma richiede più tempo iniziale.

---

## Runtime

I componenti built-in (`Column`, `Row`, `Box`, ecc.) sono implementati nel runtime Clutter — codice JS/TS compilato, opaco all'utente, distribuito come parte del pacchetto.

### Principi del runtime

- Nessuna dipendenza esterna a runtime (no Vue, no React)
- Dimensioni minime — solo ciò che serve ai componenti built-in
- API reattiva minima esposta alla sezione logica via `import { reactive, computed } from "clutter"`
- Compatibilità con i target di compilazione supportati

### Quando il target è Vue

Il runtime non serve — i componenti built-in vengono compilati come componenti Vue validi, e il runtime Vue gestisce la reattività. La sezione logica usa direttamente l'API Vue (`ref`, `computed`, ecc.).

---

## Compilation Targets

### Vue SFC

Output: file `.vue` validi, integrabili in qualsiasi app Vue/Nuxt esistente.

Caso d'uso: migrazione graduale di un'app Vue esistente (es. Comperio).

```
// Input: ProductCard.clutter
// Output: ProductCard.vue (Vue SFC valido)
```

### Vanilla JS + HTML

Output: Web Components standard o HTML/JS puri, senza dipendenze framework.

Caso d'uso: progetti nuovi, embedding in contesti non-Vue.

### Altri target (futuro)

Plugin architecture permette target aggiuntivi senza modificare il compilatore core.

---

## Unsafe Escape Hatch

Per i casi edge inevitabili — componenti di terze parti, integrazioni legacy, casi non coperti dal DSL.

### Nel template

```
<Row gap="md">
  <Text>Contenuto normale</Text>
  <unsafe>
    <div style="some-legacy-thing: value">...</div>
  </unsafe>
</Row>
```

### Nella sezione logica

Codice JS/TS arbitrario è già permesso nella sezione logica per definizione — l'`unsafe` nel template segnala esplicitamente che si sta uscendo dal vocabolario chiuso.

### Principi

- `unsafe` è visibile e deliberato — non è un workaround silenzioso
- Appare nei report del compilatore ("N blocchi unsafe nel progetto")
- È un segnale per code review, non un errore
- Non disabilita il controllo sulle parti circostanti

---

## Tooling

### Indispensabile prima del day-1

- **CLI**: `clutter build`, `clutter watch`, `clutter check` (solo validazione senza output)
- **Language Server (LSP)**: senza autocomplete nell'editor, la DX è peggiore di scrivere Vue a mano — non è opzionale
- **VS Code extension**: consumer del LSP, syntax highlighting, errori inline

### Utile post-MVP

- **Formatter**: stile consistente automatico
- **Plugin accessibilità**: warning a compile-time per pattern non accessibili
- **Plugin test**: generazione automatica di test id e asserzioni base
- **Figma plugin**: generazione `.clutter` da design

---

## Known Limitations & Tradeoffs

### Costi reali

| Problema | Impatto |
|---|---|
| Nessun Vue DevTools | Debug runtime più difficile |
| Nessun ecosistema componenti terze parti | Tutto va wrappato via `unsafe` o reimplementato |
| LSP da costruire da zero | Senza di esso la DX è pessima — è un prerequisito, non un nice-to-have |
| Parser/compilatore da mantenere | Superficie di bug aggiuntiva rispetto all'approccio libreria |
| Curva di apprendimento per il team | Nuovo formato, nuovi errori, nuova mentale |

### Scope del compilatore

Il parser gestisce il template `.clutter`. La sezione logica viene trattata come TypeScript opaco — il compilatore non fa type checking TypeScript, si limita a verificare che i riferimenti usati nel template esistano come identificatori nella sezione logica.

Per type checking completo della sezione logica serve TypeScript — che rimane opzionale e configurabile, ma non imposto.

---

## Future Extensions

- **Target React** — se necessario, aggiungibile come plugin target
- **Hot module replacement** — per dev experience fluida
- **Source maps** — per debug del codice compilato
- **WASM build del compilatore** — distribuzione zero-dipendenze, performance massima (coerente con percorso Rust)
- **Integrazione LLM** — il formato chiuso e prevedibile è ideale per fine-tuning su task di generazione UI

---

*End of Specification*
