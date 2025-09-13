
pub use crate::grammar_parser::OutSpec;
pub use crate::grammar_parser::ValueSpec;
use crate::parser::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} - {}]", self.start, self.end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol<'gr> {
    Terminal(&'gr str),
    Placeholder { name: &'gr str, typ: &'gr str },
    NonTerminal(&'gr str),
}

impl<'gr> Symbol<'gr> {
    pub fn is_terminal(&self) -> bool {
        match self {
            Symbol::Terminal(_) => true,
            _ => false,
        }
    }
}

use std::fmt;

impl<'gr> fmt::Display for Symbol<'gr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Symbol::Terminal(s) => write!(f, "{}", s),
            Symbol::Placeholder { name, typ } => write!(f, "<{}:{}>", name, typ),
            Symbol::NonTerminal(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Production<'gr> {
    pub lhs: &'gr str,
    pub rhs: Vec<Symbol<'gr>>,
    pub out: OutSpec<'gr>,
}

#[derive(Debug, Clone)]
pub struct Grammar<'gr> {
    pub productions: Vec<Production<'gr>>,
}

impl<'gr> Grammar<'gr> {
    pub fn compute_nullable(&self) -> HashSet<&'gr str> {
        let mut nullable = HashSet::new();
        let mut changed = true;

        while changed {
            changed = false;

            for prod in &self.productions {
                // If LHS is already nullable, skip
                if nullable.contains(prod.lhs) {
                    continue;
                }

                // Check if all RHS symbols are nullable
                let all_nullable = prod.rhs.iter().all(|sym| match sym {
                    Symbol::NonTerminal(nt) => nullable.contains(nt),
                    Symbol::Placeholder { name: _, typ } => nullable.contains(typ),
                    Symbol::Terminal(_) => false, // Terminals are never nullable
                });

                if all_nullable {
                    nullable.insert(prod.lhs);
                    changed = true;
                }
            }
        }

        nullable
    }
}

impl<'gr> Grammar<'gr> {
    pub fn prods_for(&'_ self, name: &str) -> Vec<(usize, &Production<'gr>)> {
        self.productions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.lhs == name)
            .map(|(i, p)| (i, p))
            .collect()
    }
}

impl<'gr> Grammar<'gr> {
    /// Detect whether the grammar contains an infinite nullable cycle (a cycle
    /// entirely through nullable nonterminals / placeholder types).
    pub fn has_infinite_loop(&self) -> bool {
        use std::collections::{HashMap, HashSet};

        // 1 compute nullable set (nonterminal names that can produce epsilon)
        let null_set: HashSet<&'gr str> = self.compute_nullable();

        // quick exit: nothing nullable -> no nullable cycles
        if null_set.is_empty() {
            return false;
        }

        // 2 build adjacency map for nullable symbols:
        //    for each nullable symbol `A`, find productions A -> rhs where
        //    every symbol of rhs is nullable; from such a rhs gather all
        //    nonterminals and placeholder types that appear -> edges A -> sym.
        let mut adj: HashMap<&'gr str, Vec<&'gr str>> = HashMap::new();

        for &sym in &null_set {
            let mut children: HashSet<&'gr str> = HashSet::new();

            for (_pid, prod) in self.prods_for(sym) {
                // check if whole rhs is nullable
                let rhs_all_nullable = prod.rhs.iter().all(|s| match s {
                    Symbol::NonTerminal(nt) => null_set.contains(nt),
                    Symbol::Placeholder { name: _, typ } => null_set.contains(typ),
                    Symbol::Terminal(_) => false,
                });

                if rhs_all_nullable {
                    // gather nonterminals / placeholder types from rhs
                    for s in &prod.rhs {
                        match s {
                            Symbol::NonTerminal(nt) => {
                                children.insert(nt);
                            }
                            Symbol::Placeholder { name: _, typ } => {
                                children.insert(typ);
                            }
                            Symbol::Terminal(_) => { /* terminals shouldn't appear here */ }
                        }
                    }
                }
            }

            // keep only children that are in the nullable set (we only care about cycles among nullable symbols)
            let filtered: Vec<&'gr str> = children
                .into_iter()
                .filter(|c| null_set.contains(c))
                .collect();
            adj.insert(sym, filtered);
        }

        // 3 detect a directed cycle reachable from some node in null_set.
        //    Use DFS with coloring (0 = unvisited, 1 = visiting, 2 = visited)
        let mut color: HashMap<&'gr str, u8> = HashMap::new();
        for &s in &null_set {
            color.insert(s, 0);
        }

        fn dfs<'a>(
            v: &'a str,
            adj: &HashMap<&'a str, Vec<&'a str>>,
            color: &mut HashMap<&'a str, u8>,
        ) -> bool {
            color.insert(v, 1); // visiting
            if let Some(neighs) = adj.get(v) {
                for &w in neighs {
                    match color.get(w).copied().unwrap_or(0) {
                        0 => {
                            if dfs(w, adj, color) {
                                return true;
                            }
                        }
                        1 => {
                            // found back-edge -> cycle
                            return true;
                        }
                        _ => {}
                    }
                }
            }
            color.insert(v, 2); // done
            false
        }

