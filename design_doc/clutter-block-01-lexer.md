# CLUTTER — Block 1: Lexer

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Cos'è un Lexer e perché esiste

Un compilatore non legge testo come lo leggiamo noi. Per una macchina, un file è una sequenza piatta di caratteri — non sa dove finisce una parola e dove inizia un numero, non sa che `<Column` è un'entità diversa da `gap="md"`.

Il Lexer (anche chiamato *tokenizer* o *scanner*) è il primo stadio: legge il testo carattere per carattere e lo trasforma in una sequenza di **token** — unità con un tipo e un valore.

Esempio:

```
<Column gap="md" padding="lg">
```

Diventa:

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

Il blocco successivo (Parser) riceve questa sequenza e ci costruisce sopra la struttura — ma non deve più preoccuparsi di caratteri, spazi, virgolette. Il Lexer ha già fatto quel lavoro.

---

## Perché separare Lexer e Parser

È una scelta di design consolidata in tutti i compilatori reali (GCC, Clang, Babel, TypeScript stesso). Il motivo è semplice: sono due problemi diversi.

Il Lexer risponde a: *"Cosa sono questi caratteri?"*
Il Parser risponde a: *"Cosa significa questa sequenza di token?"*

Tenerli separati rende entrambi più semplici da scrivere, testare e modificare. Se cambia la sintassi di una stringa (es. supporto a backtick), modifichi solo il Lexer. Se cambia la grammatica (es. nuovo costrutto `<if>`), modifichi solo il Parser.

---

## I token di Clutter

Per il POC, il Lexer deve riconoscere questi tipi di token:

### Strutturali

| Token | Esempio | Descrizione |
|---|---|---|
| `SECTION_SEPARATOR` | `---` | Separatore tra sezione logica e template |
| `OPEN_TAG` | `<Column` | Apertura tag componente |
| `CLOSE_TAG` | `>` | Chiusura apertura tag |
| `SELF_CLOSE_TAG` | `/>` | Chiusura self-closing tag |
| `CLOSE_OPEN_TAG` | `</Column>` | Tag di chiusura componente |

### Props e valori

| Token | Esempio | Descrizione |
|---|---|---|
| `IDENTIFIER` | `Column`, `gap`, `title` | Nome componente o prop |
| `EQUALS` | `=` | Assegnazione prop |
| `STRING` | `"md"`, `"primary"` | Valore stringa letterale |
| `EXPRESSION` | `{title}`, `{count}` | Riferimento a variabile della sezione logica |

### Controllo flusso

| Token | Esempio | Descrizione |
|---|---|---|
| `IF_OPEN` | `<if` | Apertura blocco condizionale |
| `ELSE_OPEN` | `<else>` | Blocco alternativo |
| `EACH_OPEN` | `<each` | Apertura iterazione |

### Contenuto

| Token | Esempio | Descrizione |
|---|---|---|
| `TEXT` | `Ciao mondo` | Testo statico tra tag |
| `WHITESPACE` | ` `, `\n` | Spazi e newline (spesso ignorati) |
| `EOF` | — | Fine del file |

### Sezione logica

La sezione logica (TypeScript) viene trattata come un blocco opaco — il Lexer non la analizza nel dettaglio, la raccoglie intera come un unico token `LOGIC_BLOCK`. Il type checking TypeScript non è compito del Lexer né del Parser di Clutter.

---

## Come funziona internamente

Il Lexer mantiene uno **stato corrente** mentre scorre il testo. Le domande che si fa a ogni carattere sono sempre le stesse:

1. Sono nella sezione logica o nel template?
2. Il carattere corrente inizia un nuovo token?
3. Il token corrente è terminato?

### La macchina a stati

Un Lexer è formalmente una *macchina a stati finiti*. Non è necessario conoscere la teoria formale — l'idea pratica è questa: il Lexer sa sempre in quale "modalità" si trova, e ogni carattere può confermare la modalità corrente o farlo passare a un'altra.

Stati principali per Clutter:

```
LOGIC       — sta leggendo la sezione TypeScript
TEMPLATE    — sta leggendo il template
IN_TAG      — dentro un tag aperto (<Column ...)
IN_STRING   — dentro una stringa "..."
IN_EXPR     — dentro un'espressione {....}
```

