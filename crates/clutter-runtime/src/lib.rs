//! Tipi condivisi dell'intero pipeline del compilatore Clutter.
//!
//! Questo crate è la dipendenza comune di tutti gli altri (`clutter-lexer`,
//! `clutter-parser`, `clutter-analyzer`, `clutter-codegen`). Non contiene logica:
//! definisce soltanto le strutture dati scambiate tra le fasi.
//!
//! # Struttura
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │           clutter-runtime           │
//! │                                     │
//! │  Token/TokenKind  ← usati da lexer  │
//! │  AST nodes        ← usati da parser │
//! │  *Error types     ← usati da tutti  │
//! └─────────────────────────────────────┘
//! ```
//!
//! # Tipi di errore
//!
//! Ogni fase produce il proprio tipo di errore, tutti con la stessa struttura
//! `{ message, pos }` per coerenza. L'integrazione `miette` (Block 5) li
//! arricchirà con codici strutturati e span multi-token.
//!
//! | Tipo            | Prodotto da       |
//! |-----------------|-------------------|
//! | [`LexError`]    | `clutter-lexer`   |
//! | [`ParseError`]  | `clutter-parser`  |
//! | [`AnalyzerError`]| `clutter-analyzer`|

// ---------------------------------------------------------------------------
// Posizione nel sorgente
// ---------------------------------------------------------------------------

/// Posizione di un token o di un nodo AST nel file sorgente `.clutter`.
///
/// Indica l'inizio del token (primo carattere). Le righe e le colonne sono
/// indicizzate a partire da 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// Numero di riga (1-based).
    pub line: usize,
    /// Numero di colonna (1-based).
    pub col: usize,
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// Categoria di un token prodotto dal lexer.
///
/// Il lexer categorizza ogni frammento del sorgente in un `TokenKind` prima di
/// passare il flusso al parser. I token `Whitespace` sono prodotti ma il parser
/// li ignora tramite `skip_whitespace`; `Unknown` segnala un carattere non
/// riconosciuto (il lexer produce anche un [`LexError`] in quel caso).
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // --- Strutturali ---
    /// Il separatore `---` tra logic block e template.
    SectionSeparator,
    /// `<Name` seguito da `>`: apertura di un tag con possibili figli.
    OpenTag,
    /// `>`: chiude un tag aperto (non self-closing).
    CloseTag,
    /// `/>`: chiude un tag senza figli.
    SelfCloseTag,
    /// `</Name>`: chiude un tag aperto in precedenza.
    CloseOpenTag,

    // --- Props ---
    /// Nome di una prop: sequenza alfanumerica/underscore/trattino prima di `=`.
    Identifier,
    /// Il carattere `=` tra nome e valore di una prop.
    Equals,
    /// Valore stringa di una prop: contenuto tra `"..."`.
    StringLit,
    /// Valore espressione di una prop o interpolazione nel testo: contenuto tra `{...}`.
    Expression,

    // --- Control flow ---
    /// Tag `<if`: introduce un condizionale. Le props vengono lette normalmente.
    IfOpen,
    /// Tag `<else`: ramo alternativo di un `<if>`.
    ElseOpen,
    /// Tag `<each`: introduce un ciclo. Props: `collection={expr} as="alias"`.
    EachOpen,

    // --- Contenuto ---
    /// Testo statico tra tag (non whitespace).
    Text,
    /// Sequenza di spazi, tab o newline tra elementi del template.
    Whitespace,
    /// Segna la fine del flusso di token. Sempre l'ultimo token emesso.
    Eof,

    // --- Logic section ---
    /// Il contenuto grezzo del logic block TypeScript (prima del `---`).
    /// Il compilatore lo tratta come opaco: viene passato invariato al codegen.
    LogicBlock,

    // --- Errore ---
    /// Carattere non riconosciuto. Accompagnato da un [`LexError`] nel vettore degli errori.
    Unknown,
}

/// Un singolo token prodotto dal lexer.
///
/// Ogni token porta il proprio [`TokenKind`], il testo originale estratto dal
/// sorgente e la [`Position`] del suo primo carattere.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Categoria del token.
    pub kind: TokenKind,
    /// Testo grezzo dal sorgente (es. `"Column"`, `"md"`, `"---"`).
    pub value: String,
    /// Posizione nel sorgente (primo carattere del token).
    pub pos: Position,
}

// ---------------------------------------------------------------------------
// Errori del lexer
// ---------------------------------------------------------------------------

/// Errore prodotto dal lexer durante la tokenizzazione.
///
/// Il lexer non si ferma al primo errore: continua la scansione e accumula tutti
/// gli errori in un `Vec<LexError>` restituito insieme al flusso di token.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    /// Descrizione leggibile del problema (es. `"unexpected character '@' in template"`).
    pub message: String,
    /// Posizione nel sorgente dove è stato rilevato l'errore.
    pub pos: Position,
}

// ---------------------------------------------------------------------------
// Nodi AST
// ---------------------------------------------------------------------------

