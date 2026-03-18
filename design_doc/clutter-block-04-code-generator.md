# CLUTTER — Block 4: Code Generator

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Cos'è il Code Generator e perché esiste

Il Code Generator è l'ultimo stadio della pipeline di compilazione. Riceve l'AST validato dal Semantic Analyzer e lo trasforma in codice nel linguaggio target — per il POC, Vue SFC o HTML statico.

È il blocco che chiude il cerchio: da un file `.clutter` scritto dal developer si ottiene codice funzionante che un browser o un'applicazione Vue può eseguire.

Il Code Generator non decide nulla — tutto è già stato deciso. Il Lexer ha riconosciuto, il Parser ha strutturato, il Semantic Analyzer ha validato. Il Code Generator si limita a tradurre fedelmente l'AST nella sintassi del target.

Questa separazione è intenzionale: tenere la logica di validazione separata dalla logica di generazione permette di aggiungere target senza toccare nulla di quello che è già stato costruito.

---

## Il concetto di target

Un target è una specifica strategia di generazione del codice. Lo stesso AST può produrre output diversi a seconda del target selezionato.

Per il POC:

**Target Vue SFC** — produce un file `.vue` valido, integrabile in qualsiasi applicazione Vue/Nuxt esistente. È il target principale perché permette adozione graduale su progetti reali (Comperio).

**Target HTML statico** — produce HTML e CSS inline, senza dipendenze. Utile per validare il concetto in isolamento, senza bisogno di un'applicazione Vue.

In futuro ogni target è un modulo separato — il Code Generator core percorre l'AST, ogni target sa come tradurre ogni tipo di nodo. Aggiungere React come target significa aggiungere un modulo, non modificare il compilatore.

---

## Come funziona internamente

Il Code Generator visita l'AST con una tecnica chiamata **Visitor Pattern** — per ogni tipo di nodo dell'AST esiste una funzione che sa come tradurlo nel target corrente.

### Il Visitor Pattern

Il Visitor Pattern è il modo standard in cui i compilatori percorrono un AST per produrre output. L'idea è semplice: invece di mettere la logica di generazione dentro i nodi stessi, si definisce un oggetto esterno (il visitor) che ha una funzione per ogni tipo di nodo.

```
Visitor {
  visitProgramNode(node)     → genera la struttura del file
  visitComponentNode(node)   → genera il tag del componente
  visitTextNode(node)        → genera il testo statico
  visitExpressionNode(node)  → genera il riferimento alla variabile
  visitIfNode(node)          → genera il costrutto condizionale
  visitEachNode(node)        → genera l'iterazione
}
```

Quando il Code Generator incontra un `ComponentNode`, chiama `visitComponentNode`. Quella funzione genera il tag di apertura, poi chiama ricorsivamente il visitor sui figli, poi genera il tag di chiusura.

Il vantaggio rispetto a un grande `if/else` o `switch` è che ogni target implementa il proprio visitor — stesso AST, traduzione diversa per ogni target.

---

## Target Vue SFC

Un Vue SFC (Single File Component) ha questa struttura:

```
<template>
  ...
</template>

<script setup lang="ts">
  ...
</script>
```

Il Code Generator per Vue deve produrre esattamente questo.

### Scelta di design: HTML nativo nel template

I componenti built-in di Clutter vengono espansi in HTML nativo nel template Vue — non come componenti `<Column>`, `<Text>` ecc. Il motivo è pragmatico: un `.vue` con HTML standard funziona in qualsiasi applicazione Vue senza installare nulla. Nessuna dipendenza runtime, nessun import aggiuntivo.

Le props semantiche di Clutter (`gap="md"`, `variant="primary"`) vengono tradotte in classi CSS generate da `tokens.clutter`. Il blocco `<style>` corrispondente viene incluso nel file `.vue` prodotto.

### Mapping dei nodi

**ProgramNode → file .vue completo**

```
<template>
  [output del template — HTML nativo]
</template>

<script setup lang="ts">
  [logicBlock invariato]
</script>

<style scoped>
  [classi CSS generate da tokens.clutter]
</style>
```

**ComponentNode Column → div flex column**

```
// Input AST
ComponentNode { name: "Column", props: [{ name: "gap", value: "md" }], children: [...] }

// Output Vue
<div class="clutter-column clutter-gap-md">
  [output dei figli]
</div>
```

**ComponentNode Row → div flex row**

```
<div class="clutter-row clutter-gap-md">
  [output dei figli]
</div>
```

**ComponentNode Text → elemento tipografico**

```
// Input: <Text size="lg" weight="bold">Ciao</Text>
// Output:
<p class="clutter-text clutter-size-lg clutter-weight-bold">Ciao</p>
```

**ComponentNode Button → button nativo**

