# CLUTTER — Block 5: Runtime

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Disambiguazione: cosa si intende per "runtime"

Il termine "runtime" è ambiguo — vale la pena chiarire cosa significa in questo documento.

**Ambiente di esecuzione** (runtime nel senso comune) — Node, Bun, il browser, la JVM. È la macchina che esegue il codice. Clutter non ne ha controllo né responsabilità. Ci sarà sempre un browser che mastica JS, e probabilmente un server Node o Bun durante lo sviluppo.

**Runtime di Clutter** (il senso usato in questo documento) — il codice che deve essere presente nell'ambiente di esecuzione dell'output perché quell'output funzioni. Non è l'ambiente stesso, è la libreria che il codice generato presuppone disponibile.

Un'analogia diretta: quando React genera il DOM virtuale, assume che `react-dom` esista nell'ambiente. `react-dom` è il runtime di React — non Node, non il browser, ma la libreria che React presuppone presente. Per Clutter è la stessa cosa.

---

## Cos'è il Runtime di Clutter e perché esiste

Il Runtime di Clutter è il codice che il Code Generator assume esista nell'ambiente quando produce l'output. Non è parte del compilatore — il compilatore lo assume presente, non lo gestisce.

La risposta a "cosa serve a runtime" dipende interamente dal target di compilazione. Non esiste un Runtime Clutter monolitico — esiste il runtime necessario a ciascun target.

---

## Principio guida

> Il runtime è quello che serve al target, niente di più.

Un runtime proprietario ha senso solo quando i runtime esistenti non soddisfano bisogni concreti. Per il POC — e probabilmente per molto tempo dopo — quei bisogni non esistono. Costruire un runtime proprio prima che emerga una necessità reale sarebbe lavoro sprecato su un problema non ancora definito.

---

## Runtime per target

### Target Vue SFC

**Runtime necessario: nessuno aggiuntivo.**

Il target Vue SFC genera HTML nativo nel template e TypeScript valido nel `<script setup>`. Il runtime è Vue stesso — già presente in qualsiasi applicazione Vue/Nuxt che consuma il file generato.

Le classi CSS sono incluse nel `<style scoped>` del file generato. Non c'è nulla da installare, nulla da importare.

Questo è uno dei vantaggi della scelta di generare HTML nativo invece di componenti `<Column>`, `<Text>` ecc.: il target Vue SFC è completamente autosufficiente.

### Target HTML

**Runtime necessario per il POC: Alpine.js**

Alpine.js gestisce la reattività nel target HTML — stato, binding, condizionali, iterazioni. È una dipendenza esterna caricata via CDN o installata come pacchetto.

Il Code Generator include automaticamente il riferimento ad Alpine nell'HTML prodotto. Non richiede configurazione da parte del developer.

**Runtime a regime: `@vue/reactivity`**

A regime, il target HTML userà `@vue/reactivity` — il pacchetto di reattività di Vue, distribuito separatamente e usabile senza il resto del framework. Questo elimina la dipendenza da Alpine e unifica il modello reattivo tra i target.

`@vue/reactivity` verrà bundlato nel runtime Clutter per il target HTML — il developer non lo installa esplicitamente, è una dipendenza interna di Clutter.

---

## Inconsistenza tra target (nota architetturale)

L'approccio "runtime diverso per target diverso" ha un rischio: Alpine e `@vue/reactivity` non si comportano identicamente in tutti i casi edge. Codice che funziona nel target Vue potrebbe avere comportamenti leggermente diversi nel target HTML.

Per il POC questa inconsistenza è accettabile — i casi d'uso sono semplici e ben definiti. A regime, unificare il runtime reattivo su `@vue/reactivity` per entrambi i target risolve il problema alla radice.

---

## Dev server e hot reload

Un ambiente di sviluppo con file watcher e hot reload è fuori scope per il POC, ma vale la pena definire come si struttura architetturalmente per non prendere decisioni sbagliate ora.

Il compilatore Rust è un binario — prende un file `.clutter` e produce output. Il file watcher che rileva le modifiche e rilancia il compilatore non deve essere scritto in Rust: è più semplice e pragmatico usare un watcher esistente (Bun, Node, o anche un semplice script shell) che osserva i file `.clutter` e invoca il binario Rust a ogni modifica.

