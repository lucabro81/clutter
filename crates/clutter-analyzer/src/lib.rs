//! Semantic analyzer for the Clutter compiler.
//!
//! # Responsabilità
//!
//! L'analyzer è il terzo stadio della pipeline di compilazione:
//!
//! ```text
//! .clutter  →  Lexer  →  Parser  →  **Analyzer**  →  Codegen
//! ```
//!
//! Riceve un [`ProgramNode`] (output del parser) e un [`DesignTokens`] (caricato da
//! `tokens.json`) e produce una lista di [`AnalyzerError`]. Una lista vuota significa
//! che il file sorgente è semanticamente valido.
//!
//! # Errori prodotti
//!
//! | Codice  | Causa                                                             |
//! |---------|-------------------------------------------------------------------|
//! | CLT101  | Prop sconosciuta su un componente noto (es. `color` su `Column`) |
//! | CLT102  | Valore stringa non presente nel design system o nell'enum fisso   |
//! | CLT103  | Componente non appartenente al vocabolario chiuso                 |
//! | CLT104  | Identificatore usato in un'espressione non dichiarato nel logic block |
//!
//! # Regole di validazione
//!
//! ## Prop type checking (CLT101–103)
//!
//! Ogni prop con un valore stringa letterale viene confrontata con il design system.
//! La mappa prop → categoria è hardcoded in [`prop_map`] per il POC; tutti i token
//! validi per una categoria vengono letti da [`DesignTokens`].
//!
//! ## Reference checking (CLT104)
//!
//! Ogni espressione `{nome}` nel template — sia come [`Node::Expr`] che come
//! [`PropValue::ExpressionValue`] — viene verificata contro il set di identificatori
//! dichiarati nel logic block TypeScript. Gli identificatori vengono estratti con una
//! scansione shallow tramite [`extract_identifiers`].
//!
//! L'alias introdotto da `<each collection={…} as="alias">` viene aggiunto al set degli
//! identificatori validi per i soli figli di quel nodo.
//!
//! ## Unsafe validation (CLT105–106)
//!
//! Non ancora implementata: richiede il supporto di `<unsafe>` nel lexer e nel parser.
//! Vedere il backlog per i dettagli.
//!
//! # Utilizzo
//!
//! ```ignore
//! let json = std::fs::read_to_string("tokens.json")?;
//! let tokens = DesignTokens::from_str(&json)?;
//! let errors = analyze(&program, &tokens);
//! if errors.is_empty() {
//!     // procedi con il codegen
//! }
//! ```

use std::collections::HashSet;

use clutter_runtime::{
    AnalyzerError, ComponentNode, EachNode, IfNode, Node, Position, PropNode, PropValue,
    ProgramNode,
};
use serde::Deserialize;

/// Vocabolario chiuso dei componenti riconosciuti dall'analyzer.
///
/// Un componente non presente in questa lista produce un errore CLT103. I figli
/// vengono comunque analizzati ricorsivamente per raccogliere tutti gli errori presenti.
const KNOWN_COMPONENTS: &[&str] = &["Column", "Row", "Box", "Text", "Button", "Input"];

// ---------------------------------------------------------------------------
// Punto di ingresso pubblico
// ---------------------------------------------------------------------------

/// Analizza semanticamente un programma Clutter e restituisce tutti gli errori trovati.
///
/// Questa è la funzione pubblica del crate e rappresenta l'intera fase di analisi.
/// Viene chiamata dopo che lexer e parser hanno prodotto un [`ProgramNode`] senza errori.
///
/// # Algoritmo
///
/// 1. Estrae gli identificatori dichiarati nel logic block TypeScript.
/// 2. Visita ricorsivamente tutti i nodi del template tramite [`analyze_nodes`].
/// 3. Restituisce la lista completa degli errori (non si ferma al primo).
///
/// # Restituisce
///
/// Un [`Vec<AnalyzerError>`] vuoto indica che il file è valido. Ogni elemento
/// descrive un singolo problema semantico con messaggio e posizione nel sorgente.
pub fn analyze(program: &ProgramNode, tokens: &DesignTokens) -> Vec<AnalyzerError> {
    let identifiers = extract_identifiers(&program.logic_block);
    let mut errors = Vec::new();
    analyze_nodes(&program.template, tokens, &identifiers, &mut errors);
    errors
}