```
// Input: <Button variant="primary">OK</Button>
// Output:
<button class="clutter-button clutter-variant-primary">OK</button>
```

**ComponentNode Box → div generico**

```
// Input: <Box bg="surface" padding="md" radius="lg">...</Box>
// Output:
<div class="clutter-box clutter-bg-surface clutter-padding-md clutter-radius-lg">
  [output dei figli]
</div>
```

**Props con expression → binding Vue**

Le props che ricevono un'espressione `{variabile}` diventano binding Vue con `:`.

```
// Input: <Button disabled={isLoading}>
// Output:
<button :disabled="isLoading" class="clutter-button">
```

**TextNode → testo nel template**

```
TextNode { value: "Ciao" }  →  Ciao
```

**ExpressionNode → interpolazione Vue**

```
ExpressionNode { name: "title" }  →  {{ title }}
```

**IfNode → v-if / v-else Vue**

```
// Input Clutter
<if condition={isLoggedIn}>
  <Text>Benvenuto</Text>
</if>
<else>
  <Button variant="primary">Login</Button>
</else>

// Output Vue
<p v-if="isLoggedIn" class="clutter-text">Benvenuto</p>
<button v-else class="clutter-button clutter-variant-primary">Login</button>
```

**EachNode → v-for Vue**

```
// Input Clutter
<each item={products} as="product">
  <Box padding="md">...</Box>
</each>

// Output Vue
<div
  v-for="product in products"
  :key="product"
  class="clutter-box clutter-padding-md"
>
  [output dei figli]
</div>
```

### Il blocco style generato

Le classi CSS usate nel template vengono definite nel `<style scoped>` del file `.vue`, generate a partire da `tokens.clutter`:

```css
.clutter-column { display: flex; flex-direction: column; }
.clutter-row    { display: flex; flex-direction: row; }
.clutter-box    { box-sizing: border-box; }

.clutter-gap-xs  { gap: 4px; }
.clutter-gap-sm  { gap: 8px; }
.clutter-gap-md  { gap: 16px; }
/* ... una classe per ogni valore di ogni categoria di token */

.clutter-bg-primary  { background-color: #007AFF; }
.clutter-bg-surface  { background-color: #F2F2F7; }
/* ... */
```

Questo blocco è generato automaticamente — non viene mai scritto a mano.

### La sezione logica

La sezione logica TypeScript viene inserita nel `<script setup>` invariata. Il Code Generator non la tocca — è già TypeScript valido, Vue sa come gestirla.

---

## Target HTML

Il target HTML produce un file `.html` autonomo — nessuna dipendenza da Vue o da un build step. È il target che dimostra la portabilità del compilatore: lo stesso sorgente `.clutter` può girare anche fuori da un ecosistema Vue.

### Il problema della reattività

Un target HTML onesto deve eseguire la sezione logica del sorgente — se il developer ha scritto stato e handler, devono funzionare nell'output. Ignorare la sezione logica produrrebbe un HTML statico che non rispetta le promesse del sorgente.

Il problema è che la reattività (stato che aggiorna il DOM automaticamente) non è gratis in HTML puro — richiede un runtime.

### POC: Alpine.js come runtime temporaneo

Per il POC si usa **Alpine.js** come runtime reattivo. Alpine è essenzialmente "Vue senza build step": dichiara stato e binding direttamente negli attributi HTML, pesa ~15KB, non richiede configurazione.

Il Code Generator per il target HTML traduce:
- La sezione logica → oggetto `x-data` di Alpine
- Le espressioni `{variabile}` → `x-text="variabile"`
- I binding props → attributi Alpine (`:class`, `:disabled`, ecc.)
- `<if condition={...}>` → `x-show` o `x-if`
- `<each item={...}>` → `x-for`

```html
<!-- Output HTML con Alpine -->
<!DOCTYPE html>
<html>
<head>
  <style>
    /* CSS generato da tokens.clutter */
    .clutter-column { display: flex; flex-direction: column; }
    .clutter-gap-md  { gap: 16px; }
    /* ... */
  </style>
  <script src="https://cdn.jsdelivr.net/npm/alpinejs@3/dist/cdn.min.js" defer></script>
</head>
<body>
  <div x-data="{ title: 'Ciao', count: 0 }">
    <div class="clutter-column clutter-gap-md">
      <p class="clutter-text clutter-size-lg" x-text="title"></p>
      <button class="clutter-button clutter-variant-primary" @click="count++">
        Clicca
      </button>
    </div>
  </div>
</body>
</html>
```

La sezione logica TypeScript viene transpilata in JS vanilla (tramite `esbuild` o `tsc`) e iniettata nell'oggetto `x-data`.

### A regime: @vue/reactivity come runtime proprio