        for &s in &null_set {
            if color.get(s).copied().unwrap_or(0) == 0 {
                if dfs(s, &adj, &mut color) {
                    return true;
                }
            }
        }

        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemKey {
    pub prod_id: usize,
    pub dot: usize,
    pub start: usize,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub key: ItemKey,
}

impl Item {
    pub fn new(prod_id: usize, dot: usize, start: usize) -> Self {
        Item {
            key: ItemKey {
                prod_id,
                dot,
                start,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Char,
    Int,
    Float,
    StringLit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'inp> {
    pub kind: TokenKind,
    pub text: &'inp str,
    pub span: Span,
}

impl<'inp> Token<'inp> {
    /// Convert a token into a semantic value if it carries one.
    /// Returns `None` for purely structural tokens like `Char`.
    pub fn get_value<'gr>(&self) -> Option<Value<'gr, 'inp>> {
        match self.kind {
            TokenKind::Int => Some(Value::Integer(self.text.parse::<i64>().ok()?)),
            TokenKind::Float => Some(Value::Float(self.text.parse::<f64>().ok()?)),
            TokenKind::StringLit => Some(Value::String(self.text)),
            TokenKind::Char => None, // structural only
        }
    }
}

pub fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = vec![];
    let mut byte_pos = 0;
    let input_len = input.len();

    while byte_pos < input_len {
        let c = input[byte_pos..].chars().next().unwrap();
        let char_len = c.len_utf8();
        let start = byte_pos;

        // String literal
        if c == '"' {
            byte_pos += char_len;
            let str_start = byte_pos;
            while byte_pos < input_len {
                let ch = input[byte_pos..].chars().next().unwrap();
                if ch == '"' {
                    break;
                }
                byte_pos += ch.len_utf8();
            }
            let str_end = byte_pos;
            let text = &input[str_start..str_end];
            tokens.push(Token {
                kind: TokenKind::StringLit,
                text,
                span: Span::new(start, str_end + 1),
            });
            byte_pos += 1; // skip closing quote
            continue;
        }

        // Number parsing (int or float)
        if c.is_ascii_digit() {
            let mut end_pos = byte_pos;
            while end_pos < input_len {
                let ch = input[end_pos..].chars().next().unwrap();
                if !ch.is_ascii_digit() && ch != '.' {
                    break;
                }
                end_pos += ch.len_utf8();
            }
            let raw = &input[byte_pos..end_pos];
            if raw.parse::<i64>().is_ok() {
                tokens.push(Token {
                    kind: TokenKind::Int,
                    text: raw,
                    span: Span::new(byte_pos, end_pos),
                });
            } else if raw.parse::<f64>().is_ok() {
                tokens.push(Token {
                    kind: TokenKind::Float,
                    text: raw,
                    span: Span::new(byte_pos, end_pos),
                });
            } else {
                for ch in raw.chars() {
                    let ch_start = byte_pos;
                    let ch_end = ch_start + ch.len_utf8();
                    tokens.push(Token {
                        kind: TokenKind::Char,
                        text: &input[ch_start..ch_end],
                        span: Span::new(ch_start, ch_end),
                    });
                    byte_pos = ch_end;
                }
            }
            byte_pos = end_pos;
            continue;
        }

        // Default: single char token
        tokens.push(Token {
            kind: TokenKind::Char,
            text: &input[start..start + char_len],
            span: Span::new(start, start + char_len),
        });
        byte_pos += char_len;
    }

    tokens
}

pub fn is_builtin(typ: &str, tok: &Token<'_>) -> bool {
    match typ.to_ascii_lowercase().as_str() {
        "int" => tok.kind == TokenKind::Int,
        "float" => tok.kind == TokenKind::Float,
        "string" | "str" => tok.kind == TokenKind::StringLit,
        _ => false,
    }
}

pub struct Chart<'gr, 'inp> {
    pub sets: Vec<HashMap<ItemKey, Item>>,
    pub tokens: Vec<Token<'inp>>,
    pub grammar: &'gr Grammar<'gr>,
    pub start: &'inp str,
}

impl<'gr, 'inp> Chart<'gr, 'inp> {
    /// Advance the dot over any nullable symbols starting at the current dot position.
    pub fn add_nullable_items(&mut self, mut item: Item, pos: usize, nullable: &HashSet<&'gr str>) {
        let prod = &self.grammar.productions[item.key.prod_id];
        let mut dot = item.key.dot;

        while dot < prod.rhs.len() {
            let sym = &prod.rhs[dot];
            let is_nullable = match sym {
                Symbol::NonTerminal(nt) => nullable.contains(nt),
                Symbol::Placeholder { name: _, typ } => nullable.contains(typ),
                Symbol::Terminal(_) => false,
            };

            if !is_nullable {
                break;
            }

            // Advance dot
            dot += 1;
            let new_item = Item::new(item.key.prod_id, dot, item.key.start);

            if self.add_item(pos, new_item.clone()) {
                // Continue with the new item for subsequent nullables
                item = new_item;
            } else {
                break;
            }
        }
    }
}

impl<'gr, 'inp> Chart<'gr, 'inp> {
    pub fn new(grammar: &'gr Grammar<'gr>, tokens: Vec<Token<'inp>>, start: &'inp str) -> Self {
        let n = tokens.len();
        let mut sets = Vec::with_capacity(n + 1);
        for _ in 0..=n {
            sets.push(HashMap::new());
        }
        Self {
            sets,
            tokens,
            grammar,
            start,
        }
    }

