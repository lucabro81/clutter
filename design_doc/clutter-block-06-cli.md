# CLUTTER — Block 6: CLI

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Cos'è la CLI e perché esiste

La CLI (Command Line Interface) è il punto di ingresso pubblico del compilatore — l'unica cosa che il developer vede e usa direttamente. Tutto il resto della pipeline (Lexer, Parser, Semantic Analyzer, Code Generator) è invisibile: la CLI li orchestra in sequenza e presenta il risultato.

È il blocco più semplice da implementare, ma è quello che determina la qualità percepita dell'intero strumento. Un compilatore tecnicamente perfetto con una CLI confusa o verbosa si usa malvolentieri.

---

## Interfaccia per il POC

Per il POC, la CLI espone un singolo comando:

```
clutter build <file> [--target <vue|html>]
```

**Argomenti:**

| Argomento | Obbligatorio | Default | Descrizione |
|---|---|---|---|
| `<file>` | Sì | — | Percorso del file `.clutter` da compilare |
| `--target` | No | `vue` | Target di compilazione |
| `--out` | No | stessa dir del sorgente | Directory di output |

**Esempi:**

```bash
# Compila verso Vue SFC (default)
clutter build src/components/Card.clutter

# Compila verso HTML
clutter build src/components/Card.clutter --target html

# Specifica directory di output
clutter build src/components/Card.clutter --out dist/
```

---

## Flusso di esecuzione

Quando il developer esegue `clutter build`, la CLI:

```
1. Legge e valida gli argomenti
2. Verifica che il file sorgente esista
3. Cerca tokens.clutter (dalla directory del file, risalendo verso la root)
4. Legge il file sorgente
5. Invoca il Lexer → array di token
6. Invoca il Parser → AST
7. Invoca il Semantic Analyzer → AST validato o lista errori
8. Se ci sono errori → stampa gli errori, esce con codice 1
9. Se nessun errore → invoca il Code Generator → stringa di output
10. Scrive il file di output su disco
11. Stampa messaggio di successo, esce con codice 0
```

Il codice di uscita (`0` = successo, `1` = errore) è importante per l'integrazione con CI/CD e script automatizzati.

---

## Output a schermo

### Successo

```
✓ Card.clutter → Card.vue (12ms)
```

Conciso. Niente di più.

### Errori

```
✗ Card.clutter — 2 errori

  Errore [CLT102] riga 4, col 12
  Valore 'xl2' non valido per la prop 'gap' di 'Column'.
  Valori validi: xs, sm, md, lg, xl, xxl

    4 │ <Column gap="xl2">
                    ^^^

  Errore [CLT104] riga 9, col 8
  '{pippo}' non è dichiarato nella sezione logica.

    9 │   <Text>{pippo}</Text>
                ^^^^^^
```

Ogni errore mostra:
- Codice e posizione
- Messaggio human-readable
- Hint con valori validi quando applicabile
- Il frammento di sorgente con il punto esatto evidenziato

Il frammento di sorgente con highlight è la differenza tra un compilatore professionale e uno amatoriale. In Rust, `miette` produce questo output automaticamente partendo dagli `ErrorNode` con posizione che il Semantic Analyzer già produce.

---

## Ricerca di tokens.clutter

Il file `tokens.clutter` non viene passato esplicitamente — la CLI lo cerca automaticamente risalendo la struttura di directory a partire dalla posizione del file sorgente, fino alla root del filesystem o fino a trovarlo.

```
src/
  components/
    Card.clutter     ← file sorgente
  tokens.clutter     ← trovato qui
```

Questo comportamento è identico a come `eslint`, `prettier` e `tsconfig.json` vengono cercati — familiare per qualsiasi developer JavaScript.

Se `tokens.clutter` non viene trovato, la CLI produce un errore esplicito:

```
✗ tokens.clutter non trovato.
  Cercato in: src/components/, src/, ./
  Crea un file tokens.clutter nella root del progetto.
```

---

## Comandi futuri (fuori scope POC)

Per completezza, l'interfaccia a regime includerà:

```bash
# Controlla senza produrre output
clutter check <file>

# Compila tutti i file .clutter in una directory
clutter build src/ --out dist/

# File watcher (rilancia alla modifica)
clutter watch src/

# Valida tokens.clutter
clutter tokens validate

# Mostra versione
clutter --version
```

Nessuno di questi è necessario per il POC.

---

## Stack: clap

In Rust, la CLI si costruisce con `clap` — la libreria standard de facto per argument parsing. Gestisce automaticamente:
- Parsing degli argomenti e flag
- Generazione dell'help (`clutter --help`)
- Validazione dei valori (es. `--target` accetta solo `vue` o `html`)
- Messaggi di errore per argomenti mancanti o non validi

```
clutter build

Usage: clutter build <file> [OPTIONS]

Arguments:
  <file>  File .clutter da compilare

Options:
  --target <TARGET>  Target di compilazione [default: vue] [possible values: vue, html]
  --out <DIR>        Directory di output
  -h, --help         Print help
```

Questo output viene generato da `clap` senza scrivere una riga di codice manuale.

---

## Input e Output del blocco

**Input**: argomenti da riga di comando

**Output**:
- File scritto su disco (successo)
- Messaggi a schermo (errori o conferma)
- Codice di uscita del processo (`0` o `1`)

La CLI è l'unico blocco che ha effetti collaterali visibili all'esterno — tutti gli altri blocchi ricevono dati e restituiscono dati, senza toccare il filesystem o lo stdout direttamente.

---

## Riferimenti

- [`clap`](https://docs.rs/clap) — argument parsing per Rust, standard de facto
- [`miette`](https://docs.rs/miette) — error reporting con highlight del sorgente, usato da Cargo e altri tool Rust
- [`std::fs`](https://doc.rust-lang.org/std/fs/) — lettura e scrittura file, parte della stdlib Rust, nessuna dipendenza esterna necessaria

---

*End of Document*
