# CLUTTER — Block 2: Parser

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Cos'è un Parser e perché esiste

Il Lexer ha trasformato il testo grezzo in una sequenza piatta di token. Il Parser prende quella sequenza e ci costruisce sopra una struttura ad albero — l'**AST** (Abstract Syntax Tree).

La differenza fondamentale è questa: il Lexer legge caratteri in sequenza senza mai "guardare indietro" o "guardare avanti" di molto. Il Parser invece deve capire le relazioni tra token — quali sono figli di quali, dove inizia e finisce un blocco, se una sequenza è valida rispetto alla grammatica del linguaggio.

Esempio: il Lexer vede questi token uno dopo l'altro e li classifica:

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

Il Parser capisce che `Text` è figlio di `Column`, che `gap="md"` è una prop di `Column`, e che tutto termina quando incontra `</Column>`.

---

## L'AST — Abstract Syntax Tree

L'AST è la rappresentazione del file `.clutter` come struttura dati. È un albero perché i componenti si annidano — ogni nodo può avere figli, e i figli possono avere figli.

"Abstract" significa che l'albero non contiene più i dettagli del testo sorgente (virgolette, parentesi angolari, spazi) — contiene solo la struttura e il significato estratto da quel testo.

Esempio: questo template:

```
<Column gap="md">
  <Text size="lg">Ciao</Text>
  <Button variant="primary">OK</Button>
</Column>
```

Produce questo AST:

```
ComponentNode "Column"
  props:
    gap = "md"
  children:
    ComponentNode "Text"
      props:
        size = "lg"
      children:
        TextNode "Ciao"
    ComponentNode "Button"
      props:
        variant = "primary"
      children:
        TextNode "OK"
```

Questo è l'oggetto che il Semantic Analyzer e il Code Generator ricevono e lavorano. Niente più stringhe, niente più token — struttura navigabile.

---

## I nodi dell'AST di Clutter

Per il POC, l'AST deve rappresentare questi tipi di nodo:

### Nodo radice

```
ProgramNode
  logicBlock: string       — il contenuto TypeScript grezzo della sezione logica
  template: TemplateNode   — la radice del template
```

Il `ProgramNode` è sempre la radice dell'albero. Contiene il blocco logico (trattato come stringa opaca) e il template.

### Nodi template

| Nodo | Descrizione |
|---|---|
| `ComponentNode` | Un componente (`<Column>`, `<Text>`, ecc.) con props e figli |
| `TextNode` | Testo statico tra tag |
| `ExpressionNode` | Riferimento a variabile `{title}` |
| `IfNode` | Blocco condizionale `<if condition={...}>` |
| `EachNode` | Iterazione `<each item={...} as="...">` |

### Struttura di un ComponentNode

```
ComponentNode
  name: string             — nome del componente ("Column", "Text", ...)
  props: PropNode[]        — lista delle props
  children: Node[]         — nodi figli
  position: Position       — riga e colonna nel sorgente

PropNode
  name: string             — nome della prop ("gap", "variant", ...)
  value: StringValue       — valore stringa letterale ("md", "primary")
        | ExpressionValue  — riferimento a variabile ({myVar})
  position: Position
```

---

## Come funziona internamente

Il Parser consuma i token uno alla volta, tenendo traccia di dove si trova nella struttura. La tecnica usata per Clutter è il **Recursive Descent Parser** — l'approccio più comune per linguaggi con sintassi relativamente semplice, usato da Babel, TypeScript, e dal compilatore Vue.

### Perché Recursive Descent

Esistono altri approcci (parser a tabella, parser generati automaticamente da una grammatica). Il Recursive Descent si scrive a mano, è semplice da capire e da debuggare, e si presta bene a produrre errori di qualità. Per un linguaggio nuovo con sintassi custom è la scelta naturale.

### Il principio

Ogni costrutto del linguaggio ha una funzione dedicata nel Parser. Quella funzione sa quali token si aspetta, li consuma in ordine, e chiama ricorsivamente le funzioni per i costrutti annidati.

Pseudocodica:

```
function parseComponent():
  consuma OPEN_TAG → prendo il nome del componente
  mentre il prossimo token è IDENTIFIER:
    chiama parseProp() → aggiungo la prop al nodo
  consuma CLOSE_TAG
  mentre il prossimo token non è CLOSE_OPEN_TAG:
    chiama parseNode() → aggiungo il figlio al nodo
  consuma CLOSE_OPEN_TAG
  restituisco il ComponentNode completo

function parseProp():
  consuma IDENTIFIER → prendo il nome della prop
  consuma EQUALS
  se il prossimo token è STRING:
    consuma STRING → prendo il valore
  se il prossimo token è EXPRESSION:
    consuma EXPRESSION → prendo il riferimento
  restituisco PropNode

function parseNode():
  se il prossimo token è OPEN_TAG:
    chiama parseComponent()
  se il prossimo token è TEXT:
    restituisco TextNode
  se il prossimo token è EXPRESSION:
    restituisco ExpressionNode
  se il prossimo token è IF_OPEN:
    chiama parseIf()
  se il prossimo token è EACH_OPEN:
    chiama parseEach()
```