Alpine è una soluzione temporanea accettabile per il POC. A regime, Clutter avrà un runtime proprio basato su `@vue/reactivity` — il pacchetto che gestisce la reattività in Vue, distribuito separatamente e usabile in autonomia.

Questo permette di:
- Eliminare la dipendenza da Alpine
- Controllare completamente il comportamento reattivo
- Mantenere la sintassi del sorgente Clutter coerente tra target Vue e target HTML
- Non reinventare la reattività da zero — `@vue/reactivity` è battle-tested

Il Code Generator a regime produce HTML + un piccolo script che inizializza il runtime Clutter, monta il componente, e collega lo stato reattivo al DOM.

---

## Generazione del codice come stringhe

Concretamente, il Code Generator costruisce il codice target come una stringa di testo. Ogni `visit*` function restituisce una stringa che viene concatenata con le stringhe dei nodi vicini.

### Indentazione

Il codice generato deve essere leggibile — non un blob di testo su una riga. Il Code Generator tiene traccia del livello di annidamento corrente e indenta di conseguenza.

Pseudologica:

```
function visitComponentNode(node, depth):
  indent = "  ".repeat(depth)
  output = indent + "<" + node.name
  
  per ogni prop in node.props:
    output += " " + visitProp(prop)
  
  se node.children è vuoto:
    output += " />"
    return output
  
  output += ">\n"
  
  per ogni figlio in node.children:
    output += visitNode(figlio, depth + 1) + "\n"
  
  output += indent + "</" + node.name + ">"
  return output
```

### Source maps (fuori scope per il POC)

Nei compilatori reali, il code generator produce anche le **source maps** — file che mappano ogni riga del codice generato alla riga corrispondente nel sorgente originale. Questo permette al debugger di mostrare il file `.clutter` originale invece del `.vue` generato.

Per il POC le source maps sono fuori scope, ma è utile sapere che esistono e che vengono prodotte in questa fase.

---

## Input e Output del blocco

**Input**: AST validato (`{ success: true, ast: ProgramNode }`) + target selezionato

**Output**: stringa di testo contenente il codice nel linguaggio target

```
// Input
{
  success: true,
  ast: ProgramNode { ... },
  target: "vue"
}

// Output
"<template>\n  <Column gap=\"md\">\n    <Text size=\"lg\">Ciao</Text>\n  </Column>\n</template>\n\n<script setup lang=\"ts\">\nconst title = 'Ciao'\n</script>"
```

Il file viene poi scritto su disco dalla CLI.

---

## Come testare il Code Generator

Il Code Generator si testa fornendo direttamente un AST — non serve rieseguire l'intera pipeline ogni volta.

Casi da coprire:

**Target Vue**
- Componente senza props → tag Vue senza attributi
- Componente con prop stringa → attributo Vue
- Componente con prop expression → binding Vue con `:`
- TextNode → testo nel template
- ExpressionNode → interpolazione `{{ }}`
- Nesting → indentazione corretta
- IfNode → `v-if` / `v-else`
- EachNode → `v-for`
- Sezione logica → inserita invariata nel `<script setup>`

**Target HTML**
- Column → div con flex column e classi CSS
- Row → div con flex row e classi CSS
- Text con contenuto statico → p con classi CSS
- ExpressionNode → attributo `x-text` Alpine
- IfNode → `x-show` Alpine
- EachNode → `x-for` Alpine
- Sezione logica → oggetto `x-data` Alpine
- Blocco `<style>` generato da tokens.clutter presente nel file

**Generale**
- Output è codice sintatticamente valido (verificabile parsando il risultato)
- Indentazione consistente
- Nessun tag non chiuso

---

## Cosa il Code Generator non fa

Il Code Generator non valida nulla — quella responsabilità appartiene al Semantic Analyzer. Se riceve un AST valido, produce output valido. Non fa controlli aggiuntivi, non trasforma la logica, non ottimizza.

Non scrive il file su disco — quello è compito della CLI. Il Code Generator produce una stringa, chi lo chiama decide cosa farne.

---

## Riferimenti

- [Crafting Interpreters](https://craftinginterpreters.com) — Cap. 8 (Statements and State) e Cap. 23 (Jumping Back and Forth) — generazione di codice e visitor pattern
- [Design Patterns — Visitor](https://refactoring.guru/design-patterns/visitor) — spiegazione chiara del Visitor Pattern con esempi pratici
- Codice sorgente `@vue/compiler-core` → `packages/compiler-core/src/codegen.ts` — code generator del compilatore Vue, riferimento per il target Vue SFC
- [Alpine.js](https://alpinejs.dev) — runtime reattivo leggero usato nel target HTML per il POC
- [`@vue/reactivity`](https://github.com/vuejs/core/tree/main/packages/reactivity) — pacchetto reattività di Vue, autonomo, target del runtime Clutter a regime

---

*End of Document*