    pub fn add_item(&mut self, pos: usize, item: Item) -> bool {
        let key = item.key.clone();
        if self.sets[pos].contains_key(&key) {
            false
        } else {
            self.sets[pos].insert(key, item);
            true
        }
    }

    pub fn recognize(&mut self, start: &str) {
        // Precompute nullable nonterminals
        let nullable = self.grammar.compute_nullable();

        // Initialize chart with start productions
        for (pid, _) in self.grammar.prods_for(start) {
            let it = Item::new(pid, 0, 0);
            self.add_item(0, it.clone());
            // Advance dot for nullable prefixes
            self.add_nullable_items(it, 0, &nullable);
        }

        let n = self.tokens.len();
        for pos in 0..=n {
            let mut changed = true;
            while changed {
                changed = false;
                let keys: Vec<ItemKey> = self.sets[pos].keys().cloned().collect();

                for key in keys {
                    let item = match self.sets[pos].get(&key) {
                        Some(it) => it.clone(),
                        None => continue,
                    };

                    let prod = &self.grammar.productions[item.key.prod_id];

                    if item.key.dot < prod.rhs.len() {
                        let next = &prod.rhs[item.key.dot];
                        match next {
                            Symbol::NonTerminal(nt) => {
                                for (pid, _) in self.grammar.prods_for(nt) {
                                    let new_it = Item::new(pid, 0, pos);
                                    if self.add_item(pos, new_it.clone()) {
                                        changed = true;
                                        self.add_nullable_items(new_it, pos, &nullable);
                                    }
                                }
                            }
                            Symbol::Terminal(lit) => {
                                if pos < self.tokens.len() && self.tokens[pos].text == *lit {
                                    let new_it = Item::new(
                                        item.key.prod_id,
                                        item.key.dot + 1,
                                        item.key.start,
                                    );
                                    if self.add_item(pos + 1, new_it) {
                                        changed = true;
                                    }
                                }
                            }
                            Symbol::Placeholder { name: _, typ } => {
                                if pos < self.tokens.len() && is_builtin(typ, &self.tokens[pos]) {
                                    let new_it = Item::new(
                                        item.key.prod_id,
                                        item.key.dot + 1,
                                        item.key.start,
                                    );
                                    if self.add_item(pos + 1, new_it) {
                                        changed = true;
                                    }
                                } else {
                                    for (pid, _) in self.grammar.prods_for(typ) {
                                        let new_it = Item::new(pid, 0, pos);
                                        if self.add_item(pos, new_it.clone()) {
                                            changed = true;
                                            self.add_nullable_items(new_it, pos, &nullable);
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Completion
                        let lhs = prod.lhs;
                        let waiting_keys: Vec<ItemKey> = self.sets[item.key.start]
                            .keys()
                            .filter(|k| {
                                let p = &self.grammar.productions[k.prod_id];
                                if k.dot < p.rhs.len() {
                                    match &p.rhs[k.dot] {
                                        Symbol::NonTerminal(name) => name == &lhs,
                                        Symbol::Placeholder { name: _, typ } => **typ == *lhs,
                                        _ => false,
                                    }
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect();

                        for wk in waiting_keys {
                            let new_it = Item::new(wk.prod_id, wk.dot + 1, wk.start);
                            if self.add_item(pos, new_it) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn accepted(&self, start: &str) -> bool {
        let n = self.tokens.len();
        self.sets[n].values().any(|it| {
            it.key.start == 0
                && it.key.dot == self.grammar.productions[it.key.prod_id].rhs.len()
                && self.grammar.productions[it.key.prod_id].lhs == start
        })
    }
}

impl<'gr, 'inp> Chart<'gr, 'inp> {
    #[allow(dead_code)]
    pub fn print_chart(&self) {
        // For each Earley set
        for (i, set) in self.sets.iter().enumerate() {
            println!("\n=== {} ===", i);

            // If the set is empty, skip
            if set.is_empty() {
                continue;
            }

            // Collect formatted lines first (for column alignment)
            let mut lines = Vec::new();
            let mut lhs_width = 0;

            for (key, _item) in set {
                let prod = &self.grammar.productions[key.prod_id];
                let lhs = prod.lhs;
                lhs_width = lhs_width.max(lhs.len());

                // Build RHS with dot
                let mut rhs = Vec::new();
                for (j, sym) in prod.rhs.iter().enumerate() {
                    if j == key.dot {
                        rhs.push("•".to_string());
                    }
                    rhs.push(format!("{}", sym));
                }
                if key.dot == prod.rhs.len() {
                    rhs.push("•".to_string());
                }
                let rhs_str = rhs.join(" ");

                let line = format!(
                    "{:<width$} -> {:<30} ({})",
                    lhs,
                    rhs_str,
                    key.start,
                    width = lhs_width
                );
                lines.push(line);
            }

            for l in lines {
                println!("{}", l);
            }
        }
    }
}

// ... (tests remain the same, they only use recognition, not parsing)

#[cfg(test)]
mod recognizer_tests {
    use super::*;

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(ValueSpec::FloatLiteral(21.1))
    }

    fn make_basic_expr_grammar<'gr>() -> Grammar<'gr> {
        Grammar {
            productions: vec![
                Production {
                    lhs: "Expr",
                    rhs: vec![
                        Symbol::NonTerminal("Term"),
                        Symbol::Terminal("+"),
                        Symbol::NonTerminal("Expr"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Expr",
                    rhs: vec![Symbol::NonTerminal("Term")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![Symbol::Placeholder {
                        name: "n",
                        typ: "Int",
                    }],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![Symbol::Placeholder {
                        name: "x",
                        typ: "Float",
                    }],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![Symbol::Placeholder {
                        name: "s",
                        typ: "String",
                    }],
                    out: dummy_outspec(),
                },
            ],
        }
    }

    #[test]
    fn recognize_simple_int_expr() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize("42");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }

    #[test]
    fn recognize_simple_float_expr() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize("3.14");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }

    #[test]
    fn recognize_simple_string_expr() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize(r#""hello""#);
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }

    #[test]
    fn recognize_addition_no_spaces() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize("42+3.14");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }

    #[test]
    fn reject_incomplete_addition() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize("42+");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(!chart.accepted("Expr"));
    }

    #[test]
    fn placeholder_bound_to_nonterminal() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![Symbol::NonTerminal("A")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "A",
                    rhs: vec![Symbol::Placeholder {
                        name: "x",
                        typ: "B",
                    }],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "B",
                    rhs: vec![Symbol::Terminal("x")],
                    out: dummy_outspec(),
                },
            ],
        };

        let toks = tokenize("x");
        let mut chart = Chart::new(&grammar, toks, "S");
        chart.recognize("S");
        chart.print_chart();
        assert!(chart.accepted("S"));
    }

    #[test]
    fn nested_nonterminal_recursion() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "Start",
                    rhs: vec![Symbol::NonTerminal("A")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "A",
                    rhs: vec![Symbol::Terminal("a"), Symbol::NonTerminal("B")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "B",
                    rhs: vec![Symbol::Terminal("b")],
                    out: dummy_outspec(),
                },
            ],
        };

        let toks = tokenize("ab");
        let mut chart = Chart::new(&grammar, toks, "Start");
        chart.recognize("Start");
        chart.print_chart();
        assert!(chart.accepted("Start"));
    }

    #[test]
    fn multiple_productions_same_lhs() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "X",
                    rhs: vec![Symbol::Terminal("x")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "X",
                    rhs: vec![Symbol::Terminal("y")],
                    out: dummy_outspec(),
                },
            ],
        };

        let toks_x = tokenize("x");
        let mut chart_x = Chart::new(&grammar, toks_x, "X");
        chart_x.recognize("X");
        chart_x.print_chart();
        assert!(chart_x.accepted("X"));

        let toks_y = tokenize("y");
        let mut chart_y = Chart::new(&grammar, toks_y, "X");
        chart_y.recognize("X");
        chart_y.print_chart();
        assert!(chart_y.accepted("X"));
    }
}

#[cfg(test)]
mod nullable_tests {
    use super::*;

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(ValueSpec::FloatLiteral(520.))
    }

    #[test]
    fn empty_rhs_nullable() {
        let grammar = Grammar {
            productions: vec![Production {
                lhs: "S",
                rhs: vec![],
                out: dummy_outspec(),
            }],
        };

        let tokens = tokenize("");
        let mut chart = Chart::new(&grammar, tokens, "S");
        chart.recognize("S");
        chart.print_chart();
        assert!(chart.accepted("S"));
    }

    #[test]
    fn nullable_nonterminal_in_sequence() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![Symbol::NonTerminal("A"), Symbol::NonTerminal("B")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "A",
                    rhs: vec![],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "B",
                    rhs: vec![Symbol::Terminal("x")],
                    out: dummy_outspec(),
                },
            ],
        };

        let tokens = tokenize("x");
        let mut chart = Chart::new(&grammar, tokens, "S");
        chart.recognize("S");
        chart.print_chart();
        assert!(chart.accepted("S"));
    }