/// Valore di una prop di un componente.
///
/// Una prop può avere un valore stringa letterale — da validare contro il design
/// system — oppure un'espressione TypeScript — valutata a runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    /// Stringa letterale: `gap="md"`. Deve essere presente nel design system.
    StringValue(String),
    /// Espressione TypeScript: `gap={myVar}`. Il nome dell'identificatore viene
    /// verificato dall'analyzer contro i binding dichiarati nel logic block.
    ExpressionValue(String),
}

/// Una singola prop `name=value` di un componente.
#[derive(Debug, Clone, PartialEq)]
pub struct PropNode {
    /// Nome della prop (es. `"gap"`, `"size"`).
    pub name: String,
    /// Valore della prop (stringa o espressione).
    pub value: PropValue,
    /// Posizione nel sorgente (primo carattere del nome).
    pub pos: Position,
}

/// Un componente del vocabolario chiuso (es. `<Column>`, `<Text />`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentNode {
    /// Nome del componente (es. `"Column"`, `"Text"`).
    pub name: String,
    /// Props dichiarate sul tag di apertura.
    pub props: Vec<PropNode>,
    /// Figli: presenti solo se il tag non è self-closing.
    pub children: Vec<Node>,
    /// Posizione del tag di apertura nel sorgente.
    pub pos: Position,
}

/// Testo statico tra tag (non interpolazione, non whitespace strutturale).
#[derive(Debug, Clone, PartialEq)]
pub struct TextNode {
    /// Il testo grezzo.
    pub value: String,
    /// Posizione nel sorgente.
    pub pos: Position,
}

/// Interpolazione di un'espressione TypeScript nel template: `{expr}`.
///
/// Il nome dell'espressione viene verificato dall'analyzer (CLT104) contro i
/// binding dichiarati nel logic block.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionNode {
    /// Nome dell'identificatore interpolato (es. `"title"`, `"count"`).
    pub value: String,
    /// Posizione nel sorgente.
    pub pos: Position,
}

/// Nodo condizionale `<if condition={expr}>…</if>` con ramo else opzionale.
#[derive(Debug, Clone, PartialEq)]
pub struct IfNode {
    /// L'espressione della condizione (nome dell'identificatore).
    pub condition: String,
    /// Figli del ramo `then` (tra `<if>` e `<else>` o `</if>`).
    pub then_children: Vec<Node>,
    /// Figli del ramo `else`, presenti solo se il tag `<else>` è dichiarato.
    pub else_children: Option<Vec<Node>>,
    /// Posizione del tag `<if>` nel sorgente.
    pub pos: Position,
}

/// Nodo di iterazione `<each collection={expr} as="alias">…</each>`.
#[derive(Debug, Clone, PartialEq)]
pub struct EachNode {
    /// L'espressione della collezione (nome dell'identificatore).
    pub collection: String,
    /// L'alias assegnato all'elemento corrente (binding locale per i figli).
    pub alias: String,
    /// Figli del corpo del ciclo.
    pub children: Vec<Node>,
    /// Posizione del tag `<each>` nel sorgente.
    pub pos: Position,
}

/// Un nodo del template: l'unione di tutti i tipi di nodo possibili.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Componente del vocabolario chiuso (es. `<Column>`, `<Text />`).
    Component(ComponentNode),
    /// Testo statico.
    Text(TextNode),
    /// Interpolazione di espressione `{expr}`.
    Expr(ExpressionNode),
    /// Condizionale `<if>`.
    If(IfNode),
    /// Iterazione `<each>`.
    Each(EachNode),
}

/// La radice dell'AST prodotta dal parser.
///
/// Corrisponde a un intero file `.clutter`. La struttura del file è:
///
/// ```text
/// [logic block TypeScript — opaco per il compilatore]
/// ---
/// [template — nodi AST]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramNode {
    /// Il contenuto grezzo del logic block TypeScript (prima del `---`).
    /// Può essere vuoto se il file inizia direttamente con `---`.
    pub logic_block: String,
    /// I nodi di primo livello del template (dopo il `---`).
    pub template: Vec<Node>,
}

// ---------------------------------------------------------------------------
// Errori del parser e dell'analyzer
// ---------------------------------------------------------------------------

/// Errore prodotto dal parser durante la costruzione dell'AST.
///
/// Il parser non si ferma al primo errore: applica una strategia di recovery
/// (avanza fino alla prossima prop boundary o tag boundary) e accumula tutti
/// gli errori in un `Vec<ParseError>`.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// Descrizione leggibile del problema.
    pub message: String,
    /// Posizione nel sorgente dove è stato rilevato l'errore.
    pub pos: Position,
}

/// Errore semantico prodotto dall'analyzer.
///
/// L'analyzer raccoglie tutti gli errori semantici (CLT101–104) in un
/// `Vec<AnalyzerError>` senza fermarsi al primo. Una lista vuota indica
/// che il file è valido e può procedere al codegen.
#[derive(Debug, Clone, PartialEq)]
pub struct AnalyzerError {
    /// Descrizione leggibile del problema, prefissata dal codice errore
    /// (es. `"CLT102: invalid value 'xl2' for prop 'gap' on 'Column'. Valid values: …"`).
    pub message: String,
    /// Posizione nel sorgente dove è stato rilevato l'errore.
    pub pos: Position,
}