// ---------------------------------------------------------------------------
// Walker ricorsivo
// ---------------------------------------------------------------------------

/// Visita una slice di nodi e accumula gli errori trovati.
///
/// Smista ogni [`Node`] al validatore specifico:
///
/// - [`Node::Component`] → [`analyze_component`]
/// - [`Node::Expr`] → controllo CLT104 sull'identificatore
/// - [`Node::If`] → [`analyze_if`]
/// - [`Node::Each`] → [`analyze_each`]
/// - [`Node::Text`] → nessuna azione (testo statico, niente da validare)
fn analyze_nodes(
    nodes: &[Node],
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    for node in nodes {
        match node {
            Node::Component(c) => analyze_component(c, tokens, identifiers, errors),
            Node::Expr(e) => {
                if let Some(err) = check_reference(&e.value, &e.pos, identifiers) {
                    errors.push(err);
                }
            }
            Node::If(i) => analyze_if(i, tokens, identifiers, errors),
            Node::Each(e) => analyze_each(e, tokens, identifiers, errors),
            Node::Text(_) => {}
        }
    }
}

/// Valida un nodo componente: controlla il nome, le props e ricorre nei figli.
///
/// # Logica
///
/// 1. Se il nome non è in [`KNOWN_COMPONENTS`] → errore CLT103; le props vengono
///    saltate (non ha senso validarle per un componente sconosciuto), ma i figli
///    vengono comunque analizzati per raccogliere tutti gli errori possibili.
/// 2. Se il componente è noto → ogni prop viene validata con [`validate_prop`].
/// 3. In entrambi i casi si ricorre nei figli con lo stesso set di identificatori.
fn analyze_component(
    node: &ComponentNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    if !KNOWN_COMPONENTS.contains(&node.name.as_str()) {
        errors.push(AnalyzerError {
            message: format!("CLT103: unknown component '{}'", node.name),
            pos: node.pos.clone(),
        });
        // Still recurse into children even for unknown components
    } else {
        for prop in &node.props {
            errors.extend(validate_prop(&node.name, prop, tokens, identifiers));
        }
    }
    analyze_nodes(&node.children, tokens, identifiers, errors);
}

/// Valida una singola prop e restituisce zero o più errori.
///
/// La logica dipende da ciò che [`prop_map`] restituisce per la coppia
/// `(component, prop.name)`:
///
/// | Risultato di `prop_map` | Azione |
/// |-------------------------|--------|
/// | `None` | CLT101: prop sconosciuta sul componente |
/// | `Some(AnyValue)` | Nessun controllo di valore; se il valore è un'espressione → CLT104 |
/// | `Some(Tokens(cat))` | Se stringa: verifica contro `tokens.valid_values(cat)` → CLT102 se assente; se espressione → CLT104 |
/// | `Some(Enum(vals))` | Se stringa: verifica contro la lista fissa `vals` → CLT102 se assente; se espressione → CLT104 |
///
/// Le [`PropValue::ExpressionValue`] non vengono mai validate contro i token di design
/// perché il loro valore è determinato a runtime: vengono invece controllate come
/// riferimenti a identificatori (CLT104).
fn validate_prop(
    component: &str,
    prop: &PropNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
) -> Vec<AnalyzerError> {
    let mut errors = Vec::new();
    match prop_map(component, &prop.name) {
        None => {
            errors.push(AnalyzerError {
                message: format!(
                    "CLT101: unknown prop '{}' on '{}'",
                    prop.name, component
                ),
                pos: prop.pos.clone(),
            });
        }
        Some(PropValidation::AnyValue) => {
            if let PropValue::ExpressionValue(ref name) = prop.value {
                if let Some(err) = check_reference(name, &prop.pos, identifiers) {
                    errors.push(err);
                }
            }
        }
        Some(PropValidation::Tokens(cat)) => match &prop.value {
            PropValue::StringValue(val) => {
                let valid = tokens.valid_values(cat);
                if !valid.contains(val) {
                    errors.push(AnalyzerError {
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val,
                            prop.name,
                            component,
                            valid.join(", ")
                        ),
                        pos: prop.pos.clone(),
                    });
                }
            }
            PropValue::ExpressionValue(name) => {
                if let Some(err) = check_reference(name, &prop.pos, identifiers) {
                    errors.push(err);
                }
            }
        },
        Some(PropValidation::Enum(vals)) => match &prop.value {
            PropValue::StringValue(val) => {
                if !vals.contains(&val.as_str()) {
                    errors.push(AnalyzerError {
                        message: format!(
                            "CLT102: invalid value '{}' for prop '{}' on '{}'. Valid values: {}",
                            val,
                            prop.name,
                            component,
                            vals.join(", ")
                        ),
                        pos: prop.pos.clone(),
                    });
                }
            }
            PropValue::ExpressionValue(name) => {
                if let Some(err) = check_reference(name, &prop.pos, identifiers) {
                    errors.push(err);
                }
            }
        },
    }
    errors
}