    #[test]
    fn multiple_nullable_in_sequence() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![
                        Symbol::NonTerminal("A"),
                        Symbol::NonTerminal("B"),
                        Symbol::NonTerminal("C"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "A",
                    rhs: vec![],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "B",
                    rhs: vec![],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "C",
                    rhs: vec![Symbol::Terminal("y")],
                    out: dummy_outspec(),
                },
            ],
        };

        let tokens = tokenize("y");
        let mut chart = Chart::new(&grammar, tokens, "S");
        chart.recognize("S");
        chart.print_chart();
        assert!(chart.accepted("S"));
    }

    #[test]
    fn nullable_user_defined_placeholder() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![
                        Symbol::Placeholder {
                            name: "x",
                            typ: "X",
                        },
                        Symbol::Terminal("b"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "X",
                    rhs: vec![],
                    out: dummy_outspec(),
                },
            ],
        };

        let tokens = tokenize("b");
        let mut chart = Chart::new(&grammar, tokens, "S");
        chart.recognize("S");
        chart.print_chart();
        assert!(chart.accepted("S"));
    }

    #[test]
    fn nullable_mixed() {
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![
                        Symbol::Terminal("a"),
                        Symbol::NonTerminal("B"),
                        Symbol::Terminal("c"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "B",
                    rhs: vec![],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "B",
                    rhs: vec![Symbol::Terminal("b")],
                    out: dummy_outspec(),
                },
            ],
        };

        let tokens1 = tokenize("ac");
        let tokens2 = tokenize("abc");

        let mut chart1 = Chart::new(&grammar, tokens1, "S");
        chart1.recognize("S");
        chart1.print_chart();
        assert!(chart1.accepted("S"));

        let mut chart2 = Chart::new(&grammar, tokens2, "S");
        chart2.recognize("S");
        chart2.print_chart();
        assert!(chart2.accepted("S"));
    }
}