Esempio di transizione:

```
stato: TEMPLATE
carattere: "<"
  → entra in IN_TAG, inizia a raccogliere il nome del tag

stato: IN_TAG
carattere: " " (spazio)
  → emette token IDENTIFIER con il nome raccolto, rimane in IN_TAG

stato: IN_TAG
carattere: "="
  → emette token EQUALS, rimane in IN_TAG

stato: IN_TAG
carattere: '"'
  → entra in IN_STRING

stato: IN_STRING
carattere: '"' (secondo)
  → emette token STRING con il valore raccolto, torna in IN_TAG
```

---

## Gestione del separatore `---`

Il separatore è il caso più peculiare di Clutter — non esiste in altri linguaggi di markup.

Il Lexer parte in modalità `LOGIC`. Quando incontra una riga che contiene esattamente `---` (e nient'altro), emette `SECTION_SEPARATOR` e passa in modalità `TEMPLATE`. Da quel punto in poi, tutto viene letto come template.

Questo significa che `---` nel codice TypeScript della sezione logica sarebbe un problema. Soluzione semplice per il POC: documentarlo come valore riservato e non supportato nella sezione logica. Se in futuro serve (es. decremento `x---`), si può gestire con un contesto più sofisticato.

---

## Informazioni di posizione

Ogni token deve portare con sé la sua posizione nel file sorgente:

```
{
  type: "STRING",
  value: "xl2",
  line: 4,
  column: 12
}
```

Questo è indispensabile per produrre errori utili negli stadi successivi:

```
Errore [CLT001] — riga 4, colonna 12
Valore 'xl2' non esiste per la prop 'gap'.
```

Senza posizione, l'errore non ha coordinate — inutile in pratica. Tutte le informazioni di posizione vengono raccolte durante la fase di lexing, perché è l'unico momento in cui si conosce il rapporto tra caratteri e righe del file sorgente.

---

## Cosa il Lexer non fa

Chiarire i confini è utile quanto definire le responsabilità.

Il Lexer **non** verifica se la sintassi è corretta — può emettere token da una sequenza malformata senza saperlo. Non sa se `<Column` ha il `>` di chiusura. Non sa se `gap="xl2"` è un valore valido. Non conosce i token del design system.

Queste sono responsabilità del Parser e del Semantic Analyzer.

---

## Gestione degli errori nel Lexer

Il Lexer può incontrare caratteri che non riesce a classificare. La strategia standard è:

1. Emettere un token `UNKNOWN` con il carattere non riconosciuto
2. Continuare a leggere (non fermarsi al primo errore)
3. Raccogliere tutti gli errori, non solo il primo

Il motivo è pratico: se il compilatore si ferma al primo errore, il developer deve correggere, ricompilare, scoprire il secondo errore, ricompilare. Raccogliere tutti gli errori possibili in una sola passata è molto più utile.

---

## Input e Output del blocco

**Input**: stringa di testo — il contenuto grezzo di un file `.clutter`

**Output**: array di token, ognuno con tipo, valore e posizione

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

## Come testare il Lexer

Il Lexer è il blocco più semplice da testare in isolamento — dato un input testuale, l'output è deterministico e verificabile.

Casi da coprire:

- File minimale (solo separatore `---`, template vuoto)
- Componente senza props
- Componente con prop stringa
- Componente con prop expression `{var}`
- Nesting di componenti
- Sezione logica con codice TypeScript reale
- Carattere non riconosciuto → token `UNKNOWN`
- File senza separatore `---` → errore esplicito

---

## Riferimenti

- [Crafting Interpreters](https://craftinginterpreters.com) — Cap. 3 (Scanning) — riferimento pratico per implementare un lexer da zero, linguaggio agnostico
- Codice sorgente `@vue/compiler-core` → `packages/compiler-core/src/tokenizer.ts` — esempio reale di tokenizer per sintassi JSX-like
- Codice sorgente Babel → `@babel/parser/src/tokenizer` — riferimento per gestione stati e posizioni

---

*End of Document*
