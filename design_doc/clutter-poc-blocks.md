# CLUTTER — POC Building Blocks

**Version**: 0.1.0-draft  
**Status**: Exploration  
**Author**: Luca

---

## Approccio

Compilatore/transpiler custom. File `.clutter` → AST → validazione semantica → codice target.

Riferimenti architetturali: Babel, `@vue/compiler-sfc`, compilatore Svelte. La struttura è consolidata — variazioni minori tra progetti, dipendenze tra blocchi sempre le stesse.

---

## Blocchi in ordine di dipendenza

### 1. Lexer

Primo contatto con il testo grezzo del file `.clutter`. Trasforma la sequenza di caratteri in token significativi.

Esempi di token: `<`, `Column`, `gap`, `=`, `"md"`, `>`, `{`, `title`, `}`, `---`

Tutto il resto della pipeline dipende da questo blocco. Nessun altro blocco può esistere senza di esso.

---

### 2. Parser

Prende i token prodotti dal lexer e costruisce un AST (Abstract Syntax Tree).

Responsabilità:
- Riconoscere il separatore `---` e distinguere le due sezioni del file (logica / template)
- Ricostruire la struttura gerarchica del template (nesting componenti)
- Riconoscere props, valori, riferimenti a variabili
- Trattare la sezione logica come blocco TypeScript opaco

Output: albero che rappresenta il file in forma manipolabile dai blocchi successivi.

---

### 3. Semantic Analyzer

Percorre l'AST e valida il significato — non la sintassi. È il blocco che dimostra il valore principale del progetto.

Responsabilità:
- Caricare `tokens.clutter` (design system)
- Verificare che ogni prop riceva un valore presente nei token
- Verificare che i riferimenti a variabili nella sezione template esistano nella sezione logica
- Produrre errori semantici human-readable

Esempio di errore atteso:
```
Errore [CLT001] — riga 4, colonna 12
Valore 'xl2' non esiste per la prop 'gap'.
Valori validi: xs, sm, md, lg, xl, xxl
```

---

### 4. Code Generator

Percorre l'AST validato e produce il codice target. Separato dall'analyzer perché i target sono intercambiabili.

Target per il POC: Vue SFC o HTML statico.

Responsabilità:
- Mappare ogni nodo dell'AST al costrutto corrispondente nel target
- Preservare la sezione logica (TypeScript) invariata o con modifiche minime
- Referenziare correttamente i componenti del runtime

---

### 5. Runtime (minimale)

Implementazione concreta dei componenti built-in. Il codice generato dal code generator li referenzia — devono esistere da qualche parte.

Componenti per il POC: `Column`, `Row`, `Box`, `Text`, `Button`

Per il POC possono essere implementazioni semplificate. L'importante è che girino e che producano output visivamente corretto. Non sono esposti al developer — sono opachi.

---

### 6. CLI

Il collante dell'intera pipeline. Prende un file `.clutter`, lo passa attraverso i blocchi 1→4, scrive l'output su disco.

Interfaccia minima per il POC:
```
clutter build <file>
```

Nient'altro. Nessun watch mode, nessuna configurazione, nessun output di debug per default.

---

## Schema dipendenze

```
tokens.clutter
      |
      ↓
File .clutter
      |
      ↓
   [Lexer]
      |
      ↓
   [Parser] → AST
      |
      ↓
[Semantic Analyzer] ← tokens.clutter
      |
      ↓
[Code Generator]
      |
      ↓
  Output target
  (Vue SFC | HTML)
      |
      ↓
   [Runtime]
   (referenziato dall'output)

   [CLI]
   (orchestra tutto)
```

---

## Note

- L'ordine dei blocchi è vincolato dalle dipendenze — non è arbitrario
- CLI e Runtime vengono per ultimi perché sono consumatori di tutto il resto
- Il Semantic Analyzer è il blocco con più valore dimostrativo per il POC — è dove si vede che "un valore sbagliato non compila"
- LSP, tooling editor, plugin architecture, target multipli: fuori scope per il POC

---

*End of Document*
