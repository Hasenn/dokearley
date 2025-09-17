use crate::recognizer::{is_builtin, Chart, Grammar, Production, Symbol, Token, ValueSpec};
use std::{collections::HashMap, usize};

/// Represents a completed grammar rule (or terminal edge) in the chart.
/// `rule = usize::MAX` is a sentinel for a terminal/token edge.
#[derive(Debug, Clone)]
pub struct Edge {
    pub rule: usize,   // production id, usize::MAX = terminal edge
    pub finish: usize, // end position in the input
}

#[derive(Debug, Clone)]
pub enum OutSpec<'gr> {
    // A value corresponding to a basic type
    Value(ValueSpec<'gr>),
    // A resource with a type and optionally fixed fields
    Resource {
        typ: &'gr str,
        fields: HashMap<&'gr str, ValueSpec<'gr>>,
    },
    Dict(HashMap<&'gr str, ValueSpec<'gr>>),
    // Transparent rules that yield their single nonterminal's value (Disjunction)
    Transparent,
}

/// A parse tree node:
/// - `Token(Token<'inp>)` represents a leaf token in the input.
/// - `Node` represents a nonterminal with children, production OutSpec, and optional name.
#[derive(Debug, Clone)]
pub enum ParseTree<'gr, 'inp> {
    Token(Token<'inp>),
    Node {
        rule: Production<'gr>,
        children: Vec<ParseTree<'gr, 'inp>>,
    },
}
impl<'gr, 'inp> Chart<'gr, 'inp>
where
    'gr: 'inp,
{
    /// Build edges from completed items
    pub fn chart_of_items(&self) -> Vec<Vec<Edge>> {
        let n = self.sets.len();
        let mut chart: Vec<Vec<Edge>> = vec![Vec::new(); n];
        for (i, set) in self.sets.iter().enumerate() {
            for item in set.values() {
                let prod = &self.grammar.productions[item.key.prod_id];
                if item.key.dot == prod.rhs.len() {
                    chart[item.key.start].push(Edge {
                        rule: item.key.prod_id,
                        finish: i,
                    });
                }
            }
        }
        for edges in &mut chart {
            edges.sort_by(|a, b| a.rule.cmp(&b.rule).then(a.finish.cmp(&b.finish)));
        }
        chart
    }

    /// For a completed edge, produce the list of edges corresponding to RHS
    fn top_list<'a>(
        &self,
        chart: &'a [Vec<Edge>],
        tokens: &'a [Token<'inp>],
        start: usize,
        completed_edge: &Edge,
    ) -> Vec<(usize, Edge)> {
        let prod_id = completed_edge.rule;
        let prod = &self.grammar.productions[prod_id];
        let symbols = &prod.rhs;
        let bottom = symbols.len();
        let finish = completed_edge.finish;

        let pred = |depth: usize, cur_start: usize| depth == bottom && cur_start == finish;
        let child = |_depth: usize, edge: &Edge| edge.finish;
        let this = self;

        let edges_fn = move |depth: usize, cur_start: usize| -> Vec<Edge> {
            if depth >= bottom {
                return Vec::new();
            }
            match &symbols[depth] {
                Symbol::Terminal(lit) => {
                    if cur_start < tokens.len() && tokens[cur_start].text == *lit {
                        vec![Edge {
                            rule: usize::MAX,
                            finish: cur_start + 1,
                        }]
                    } else {
                        Vec::new()
                    }
                }
                Symbol::NonTerminal(name) => {
                    if cur_start < chart.len() {
                        chart[cur_start]
                            .iter()
                            .filter(|e| this.grammar.productions[e.rule].lhs == *name)
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    }
                }
                Symbol::Placeholder { name: _, typ } => {
                    // built in types act like non-terminals
                    if is_builtin(typ, &tokens[cur_start]) {
                        vec![Edge {
                            rule: usize::MAX,
                            finish: cur_start + 1,
                        }]
                    } else if cur_start < chart.len() {
                        chart[cur_start]
                            .iter()
                            .filter(|e| this.grammar.productions[e.rule].lhs == *typ)
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    }
                }
            }
        };

        fn dfs<FEdges, FChild, FPred>(
            depth: usize,
            start: usize,
            edges_fn: &FEdges,
            child_fn: &FChild,
            pred_fn: &FPred,
        ) -> Option<Vec<(usize, Edge)>>
        where
            FEdges: Fn(usize, usize) -> Vec<Edge>,
            FChild: Fn(usize, &Edge) -> usize,
            FPred: Fn(usize, usize) -> bool,
        {
            if pred_fn(depth, start) {
                return Some(Vec::new());
            }
            for edge in edges_fn(depth, start) {
                let next_start = child_fn(depth, &edge);
                if let Some(mut path) = dfs(depth + 1, next_start, edges_fn, child_fn, pred_fn) {
                    let mut res = Vec::with_capacity(1 + path.len());
                    res.push((start, edge));
                    res.append(&mut path);
                    return Some(res);
                }
            }
            None
        }

        dfs(0, start, &edges_fn, &child, &pred)
            .expect("recogniser invariants should guarantee a solution")
    }

    /// Build parse tree borrowing tokens
    pub fn build_parse_tree<'s>(&'s self) -> Option<ParseTree<'gr, 'inp>>
    where
        's: 'inp,
    {
        let chart = self.chart_of_items();
        let start_pos = 0;
        let finish_pos = chart.len() - 1;
        let start_symbol = self.start;

        let top_edge = chart[start_pos]
            .iter()
            .find(|e| {
                e.finish == finish_pos && self.grammar.productions[e.rule].lhs == start_symbol
            })?
            .clone();

        fn build<'gr, 'inp>(
            chart: &[Vec<Edge>],
            tokens: &'inp [Token<'inp>],
            grammar: &'gr Grammar<'gr>,
            start: usize,
            edge: Edge,
        ) -> ParseTree<'gr, 'inp> {
            if edge.rule == usize::MAX {
                return ParseTree::Token(tokens[start].clone());
            }

            let path = Chart {
                sets: Vec::new(),
                tokens: tokens.to_vec(),
                grammar,
                start: "",
            }
            .top_list(chart, tokens, start, &edge);

            let children = path
                .into_iter()
                .map(|(child_start, child_edge)| {
                    build(chart, tokens, grammar, child_start, child_edge)
                })
                .collect();

            //ParseTree::Node(grammar.productions[edge.rule].lhs.to_string(), children)
            ParseTree::Node {
                rule: grammar.productions[edge.rule].clone(),
                children,
            }
        }

        Some(build(
            &chart,
            &self.tokens,
            self.grammar,
            start_pos,
            top_edge,
        ))
    }
}