/// Valida un nodo `<if>`.
///
/// Controlla che l'espressione nella `condition` sia un identificatore dichiarato
/// (CLT104), poi ricorre sia nel ramo `then` che nell'eventuale ramo `else`.
/// Il set di identificatori non viene esteso: `<if>` non introduce nuovi binding.
fn analyze_if(
    node: &IfNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    if let Some(err) = check_reference(&node.condition, &node.pos, identifiers) {
        errors.push(err);
    }
    analyze_nodes(&node.then_children, tokens, identifiers, errors);
    if let Some(else_children) = &node.else_children {
        analyze_nodes(else_children, tokens, identifiers, errors);
    }
}

/// Valida un nodo `<each>`.
///
/// Controlla che `collection` sia un identificatore dichiarato (CLT104), poi
/// ricorre nei figli con un set di identificatori **esteso** con l'alias del ciclo.
///
/// L'alias (`node.alias`) è un binding introdotto dall'`<each>` stesso — ad esempio
/// `<each collection={items} as="item">` porta `"item"` in scope per tutti i figli.
/// Non sarebbe corretto segnalare CLT104 per `{item}` usato dentro il ciclo.
fn analyze_each(
    node: &EachNode,
    tokens: &DesignTokens,
    identifiers: &HashSet<String>,
    errors: &mut Vec<AnalyzerError>,
) {
    if let Some(err) = check_reference(&node.collection, &node.pos, identifiers) {
        errors.push(err);
    }
    // The alias is in scope for children only — clone to avoid polluting the outer scope.
    let mut child_ids = identifiers.clone();
    child_ids.insert(node.alias.clone());
    analyze_nodes(&node.children, tokens, &child_ids, errors);
}