Il hot reload lato browser (aggiornamento senza refresh completo) dipende dal target:
- Target Vue: gestito da Vite/Nuxt, già presente nell'applicazione host
- Target HTML: richiederebbe un server di sviluppo con WebSocket — da costruire o da delegare a uno strumento esistente (Bun serve, Vite in modalità standalone)

In entrambi i casi il compilatore Rust non sa nulla del dev server — è il layer esterno che si occupa di orchestrare.

---

## Nota sul futuro: linguaggio della sezione logica

La sezione logica di un file `.clutter` è oggi TypeScript. Questa scelta è pragmatica per il POC, non architetturale.

Se il compilatore è in Rust e i target sono intercambiabili, la sezione logica potrebbe in futuro essere scritta in qualsiasi linguaggio che:
- Abbia un sistema di tipi sufficientemente espressivo
- Possa essere compilato o transpilato verso il runtime del target
- Abbia o possa avere un sistema di reattività implementabile

Rust, Go, o un DSL proprietario sono tutti candidati teoricamente validi. È una direzione coerente con la visione del progetto — da tenere come nota a margine, non come obiettivo immediato.

---

## Cosa il Runtime non è

Il Runtime non è il compilatore. Non partecipa alla pipeline Lexer → Parser → Semantic Analyzer → Code Generator. Non viene invocato da `clutter build`.

Il Runtime esiste nell'ambiente di esecuzione dell'output — nel browser, nell'applicazione Vue, nel file HTML prodotto. Il compilatore lo assume presente, non lo gestisce.

---

## Runtime proprietario: quando e perché

Un runtime proprietario significa scrivere e controllare direttamente il codice che gestisce la reattività, il mounting dei componenti, il lifecycle e gli update del DOM — invece di delegare a Vue o Alpine.

### Cosa farebbe concretamente

- Gestirebbe la reattività con un'implementazione propria, ottimizzata per le assunzioni che Clutter può fare (vocabolario chiuso, struttura prevedibile)
- Monterebbe i componenti nell'albero DOM
- Gestirebbe il lifecycle (created, mounted, destroyed, ecc.)
- Ottimizzerebbe gli update del DOM in modo specifico per l'output del compilatore Clutter

### Lo sweet spot realistico

Reinventare la reattività da zero è un progetto nel progetto. Lo sweet spot è prendere `@vue/reactivity` o il core di React, modificarlo dove necessario, e distribuirlo come pacchetto interno di Clutter. Il developer non lo vede, non lo installa, non sa che esiste — è una dipendenza interna bundlata nell'output.

Questo dà controllo totale sul comportamento, zero dipendenza esplicita da Vue o React per il developer, e non richiede di reinventare anni di lavoro sul problema della reattività.

### WASM: la direzione per casi d'uso intensivi

Se il compilatore è in Rust, il runtime potrebbe essere scritto in Rust e compilato a **WebAssembly**. Il codice generato chiamerebbe il runtime WASM invece di una libreria JS — il browser esegue bytecode nativo invece di interpretare JavaScript.

Il vantaggio non è la reattività — per aggiornare il DOM la differenza è trascurabile. Il vantaggio è per computazioni intensive: elaborazione dati, simulazioni, visualizzazioni scientifiche, algoritmi su grandi dataset. Casi in cui JS è il collo di bottiglia e WASM risolve il problema strutturalmente.

Questa è la direzione di lungo termine per Clutter come piattaforma per applicazioni data-intensive — non un obiettivo immediato, ma una freccia coerente con la scelta di Rust come linguaggio del compilatore.

---



## Riferimenti

- [`@vue/reactivity`](https://github.com/vuejs/core/tree/main/packages/reactivity) — pacchetto reattività Vue standalone, base per il runtime proprietario
- [Alpine.js](https://alpinejs.dev) — runtime reattivo leggero per il target HTML nel POC
- [Bun](https://bun.sh) — candidato per file watcher e dev server nel layer di orchestrazione
- [Leptos](https://leptos.dev) — framework Rust con runtime WASM, riferimento per capire come funziona un runtime Rust→WASM nel browser
- [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) — tool per chiamare API browser da Rust compilato a WASM

---

*End of Document*