impl<'gr, 'inp> ParseTree<'gr, 'inp> {
    /// Pretty-print the parse tree with indentation
    #[allow(dead_code)]
    pub fn pretty_print(&self, indent: usize) {
        let padding = "  ".repeat(indent);
        match self {
            ParseTree::Token(tok) => {
                println!("{}Token({})", padding, tok.text);
            }
            ParseTree::Node { rule, children } => {
                println!("{}Node({:?})", padding, rule);
                for child in children {
                    child.pretty_print(indent + 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod parse_tree_pretty_tests {
    use crate::recognizer::{tokenize, Chart, Grammar, OutSpec, Production, Symbol, ValueSpec};

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(ValueSpec::FloatLiteral(0.0))
    }

    #[test]
    fn pretty_print_single_terminal() {
        // Grammar: S -> "a"
        let grammar = Grammar {
            productions: vec![Production {
                lhs: "S",
                rhs: vec![Symbol::Terminal("a")],
                out: dummy_outspec(),
            }],
        };
        let toks = tokenize("a");
        let mut chart = Chart::new(&grammar, toks, "S");
        chart.recognize("S");

        let tree = chart.build_parse_tree().expect("should build tree");
        println!("Pretty-print single terminal:");
        tree.pretty_print(0);
    }

    #[test]
    fn pretty_print_sequence() {
        // Grammar: S -> A B, A -> "a", B -> "b"
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![Symbol::NonTerminal("A"), Symbol::NonTerminal("B")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "A",
                    rhs: vec![Symbol::Terminal("a")],
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
        let mut chart = Chart::new(&grammar, toks, "S");
        chart.recognize("S");

        let tree = chart.build_parse_tree().expect("should build tree");
        println!("Pretty-print sequence:");
        tree.pretty_print(0);
    }

    #[test]
    fn pretty_print_placeholder() {
        // Grammar: S -> X, X -> {n:Int} (matches a number like "42")
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![Symbol::NonTerminal("X")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "X",
                    rhs: vec![Symbol::Placeholder {
                        name: "n",
                        typ: "Int",
                    }],
                    out: dummy_outspec(),
                },
            ],
        };
        let toks = tokenize("42");
        let mut chart = Chart::new(&grammar, toks, "S");
        chart.recognize("S");

        let tree = chart.build_parse_tree().expect("should build tree");
        println!("Pretty-print placeholder:");
        tree.pretty_print(0);
    }

    #[test]
    fn pretty_print_nested() {
        // Grammar: S -> A A, A -> "a"
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "S",
                    rhs: vec![Symbol::NonTerminal("A"), Symbol::NonTerminal("A")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "A",
                    rhs: vec![Symbol::Terminal("a")],
                    out: dummy_outspec(),
                },
            ],
        };
        let toks = tokenize("aa");
        let mut chart = Chart::new(&grammar, toks, "S");
        chart.recognize("S");

        let tree = chart.build_parse_tree().expect("should build tree");
        println!("Pretty-print nested nonterminals:");
        tree.pretty_print(0);
    }
}

#[derive(Debug, Clone)]
pub enum Value<'gr, 'inp> {
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(&'inp str),
    Resource {
        typ: &'gr str,
        fields: HashMap<&'gr str, Value<'gr, 'inp>>,
    },
    Dictionary(HashMap<&'gr str, Value<'gr, 'inp>>),
    /// A value that will come from the first child matching the given non-terminal.
    Child(&'gr str),
    /// A value that will collect all children matching the given non-terminal into a vec.
    Children(&'gr str),
}

impl<'gr, 'inp> ParseTree<'gr, 'inp>
where
    'gr: 'inp,
{
    pub fn compute_value(&self) -> Value<'gr, 'inp> {
        match self {
            // Tokens can yield a value if needed, but this would not be used currently.
            ParseTree::Token(tok) => tok.get_value().unwrap_or(Value::String(tok.text)),
            // For nodes, we check the OutSpec and do what it says
            ParseTree::Node { rule, children } => match &rule.out {
                OutSpec::Value(spec) => match spec {
                    ValueSpec::IntegerLiteral(i) => Value::Integer(*i),
                    ValueSpec::FloatLiteral(f) => Value::Float(*f),
                    ValueSpec::StringLiteral(s) => Value::String(s),
                    ValueSpec::BoolLiteral(b) => Value::Bool(*b),
                    ValueSpec::Identifier(name) => {
                                        // find first child matching placeholder name
                                        children
                                            .iter()
                                            .find_map(|c| match c {
                                                ParseTree::Node {
                                                    rule: child_rule, ..
                                                } => child_rule.rhs.iter().zip(c.as_children()).find_map(
                                                    |(sym, child)| match sym {
                                                        Symbol::Placeholder { name: n, .. } if *n == **name => {
                                                            Some(child.compute_value())
                                                        }
                                                        _ => None,
                                                    },
                                                ),
                                                ParseTree::Token(_tok) => None,
                                            })
                                            .unwrap_or(Value::String("<missing_placeholder>"))
                                    }
                    ValueSpec::Child(c) => Value::Child(c),
                    ValueSpec::Children(c) => Value::Children(c),
                },
                // If the outspec says to build a resource, make it
                OutSpec::Resource { typ, fields } => {
                    let mut result_fields = HashMap::new();

                    // Collect children placeholders
                    for (i, sym) in rule.rhs.iter().enumerate() {
                        match sym {
                            Symbol::Placeholder { name, .. } => {
                                let val = children[i].compute_value();
                                result_fields.insert(*name, val);
                            }
                            Symbol::NonTerminal(nt_name) => {
                                let child_val = children[i].compute_value();
                                // if child is a __Propagate__ resource, merge fields
                                match &child_val {
                                    Value::Resource { typ: t, fields: f }
                                        if *t == "__Propagate__" =>
                                    {
                                        for (k, v) in f {
                                            result_fields.insert(k, v.clone());
                                        }
                                    }
                                    _ => {
                                        // otherwise, keep under nonterminal name
                                        result_fields.insert(*nt_name, child_val);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    // fixed aliases
                    for (k, v) in fields {
                        let val = match v {
                            ValueSpec::Identifier(n) => children
                                                        .iter()
                                                        .find_map(|c| c.find_placeholder(n))
                                                        .unwrap_or(Value::String("<missing_i>")),
                            ValueSpec::IntegerLiteral(i) => Value::Integer(*i),
                            ValueSpec::FloatLiteral(f) => Value::Float(*f),
                            ValueSpec::StringLiteral(s) => Value::String(s),
                            ValueSpec::BoolLiteral(b) => Value::Bool(*b),
                            ValueSpec::Child(c) => Value::Child(c),
                            ValueSpec::Children(c) => Value::Children(c),

                        };
                        result_fields.insert(*k, val);
                    }

                    Value::Resource {
                        typ,
                        fields: result_fields,
                    }
                }
                OutSpec::Transparent => children[0].compute_value(),
                // If the outspec says to build a dictionary, make it
                OutSpec::Dict(fields) => {
                    let mut result_fields = HashMap::new();

                    // collect children placeholders and non-terminals
                    for (i, sym) in rule.rhs.iter().enumerate() {
                        match sym {
                            Symbol::Placeholder { name, .. } => {
                                let val = children[i].compute_value();
                                result_fields.insert(*name, val);
                            }
                            Symbol::NonTerminal(nt_name) => {
                                let child_val = children[i].compute_value();
                                result_fields.insert(*nt_name, child_val);
                            }
                            _ => {}
                        }
                    }

                    // fixed fields (aliases) from OutSpec::Dict definition
                    for (k, v) in fields {
                        let val = match v {
                            ValueSpec::Identifier(name) => {
                                                                                self.find_placeholder(name).unwrap_or(Value::String("<missing related placeholder>"))
                                                                            },
                            ValueSpec::IntegerLiteral(i) => Value::Integer(*i),
                            ValueSpec::FloatLiteral(f) => Value::Float(*f),
                            ValueSpec::StringLiteral(s) => Value::String(s),
                            ValueSpec::BoolLiteral(b) => Value::Bool(*b),
                            ValueSpec::Child(c) => Value::Child(c),
                            ValueSpec::Children(c) => Value::Children(c),
                        };
                        result_fields.insert(*k, val);
                    }

                    Value::Dictionary(result_fields)
                }
            },
        }
    }

    fn as_children(&self) -> Vec<ParseTree<'gr, 'inp>> {
        match self {
            ParseTree::Node { rule: _, children } => children.clone(),
            _ => vec![],
        }
    }

    fn find_placeholder(&self, name: &str) -> Option<Value<'gr, 'inp>> {
        match self {
            ParseTree::Node { rule, children } => {
                for (sym, child) in rule.rhs.iter().zip(children) {
                    if let Symbol::Placeholder { name: n, .. } = sym {
                        if **n == *name {
                            return Some(child.compute_value());
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }
}
#[cfg(test)]
mod parse_tree_value_tests {
    use super::*;
    use crate::{recognizer::tokenize};

    #[test]
    fn compute_value_simple_effect() {
        // Effect : "Deal {damage:Int} damage to {Target}" -> DamageEffect
        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "Effect",
                    rhs: vec![
                        Symbol::Terminal("D"),
                        Symbol::Terminal("e"),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("l"),
                        Symbol::Terminal(" "),
                        Symbol::Placeholder {
                            name: "damage",
                            typ: "Int",
                        },
                        Symbol::Terminal(" "),
                        Symbol::Terminal("d"),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("m"),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("g"),
                        Symbol::Terminal("e"),
                        Symbol::Terminal(" "),
                        Symbol::Terminal("t"),
                        Symbol::Terminal("o"),
                        Symbol::Terminal(" "),
                        Symbol::NonTerminal("Target"),
                    ],
                    out: OutSpec::Resource {
                        typ: "DamageEffect",
                        fields: HashMap::new(), // implicit fields come from placeholders + children
                    },
                },
                Production {
                    lhs: "Target",
                    rhs: vec![
                        Symbol::Terminal("e"),
                        Symbol::Terminal("n"),
                        Symbol::Terminal("e"),
                        Symbol::Terminal("m"),
                        Symbol::Terminal("i"),
                        Symbol::Terminal("e"),
                        Symbol::Terminal("s"),
                    ],
                    out: OutSpec::Value(ValueSpec::IntegerLiteral(1)),
                },
            ],
        };

        let toks = tokenize("Deal 32 damage to enemies");
        let mut chart = Chart::new(&grammar, toks, "Effect");
        chart.recognize("Effect");

        let tree = chart.build_parse_tree().expect("tree should build");
        tree.pretty_print(0);

        let val = tree.compute_value();
        println!("Computed value: {:?}", val);

        match val {
            Value::Resource { typ, fields } => {
                assert_eq!(typ, "DamageEffect");
                assert!(matches!(fields["damage"], Value::Integer(32)));
                assert!(matches!(fields["Target"], Value::Integer(1)));
            }
            _ => panic!("expected Resource"),
        }
    }

    #[test]
    fn compute_value_with_dict() {
        // Effect : "Deal {damage:Int} damage at {Position}" -> DamageEffect
        // Position : "(" {x:Int} "," {y:Int} ")" -> { x: {x}, y: {y} }

        let grammar = Grammar {
            productions: vec![
                Production {
                    lhs: "Effect",
                    rhs: vec![
                        Symbol::Terminal("D"),
                        Symbol::Terminal("e"),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("l"),
                        Symbol::Terminal(" "),
                        Symbol::Placeholder {
                            name: "damage",
                            typ: "Int",
                        },
                        Symbol::Terminal(" "),
                        Symbol::Terminal("d"),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("m"),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("g"),
                        Symbol::Terminal("e"),
                        Symbol::Terminal(" "),
                        Symbol::Terminal("a"),
                        Symbol::Terminal("t"),
                        Symbol::Terminal(" "),
                        Symbol::NonTerminal("Position"),
                    ],
                    out: OutSpec::Resource {
                        typ: "DamageEffect",
                        fields: HashMap::new(),
                    },
                },
                Production {
                    lhs: "Position",
                    rhs: vec![
                        Symbol::Terminal("("),
                        Symbol::Placeholder {
                            name: "x",
                            typ: "Int",
                        },
                        Symbol::Terminal(","),
                        Symbol::Placeholder {
                            name: "y",
                            typ: "Int",
                        },
                        Symbol::Terminal(")"),
                    ],
                    out: OutSpec::Dict(HashMap::new()),
                },
            ],
        };

        let toks = tokenize("Deal 32 damage at (2,5)");
        let mut chart = Chart::new(&grammar, toks, "Effect");
        chart.recognize("Effect");

        let tree = chart.build_parse_tree().expect("tree should build");
        tree.pretty_print(0);

        let val = tree.compute_value();
        println!("Computed value: {:?}", val);

        match val {
            Value::Resource { typ, fields } => {
                assert_eq!(typ, "DamageEffect");
                assert!(matches!(fields["damage"], Value::Integer(32)));
                assert!(matches!(fields["Position"], Value::Dictionary(_)));

                if let Value::Dictionary(dict) = &fields["Position"] {
                    assert!(matches!(dict["x"], Value::Integer(2)));
                    assert!(matches!(dict["y"], Value::Integer(5)));
                } else {
                    panic!("Position should be a Dictionary");
                }
            }
            _ => panic!("expected Resource"),
        }
    }
}