/// Verifica che `name` sia presente nel set degli identificatori dichiarati.
///
/// Restituisce `None` se il riferimento è valido, `Some(AnalyzerError)` con codice
/// CLT104 altrimenti. Usato da [`validate_prop`], [`analyze_if`], [`analyze_each`]
/// e direttamente da [`analyze_nodes`] per i nodi [`Node::Expr`].
fn check_reference(name: &str, pos: &Position, identifiers: &HashSet<String>) -> Option<AnalyzerError> {
    if identifiers.contains(name) {
        None
    } else {
        Some(AnalyzerError {
            message: format!("CLT104: undeclared identifier '{}'", name),
            pos: pos.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tipi interni
// ---------------------------------------------------------------------------

/// Categoria di token di design a cui una prop può fare riferimento.
///
/// Usato da [`PropValidation::Tokens`] per indirizzare la ricerca dei valori
/// validi in [`DesignTokens::valid_values`].
#[derive(Debug, Clone, Copy)]
enum TokenCategory {
    /// Spaziatura: gap, padding, margin. Es. `xs | sm | md | lg | xl | xxl`.
    Spacing,
    /// Colori semantici. Es. `primary | secondary | danger | surface | background`.
    Color,
    /// Dimensioni tipografiche. Es. `xs | sm | base | lg | xl | xxl`.
    FontSize,
    /// Pesi tipografici. Es. `normal | medium | semibold | bold`.
    FontWeight,
    /// Raggi di bordo. Es. `none | sm | md | lg | full`.
    Radius,
    /// Ombre. Es. `sm | md | lg`.
    Shadow,
}

/// Struttura interna al JSON di `tokens.json` per la sezione tipografica.
#[derive(Debug, Deserialize)]
struct Typography {
    sizes: Vec<String>,
    weights: Vec<String>,
}

/// Design system deserializzato da `tokens.json`.
///
/// Contiene i valori validi per ogni categoria di token. Viene costruito una volta
/// sola alla chiamata di [`analyze`] e passato in sola lettura a tutta la visita.
///
/// # Formato JSON atteso
///
/// ```json
/// {
///   "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
///   "colors":     ["primary", "secondary", "danger", "surface", "background"],
///   "typography": { "sizes": [...], "weights": [...] },
///   "radii":      ["none", "sm", "md", "lg", "full"],
///   "shadows":    ["sm", "md", "lg"]
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct DesignTokens {
    spacing: Vec<String>,
    colors: Vec<String>,
    typography: Typography,
    radii: Vec<String>,
    shadows: Vec<String>,
}

/// Tipo di validazione applicabile a una prop del vocabolario chiuso.
///
/// [`prop_map`] restituisce un `Option<PropValidation>`: `None` indica che la prop
/// non è riconosciuta sul componente dato (→ CLT101).
enum PropValidation {
    /// Il valore deve essere presente in una categoria del design system.
    Tokens(TokenCategory),
    /// Il valore deve essere uno degli elementi dell'insieme fisso fornito.
    Enum(&'static [&'static str]),
    /// La prop è valida con qualsiasi valore stringa; se è un'espressione, il nome
    /// dell'identificatore viene comunque sottoposto a controllo CLT104.
    AnyValue,
}

// ---------------------------------------------------------------------------
// Estrazione identificatori
// ---------------------------------------------------------------------------

/// Estrae gli identificatori dichiarati nel logic block TypeScript.
///
/// Esegue una scansione shallow basata su token di parole chiave: cattura il nome
/// che segue immediatamente `const`, `let`, `var`, `function` o `component`.
///
/// # Limitazioni note
///
/// Questa implementazione è intenzionalmente approssimativa e adatta al POC:
///
/// - **Destructuring**: `const { a, b } = obj` → né `a` né `b` vengono estratti.
/// - **Import**: `import foo from "bar"` → `foo` non viene estratto.
/// - **Alias di tipo** e variabili di closure non vengono riconosciuti.
///
/// Questi casi sono documentati nel backlog come *known limitation*.
fn extract_identifiers(logic_block: &str) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    let mut prev = "";
    for token in logic_block.split_whitespace() {
        // Take only the leading identifier portion: "handleClick(" → "handleClick"
        let name = token.split(|c: char| !c.is_alphanumeric() && c != '_').next().unwrap_or("");
        if matches!(prev, "const" | "let" | "var" | "function" | "component") && !name.is_empty() {
            ids.insert(name.to_string());
        }
        prev = token;
    }
    ids
}

// ---------------------------------------------------------------------------
// Mappa delle props — vocabolario chiuso
// ---------------------------------------------------------------------------

/// Restituisce la regola di validazione per la coppia `(componente, prop)`.
///
/// # Restituisce
///
/// - `Some(PropValidation)` se la prop è riconosciuta sul componente dato.
/// - `None` in due casi distinti, indistinguibili dalla firma ma gestiti diversamente
///   dal chiamante [`validate_prop`]:
///   - La prop non esiste sul componente (es. `color` su `Column`) → CLT101.
///   - Il componente stesso non è nel vocabolario (es. `Grid`) → CLT103 emesso
///     prima di chiamare questa funzione, quindi `None` qui non viene mai raggiunto
///     per componenti sconosciuti.
///
/// # Estensibilità
///
/// La mappa è hardcoded per il POC. L'introduzione di nuovi componenti built-in o
/// di props dinamiche è discussa nel backlog ("Prop map dinamica / componenti custom").
fn prop_map(component: &str, prop: &str) -> Option<PropValidation> {
    use PropValidation::*;
    use TokenCategory::*;

    const LAYOUT_AXES: &[&str] = &["start", "end", "center", "spaceBetween", "spaceAround", "spaceEvenly"];
    const CROSS_AXES:  &[&str] = &["start", "end", "center", "stretch"];
    const ALIGNS:      &[&str] = &["left", "center", "right"];
    const BTN_VARIANTS: &[&str] = &["primary", "secondary", "outline", "ghost", "danger"];
    const BTN_SIZES:    &[&str] = &["sm", "md", "lg"];
    const INPUT_TYPES:  &[&str] = &["text", "email", "password", "number"];

    match (component, prop) {
        ("Column" | "Row", "gap" | "padding") => Some(Tokens(Spacing)),
        ("Column" | "Row", "mainAxis")        => Some(Enum(LAYOUT_AXES)),
        ("Column" | "Row", "crossAxis")       => Some(Enum(CROSS_AXES)),
        ("Text", "value")                     => Some(AnyValue),
        ("Text", "size")                      => Some(Tokens(FontSize)),
        ("Text", "weight")                    => Some(Tokens(FontWeight)),
        ("Text", "color")                     => Some(Tokens(Color)),
        ("Text", "align")                     => Some(Enum(ALIGNS)),
        ("Button", "variant")                 => Some(Enum(BTN_VARIANTS)),
        ("Button", "size")                    => Some(Enum(BTN_SIZES)),
        ("Button", "disabled")                => Some(AnyValue),
        ("Box", "bg")                         => Some(Tokens(Color)),
        ("Box", "padding" | "margin")         => Some(Tokens(Spacing)),
        ("Box", "radius")                     => Some(Tokens(Radius)),
        ("Box", "shadow")                     => Some(Tokens(Shadow)),
        ("Input", "placeholder" | "value")    => Some(AnyValue),
        ("Input", "type")                     => Some(Enum(INPUT_TYPES)),
        ("Column" | "Row" | "Text" | "Button" | "Box" | "Input", _) => None, // known component, unknown prop
        _ => None, // unknown component
    }
}

// ---------------------------------------------------------------------------
// DesignTokens — impl
// ---------------------------------------------------------------------------

impl DesignTokens {
    /// Deserializza un [`DesignTokens`] da una stringa JSON.
    ///
    /// # Errori
    ///
    /// Restituisce un [`serde_json::Error`] se il JSON è malformato o mancano
    /// campi obbligatori (`spacing`, `colors`, `typography`, `radii`, `shadows`).
    pub fn from_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Restituisce la slice dei valori validi per la categoria richiesta.
    ///
    /// Usato da [`validate_prop`] per confrontare il valore della prop e per
    /// costruire il messaggio di errore CLT102 con la lista dei valori accettati.
    pub(crate) fn valid_values(&self, category: TokenCategory) -> &[String] {
        match category {
            TokenCategory::Spacing    => &self.spacing,
            TokenCategory::Color      => &self.colors,
            TokenCategory::FontSize   => &self.typography.sizes,
            TokenCategory::FontWeight => &self.typography.weights,
            TokenCategory::Radius     => &self.radii,
            TokenCategory::Shadow     => &self.shadows,
        }
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tokens() -> DesignTokens {
        DesignTokens::from_str(r#"{
            "spacing":    ["xs", "sm", "md", "lg", "xl", "xxl"],
            "colors":     ["primary", "secondary", "danger", "surface", "background"],
            "typography": {
                "sizes":   ["xs", "sm", "base", "lg", "xl", "xxl"],
                "weights": ["normal", "medium", "semibold", "bold"]
            },
            "radii":   ["none", "sm", "md", "lg", "full"],
            "shadows": ["sm", "md", "lg"]
        }"#).unwrap()
    }

    #[test]
    fn design_tokens_parses_valid_json() {
        let t = test_tokens();
        assert!(t.valid_values(TokenCategory::Spacing).contains(&"md".to_string()));
        assert!(t.valid_values(TokenCategory::Color).contains(&"primary".to_string()));
        assert!(t.valid_values(TokenCategory::FontSize).contains(&"lg".to_string()));
        assert!(t.valid_values(TokenCategory::FontWeight).contains(&"bold".to_string()));
        assert!(t.valid_values(TokenCategory::Radius).contains(&"full".to_string()));
        assert!(t.valid_values(TokenCategory::Shadow).contains(&"sm".to_string()));
    }

    #[test]
    fn design_tokens_rejects_invalid_json() {
        assert!(DesignTokens::from_str("not json").is_err());
    }

    // --- prop_map ---

    #[test]
    fn prop_map_known_token_prop() {
        assert!(matches!(prop_map("Column", "gap"), Some(PropValidation::Tokens(TokenCategory::Spacing))));
        assert!(matches!(prop_map("Text", "size"), Some(PropValidation::Tokens(TokenCategory::FontSize))));
        assert!(matches!(prop_map("Box", "bg"), Some(PropValidation::Tokens(TokenCategory::Color))));
    }

    #[test]
    fn prop_map_known_enum_prop() {
        assert!(matches!(prop_map("Column", "mainAxis"), Some(PropValidation::Enum(_))));
        assert!(matches!(prop_map("Text", "align"), Some(PropValidation::Enum(_))));
        assert!(matches!(prop_map("Button", "variant"), Some(PropValidation::Enum(_))));
    }

    #[test]
    fn prop_map_any_value_prop() {
        assert!(matches!(prop_map("Text", "value"), Some(PropValidation::AnyValue)));
        assert!(matches!(prop_map("Button", "disabled"), Some(PropValidation::AnyValue)));
        assert!(matches!(prop_map("Input", "placeholder"), Some(PropValidation::AnyValue)));
    }

    #[test]
    fn prop_map_unknown_component_returns_none() {
        assert!(prop_map("Grid", "gap").is_none());
    }

    #[test]
    fn prop_map_unknown_prop_on_known_component_returns_none() {
        assert!(prop_map("Column", "color").is_none());
        assert!(prop_map("Text", "border").is_none());
    }

    // --- analyze() helpers ---

    use clutter_runtime::{
        ComponentNode, EachNode, ExpressionNode, IfNode, Node, Position, ProgramNode, PropNode,
        PropValue,
    };

    fn pos() -> Position {
        Position { line: 1, col: 1 }
    }

    fn program(logic_block: &str, template: Vec<Node>) -> ProgramNode {
        ProgramNode { logic_block: logic_block.to_string(), template }
    }

    fn component(name: &str, props: Vec<PropNode>, children: Vec<Node>) -> Node {
        Node::Component(ComponentNode { name: name.to_string(), props, children, pos: pos() })
    }

    fn prop_str(name: &str, value: &str) -> PropNode {
        PropNode { name: name.to_string(), value: PropValue::StringValue(value.to_string()), pos: pos() }
    }

    fn prop_expr(name: &str, expr: &str) -> PropNode {
        PropNode { name: name.to_string(), value: PropValue::ExpressionValue(expr.to_string()), pos: pos() }
    }

    fn expr_node(value: &str) -> Node {
        Node::Expr(ExpressionNode { value: value.to_string(), pos: pos() })
    }

    fn if_node(condition: &str, then_children: Vec<Node>) -> Node {
        Node::If(IfNode { condition: condition.to_string(), then_children, else_children: None, pos: pos() })
    }

    fn each_node(collection: &str, alias: &str, children: Vec<Node>) -> Node {
        Node::Each(EachNode {
            collection: collection.to_string(),
            alias: alias.to_string(),
            children,
            pos: pos(),
        })
    }

    // 1. Valid prop value → no errors
    #[test]
    fn analyze_valid_prop_no_errors() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_str("gap", "md")], vec![])]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 2. Invalid prop value → CLT102 with message listing valid values
    #[test]
    fn analyze_invalid_token_value_error() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_str("gap", "xl2")], vec![])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("xl2"), "message should mention the bad value");
        assert!(errors[0].message.contains("gap"), "message should mention the prop name");
    }

    // 3. ExpressionValue prop with known identifier → no errors
    #[test]
    fn analyze_expression_prop_known_ident_no_errors() {
        let t = test_tokens();
        let p = program("const myVar = 4;", vec![
            component("Column", vec![prop_expr("gap", "myVar")], vec![]),
        ]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 4. ExpressionValue prop with unknown identifier → CLT104
    #[test]
    fn analyze_expression_prop_unknown_ident_error() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_expr("gap", "unknown")], vec![])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("unknown"), "message should mention the identifier");
    }

    // 5. Unknown component → CLT103
    #[test]
    fn analyze_unknown_component_error() {
        let t = test_tokens();
        let p = program("", vec![component("Grid", vec![], vec![])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Grid"));
    }

    // 6. Unknown prop on known component → CLT101
    #[test]
    fn analyze_unknown_prop_on_known_component_error() {
        let t = test_tokens();
        let p = program("", vec![component("Column", vec![prop_str("color", "primary")], vec![])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("color"));
        assert!(errors[0].message.contains("Column"));
    }

    // 7. Multiple errors collected
    #[test]
    fn analyze_multiple_errors_collected() {
        let t = test_tokens();
        let p = program("", vec![
            component("Column", vec![prop_str("gap", "bad1")], vec![]),
            component("Column", vec![prop_str("gap", "bad2")], vec![]),
        ]);
        assert_eq!(analyze(&p, &t).len(), 2);
    }

    // 8. Nested component — props validated the same way
    #[test]
    fn analyze_nested_component_props_validated() {
        let t = test_tokens();
        let inner = component("Text", vec![prop_str("size", "huge")], vec![]);
        let p = program("", vec![component("Column", vec![], vec![inner])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("huge"));
    }

    // 9. Children of <if>/<each> validated recursively
    #[test]
    fn analyze_if_each_children_validated() {
        let t = test_tokens();
        let bad_child = component("Text", vec![prop_str("size", "nope")], vec![]);
        let p = program("const flag = true;", vec![if_node("flag", vec![bad_child])]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nope"));
    }

    // 10. Empty template → no errors
    #[test]
    fn analyze_empty_template_no_errors() {
        let t = test_tokens();
        let p = program("", vec![]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 11. ExpressionNode with known identifier → no errors
    #[test]
    fn analyze_expression_node_known_ident_no_errors() {
        let t = test_tokens();
        let p = program("const title = \"Hello\";", vec![expr_node("title")]);
        assert!(analyze(&p, &t).is_empty());
    }

    // 12. ExpressionNode with unknown identifier → CLT104
    #[test]
    fn analyze_expression_node_unknown_ident_error() {
        let t = test_tokens();
        let p = program("", vec![expr_node("foo")]);
        let errors = analyze(&p, &t);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("foo"));
    }

    // 13. <each> alias in scope for children → no CLT104
    #[test]
    fn analyze_each_alias_in_scope_for_children() {
        let t = test_tokens();
        // each collection={items} as="item" → "item" must be in scope for children
        let child = component("Text", vec![prop_expr("value", "item")], vec![]);
        let p = program("const items = [];", vec![
            each_node("items", "item", vec![child]),
        ]);
        assert!(analyze(&p, &t).is_empty());
    }

    // --- extract_identifiers ---

    #[test]
    fn extract_identifiers_const_let_var() {
        let ids = extract_identifiers("const title = \"Hello\";\nlet count = 0;\nvar flag = true;");
        assert!(ids.contains("title"));
        assert!(ids.contains("count"));
        assert!(ids.contains("flag"));
    }

    #[test]
    fn extract_identifiers_function_and_component() {
        let ids = extract_identifiers("function handleClick() {}\ncomponent Card(props) {}");
        assert!(ids.contains("handleClick"));
        assert!(ids.contains("Card"));
    }

    #[test]
    fn extract_identifiers_empty_logic_block() {
        assert!(extract_identifiers("").is_empty());
    }

    #[test]
    fn extract_identifiers_does_not_include_values() {
        let ids = extract_identifiers("const title = \"Hello\";");
        assert!(!ids.contains("Hello"));
    }
}
