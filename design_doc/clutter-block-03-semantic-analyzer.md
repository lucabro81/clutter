# CLUTTER — Block 3: Semantic Analyzer

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Cos'è il Semantic Analyzer e perché esiste

Il Lexer ha riconosciuto i caratteri. Il Parser ha verificato che la struttura sintattica sia corretta — i tag sono bilanciati, le props hanno la forma giusta. Ma nessuno dei due sa ancora se quello che è scritto ha senso.

Il Semantic Analyzer è il blocco che risponde alla domanda: **questo codice sintatticamente corretto significa qualcosa di valido?**

Esempio concreto: questo template passa il Lexer e il Parser senza problemi —

```
<Column gap="xl2" color="banana">
  <Text>{pippo}</Text>
</Column>
```

È sintatticamente perfetto. Ma:
- `xl2` non esiste nello spacing del design system
- `color` non è una prop di `Column`
- `banana` non esiste nei colori del design system
- `pippo` non è una variabile dichiarata nella sezione logica

Nessuno di questi errori è rilevabile dal Parser. Sono errori di **significato**, non di struttura. Il Semantic Analyzer li trova tutti.

Questo è il blocco che dimostra il valore del progetto. Zero CSS, zero configurazioni, zero convenzioni da ricordare — se scrivi qualcosa di non valido, il compilatore te lo dice prima che il codice esista.

---

## Compiti del Semantic Analyzer

Il Semantic Analyzer ha tre responsabilità separate, che è utile tenere concettualmente distinte:

**1. Type checking delle props** — verifica che ogni prop di ogni componente riceva un valore presente nel design system (`tokens.clutter`)

**2. Reference checking** — verifica che ogni espressione `{variabile}` nel template corrisponda a un identificatore dichiarato nella sezione logica

**3. Validazione degli unsafe** — verifica che ogni uso di `unsafe` sia accompagnato da un commento obbligatorio non vuoto

Sono tre problemi distinti con sorgenti di verità diverse: i token del design system per il primo, la sezione logica per il secondo, le regole sintattiche di Clutter per il terzo.

---

## Sorgente 1: tokens.clutter

`tokens.clutter` è il design system — l'unica fonte di verità per i valori validi. Il Semantic Analyzer lo carica prima di analizzare qualsiasi file.

Struttura attesa (formato da definire nello stack, per il documento usiamo JSON come riferimento):

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

Da `tokens.clutter` il Semantic Analyzer costruisce internamente una mappa che associa ogni prop di ogni componente al set di valori validi. Questa mappa è la base del type checking.

---

## La mappa props → token

Per ogni componente built-in, il Semantic Analyzer conosce quali props esistono e a quale categoria di token appartengono.

Esempio per il POC:

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

Questa mappa è hardcoded per i componenti built-in nel POC. In futuro, con componenti custom, sarà generata dinamicamente dalla definizione del componente.

---

## Come funziona internamente

Il Semantic Analyzer percorre l'AST con una visita ricorsiva — parte dalla radice (`ProgramNode`) e scende verso le foglie, analizzando ogni nodo.

### Pseudologica della visita

```
function analyzeProgram(node: ProgramNode):
  identifiers = extractIdentifiers(node.logicBlock)
  analyzeTemplate(node.template, identifiers)

function analyzeTemplate(node, identifiers):
  per ogni figlio di node:
    se è ComponentNode:
      analyzeComponent(node, identifiers)
    se è ExpressionNode:
      analyzeExpression(node, identifiers)
    se è IfNode o EachNode:
      analyzeControlFlow(node, identifiers)
    se è UnsafeBlockNode:
      analyzeUnsafeBlock(node)

function analyzeComponent(node: ComponentNode, identifiers):
  verifica che node.name sia un componente built-in o importato
  
  per ogni prop in node.props:
    verifica che prop.name esista nella mappa per node.name
    
    se prop.value è StringValue:
      verifica che prop.value.value sia nel set di valori validi per quella prop
    
    se prop.value è ExpressionValue:
      verifica che il nome nell'espressione esista in identifiers
    
    se prop.value è UnsafeValue:
      analyzeUnsafeValue(prop.value)
  
  per ogni figlio in node.children:
    analyzeTemplate(figlio, identifiers)

function analyzeExpression(node: ExpressionNode, identifiers):
  verifica che node.name esista in identifiers

function analyzeUnsafeBlock(node: UnsafeBlockNode):
  se node.reason è assente o stringa vuota:
    emetti errore CLT105
  — il contenuto del blocco non viene analizzato

function analyzeUnsafeValue(node: UnsafeValue):
  se node.reason è assente o stringa vuota:
    emetti errore CLT106
  — il valore custom non viene validato contro i token
```

### Estrazione degli identificatori dalla sezione logica

La sezione logica è TypeScript grezzo — non viene parsata completamente. Per il POC è sufficiente un'analisi superficiale: raccogliere tutti i nomi che seguono `const`, `let`, `var`, `function`, e la keyword custom `component`.

```
const title = "Ciao"       → identifiers: ["title"]
let count = 0              → identifiers: ["title", "count"]
function handleClick() {}  → identifiers: ["title", "count", "handleClick"]
component Card(props) {}   → identifiers: ["title", "count", "handleClick", "Card"]
```

Questo approccio ha falsi negativi (es. destructuring, import) — per il POC è accettabile. Documentato come limitazione nota.

---

## Gli errori semantici

Gli errori sono il prodotto principale del Semantic Analyzer — non solo il fatto che qualcosa sia sbagliato, ma il perché e come correggerlo.

### Tipologia di errori

**Prop sconosciuta**
```
Errore [CLT101] — riga 3, colonna 5
La prop 'color' non esiste sul componente 'Column'.
```