La ricorsione avviene perché `parseComponent` chiama `parseNode`, e `parseNode` può chiamare `parseComponent` — questo è ciò che permette l'annidamento arbitrario dei componenti.

### Lookahead

Il Parser ha bisogno di "guardare avanti" di un token per decidere quale funzione chiamare — questa tecnica si chiama **lookahead**. Per Clutter è sufficiente un lookahead di 1 (si guarda solo il prossimo token, non due o tre avanti). Questo semplifica molto l'implementazione.

---

## Gestione della sezione logica

La sezione logica (TypeScript) non viene analizzata nel dettaglio dal Parser di Clutter. Viene raccolta come stringa grezza e inserita nel `ProgramNode` come `logicBlock`.

Il motivo è pragmatico: analizzare TypeScript correttamente è un problema già risolto dal compilatore TypeScript. Farlo di nuovo in Clutter sarebbe un lavoro enorme per un beneficio marginale nel POC.

L'unica cosa che il Parser deve fare con la sezione logica è identificare i nomi degli identificatori dichiarati — per permettere al Semantic Analyzer di verificare che i riferimenti `{variabile}` nel template esistano. Questo può essere fatto con un'analisi superficiale (raccogliere tutte le parole dopo `const`, `let`, `function`, `component`) senza un parser TypeScript completo.

---

## Gestione degli errori nel Parser

Come nel Lexer, l'obiettivo è raccogliere più errori possibili prima di fermarsi. La tecnica standard è il **panic mode recovery**: quando il Parser incontra un token inatteso, segnala l'errore e avanza fino a un punto di sincronizzazione noto (tipicamente la fine di un tag o del file), poi riprende l'analisi da lì.

Errori tipici da gestire:

- Tag aperto senza chiusura corrispondente
- Tag di chiusura senza apertura (`</Column>` senza `<Column>`)
- Prop senza `=` o senza valore
- Espressione `{` senza `}` di chiusura
- File senza separatore `---`

Per ogni errore: tipo, messaggio leggibile, posizione (dalla `position` del token coinvolto).

---

## Input e Output del blocco

**Input**: array di token prodotto dal Lexer

**Output**: AST — un oggetto `ProgramNode` che rappresenta l'intero file

```
ProgramNode {
  logicBlock: "const title = 'Ciao'\nconst handleClick = () => ...",
  template: ComponentNode {
    name: "Column",
    props: [
      PropNode { name: "gap", value: StringValue { value: "md" } }
    ],
    children: [
      ComponentNode {
        name: "Text",
        props: [
          PropNode { name: "size", value: StringValue { value: "lg" } }
        ],
        children: [
          TextNode { value: "Ciao" }
        ]
      }
    ]
  }
}
```

---

## Come testare il Parser

Il Parser si testa in isolamento partendo da token già prodotti — non serve rieseguire il Lexer ogni volta.

Casi da coprire:

- Template con componente singolo senza props
- Template con componente e props stringa
- Template con prop expression `{var}`
- Nesting a due livelli
- Nesting profondo (3+ livelli)
- Componente self-closing (`<Text />`)
- Blocco `<if>` con e senza `<else>`
- Blocco `<each>`
- Tag aperto senza chiusura → errore
- Prop senza valore → errore
- File senza `---` → errore

---

## Cosa il Parser non fa

Il Parser non sa se `gap="xl2"` è un valore valido per quella prop. Non conosce i token del design system. Non sa se `{pippo}` è una variabile che esiste davvero nella sezione logica.

Queste sono responsabilità del Semantic Analyzer.

Il Parser sa solo che la struttura sintattica è corretta — i tag sono bilanciati, le props hanno la forma giusta, le espressioni sono ben formate.

---

## Riferimenti

- [Crafting Interpreters](https://craftinginterpreters.com) — Cap. 5 (Representing Code) e Cap. 6 (Parsing Expressions) — costruzione AST e Recursive Descent
- [AST Explorer](https://astexplorer.net) — strumento interattivo, incolla codice JS/Vue e vedi l'AST prodotto in tempo reale. Utile per capire come appaiono gli AST di linguaggi reali
- Codice sorgente `@vue/compiler-core` → `packages/compiler-core/src/parse.ts` — parser per template Vue, sintassi molto simile a quella di Clutter

---

*End of Document*