#[cfg(test)]
mod complex_expr_tests {
    use super::*;

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(ValueSpec::FloatLiteral(999.))
    }

    /// Grammar for a small arithmetic language:
    /// Expr    -> Expr '+' Term
    /// Expr    -> Expr '-' Term
    /// Expr    -> Term
    /// Term    -> Term '*' Factor
    /// Term    -> Term '/' Factor
    /// Term    -> Factor
    /// Factor  -> Number
    /// Factor  -> '(' Expr ')'
    /// Number  -> {n:Int}
    /// Number  -> {x:Float}
    fn make_expr_grammar<'gr>() -> Grammar<'gr> {
        Grammar {
            productions: vec![
                // Expr
                Production {
                    lhs: "Expr",
                    rhs: vec![
                        Symbol::NonTerminal("Expr"),
                        Symbol::Terminal("+"),
                        Symbol::NonTerminal("Term"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Expr",
                    rhs: vec![
                        Symbol::NonTerminal("Expr"),
                        Symbol::Terminal("-"),
                        Symbol::NonTerminal("Term"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Expr",
                    rhs: vec![Symbol::NonTerminal("Term")],
                    out: dummy_outspec(),
                },
                // Term
                Production {
                    lhs: "Term",
                    rhs: vec![
                        Symbol::NonTerminal("Term"),
                        Symbol::Terminal("*"),
                        Symbol::NonTerminal("Factor"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![
                        Symbol::NonTerminal("Term"),
                        Symbol::Terminal("/"),
                        Symbol::NonTerminal("Factor"),
                    ],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![Symbol::NonTerminal("Factor")],
                    out: dummy_outspec(),
                },
                // Factor
                Production {
                    lhs: "Factor",
                    rhs: vec![Symbol::NonTerminal("Number")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Factor",
                    rhs: vec![
                        Symbol::Terminal("("),
                        Symbol::NonTerminal("Expr"),
                        Symbol::Terminal(")"),
                    ],
                    out: dummy_outspec(),
                },
                // Number
                Production {
                    lhs: "Number",
                    rhs: vec![Symbol::Placeholder {
                        name: "n",
                        typ: "Int",
                    }],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Number",
                    rhs: vec![Symbol::Placeholder {
                        name: "x",
                        typ: "Float",
                    }],
                    out: dummy_outspec(),
                },
            ],
        }
    }

    #[test]
    fn recognize_nested_expression() {
        let grammar = make_expr_grammar();
        let toks = tokenize("(2+6)*4+2");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }

    #[test]
    fn recognize_expression_with_precedence() {
        let grammar = make_expr_grammar();
        let toks = tokenize("2+3*4-5");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }

    #[test]
    fn recognize_parenthesized_expression() {
        let grammar = make_expr_grammar();
        let toks = tokenize("(1+2)*(3+(4*5))");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();
        assert!(chart.accepted("Expr"));
    }
}