**Valore non nel design system**
```
Errore [CLT102] — riga 4, colonna 12
Valore 'xl2' non valido per la prop 'gap' di 'Column'.
Valori validi: xs, sm, md, lg, xl, xxl
```

**Componente sconosciuto**
```
Errore [CLT103] — riga 7, colonna 3
Componente 'Griglia' non riconosciuto.
Componenti disponibili: Column, Row, Box, Text, Button, Input
```

**Riferimento a variabile non dichiarata**
```
Errore [CLT104] — riga 9, colonna 8
'{pippo}' non è dichiarato nella sezione logica.
```

**Blocco unsafe senza reason**
```
Errore [CLT105] — riga 12, colonna 3
<unsafe> richiede un attributo 'reason' non vuoto.
Esempio: <unsafe reason="componente legacy, sostituire con Wrapper in v2">
```

**Valore unsafe senza reason**
```
Errore [CLT106] — riga 15, colonna 10
unsafe() richiede un secondo argomento con la motivazione.
Esempio: unsafe('16px', 'spaziatura non standard, aggiungere token print-spacing')
```

### Struttura di un errore

```
ErrorNode
  code:     string     — codice errore (CLT101, CLT102, ...)
  message:  string     — descrizione human-readable
  hint:     string?    — suggerimento opzionale ("Valori validi: ...")
  position: Position   — riga e colonna nel sorgente
  severity: "error"    — per il POC tutti gli errori bloccano la compilazione
           | "warning" — futuro: warning non bloccanti
```

### Raccolta degli errori

Come il Lexer e il Parser, il Semantic Analyzer non si ferma al primo errore. Continua la visita dell'AST raccogliendo tutti gli errori trovati, poi li riporta tutti insieme alla fine.

L'unica eccezione: se un componente è sconosciuto (CLT103), non ha senso analizzare le sue props — non sappiamo quali siano valide. In questo caso si salta l'analisi delle props di quel nodo e si continua con i fratelli.

---

## Warning vs Error

Per il POC, tutto è un errore bloccante — se il Semantic Analyzer trova problemi, il Code Generator non viene eseguito.

In futuro ha senso distinguere:

- **Error**: valore non nel design system, componente sconosciuto, variabile non dichiarata → blocca la compilazione
- **Warning**: prop ridondante (es. `gap` su un componente senza figli), pattern sconsigliato → compila ma avvisa

Per ora questa distinzione è fuori scope.

---

## Input e Output del blocco

**Input**: AST prodotto dal Parser + file `tokens.clutter`

**Output**: uno di due casi possibili

*Caso 1 — nessun errore*: l'AST originale viene passato al Code Generator invariato (o annotato con informazioni aggiuntive raccolte durante l'analisi)

*Caso 2 — errori trovati*: lista di `ErrorNode` con codice, messaggio, hint e posizione. La compilazione si ferma qui, il Code Generator non viene eseguito.

```
// Caso 1
{
  success: true,
  ast: ProgramNode { ... }
}

// Caso 2
{
  success: false,
  errors: [
    { code: "CLT102", message: "Valore 'xl2' non valido...", hint: "Valori validi: xs, sm, ...", position: { line: 4, col: 12 } },
    { code: "CLT104", message: "'{pippo}' non è dichiarato...", position: { line: 9, col: 8 } }
  ]
}
```

---

## Come testare il Semantic Analyzer

Il Semantic Analyzer si testa fornendo direttamente un AST — non serve rieseguire Lexer e Parser ogni volta. Questo rende i test veloci e precisi.

Casi da coprire:

**Validi (nessun errore atteso)**
- Componente con tutte le props corrette
- Componente senza props
- Espressione che referenzia una variabile dichiarata
- Componenti annidati tutti validi

**Errori attesi**
- Prop con valore non nel design system → CLT102
- Prop sconosciuta per quel componente → CLT101
- Componente non riconosciuto → CLT103
- Espressione con variabile non dichiarata → CLT104
- Blocco `<unsafe>` senza attributo `reason` → CLT105
- Blocco `<unsafe>` con `reason` vuoto → CLT105
- Valore `unsafe()` senza secondo argomento → CLT106
- Valore `unsafe()` con secondo argomento vuoto → CLT106
- File con più errori → tutti riportati, non solo il primo

**Unsafe validi (nessun errore atteso)**
- Blocco `<unsafe reason="...">` con reason non vuoto → compila, contenuto ignorato
- Valore `unsafe('16px', 'motivazione...')` con entrambi gli argomenti → compila, valore passato verbatim

---

## Una nota sul design system come tipo

Vale la pena fermarsi un momento su questa cosa.

In un'applicazione normale, il design system è documentazione — una lista di valori da rispettare per convenzione. Se scrivi `gap: 17px` invece di `gap: var(--spacing-md)`, nessuno te lo impedisce. Lo scopri in code review, o non lo scopri mai.

In Clutter, il design system è il sistema di tipi. `gap="xl2"` non è una convenzione violata — è un errore di compilazione, esattamente come passare una stringa dove un compilatore si aspetta un numero.

Questo è il salto concettuale che giustifica tutta la complessità del progetto. Il Semantic Analyzer è il punto dove quel salto diventa concreto.

---

## Riferimenti

- [Crafting Interpreters](https://craftinginterpreters.com) — Cap. 11 (Resolving and Binding) — gestione degli scope e risoluzione dei riferimenti
- [Writing a Compiler in Go](https://compilerbook.com) — Cap. 4 (Symbol Table) — gestione degli identificatori e scope
- Codice sorgente TypeScript → `src/compiler/checker.ts` — il type checker di TypeScript, riferimento per capire la scala del problema (non da leggere per intero, ma utile per vedere come è strutturato un semantic analyzer reale)

---

*End of Document*
