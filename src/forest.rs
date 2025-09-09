use std::collections::HashMap;
use std::rc::Rc;

use thiserror::Error;

use crate::recognizer::{Chart, ItemKey, Token};

/// A node in the parse forest
#[derive(Debug)]
pub enum ForestNode<'gr, 'inp> {
    /// Non-terminal node: stores the name and a list of derivations
    NonTerminal {
        name: &'gr str,
        /// Each derivation is a vector of child nodes (terminals or non-terminals)
        derivations: Vec<Vec<Rc<ForestNode<'gr, 'inp>>>>,
    },
    /// Terminal node: stores a reference to the matched token
    Terminal { token: &'inp Token<'inp> },
}

/// The parse forest itself: maps non-terminal names to their root nodes
#[derive(Debug)]
pub struct ParseForest<'gr, 'inp> {
    roots: HashMap<&'gr str, Vec<Rc<ForestNode<'gr, 'inp>>>>,
}

/// Errors that can occur while building a parse forest
#[derive(Debug, Error)]
pub enum ForestError {
    /// A required token was not found in the input
    #[error("Missing token at index {0}")]
    MissingToken(usize),

    /// A required item (production at a specific dot and start) was not found in the chart
    #[error(
        "Missing item in chart: prod_id={prod_id}, dot={dot}, start={start}"
    )]
    MissingItem {
        prod_id: usize,
        dot: usize,
        start: usize,
    },

    /// No completed items were found for a start symbol
    #[error("No completed items found for start production: {0:?}")]
    NoCompletedStartItem(ItemKey),
}

impl<'gr, 'inp> ParseForest<'gr, 'inp> {
    /// Build a parse forest from a recognized Earley chart
    pub fn from_chart(chart: &'inp Chart<'gr, 'inp>) -> Result<Self, ForestError> {
        let mut roots: HashMap<&'gr str, Vec<Rc<ForestNode<'gr, 'inp>>>> = HashMap::new();

        // Iterate through all productions of the start symbol
        let start_prods = chart
            .grammar
            .productions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.lhs == chart.start);

        for (prod_id, prod) in start_prods {
            let key = ItemKey {
                prod_id,
                dot: prod.rhs.len(), // fully completed
                start: 0,
            };

            let mut found_any = false;

            // Search through all chart sets for this completed item
            for set in &chart.sets {
                if set.contains_key(&key) {
                    let node = Self::build_node(chart, &key);
                    roots.entry(prod.lhs).or_default().push(node);
                    found_any = true;
                }
            }

            if !found_any {
                eprintln!(
                    "Warning: no completed items found for start production: {:?}",
                    key
                );
            }
        }

        Ok(Self { roots })
    }

    /// Recursive function to build a forest node from a completed ItemKey
    fn build_node(chart: &'inp Chart<'gr, 'inp>, key: &ItemKey) -> Rc<ForestNode<'gr, 'inp>> {
        // Special markers used in the chart:
        // usize::MAX -> terminal matched directly
        // usize::MAX - 1 -> placeholder matched directly
        if key.prod_id == usize::MAX || key.prod_id == usize::MAX - 1 {
            let token = &chart.tokens[key.start];
            return Rc::new(ForestNode::Terminal { token });
        }

        // Lookup the item in the chart
        let item = chart.sets[key.start]
            .get(key)
            .unwrap_or_else(|| panic!("ItemKey not found in chart (build_node): {:?}", key));

        let prod = &chart.grammar.productions[key.prod_id];

        let mut derivations = Vec::new();

        // Each backpointer sequence represents one possible derivation
        for bp_seq in &item.bps {
            let children: Vec<Rc<ForestNode<'gr, 'inp>>> = bp_seq
                .iter()
                .map(|bp| Self::build_node(chart, &bp.child))
                .collect();
            derivations.push(children);
        }

        Rc::new(ForestNode::NonTerminal {
            name: prod.lhs,
            derivations,
        })
    }

    /// Get the root nodes for a given non-terminal
    pub fn get_roots_for(&self, name: &'gr str) -> Vec<Rc<ForestNode<'gr, 'inp>>> {
        self.roots.get(name).cloned().unwrap_or_default()
    }

    /// Utility function: print the parse forest recursively
    pub fn print_forest(node: &Rc<ForestNode<'gr, 'inp>>, indent: usize) {
        let pad = "  ".repeat(indent);

        match node.as_ref() {
            ForestNode::Terminal { token } => {
                println!(
                    "{}Terminal('{}') [{}-{}]",
                    pad, token.text, token.span.start, token.span.end
                );
            }
            ForestNode::NonTerminal { name, derivations } => {
                println!("{}NonTerminal({})", pad, name);
                for (i, derivation) in derivations.iter().enumerate() {
                    println!("{}  Derivation {}:", pad, i);
                    for child in derivation {
                        Self::print_forest(child, indent + 2);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod forest_tests {
    use super::*;
    use crate::recognizer::{Chart, Grammar, OutSpec, Production, Symbol, Value, tokenize};

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(Value::FloatLiteral(0.0))
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
    fn parse_forest_simple_int() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize("42+32");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");
        chart.print_chart();

        let forest = ParseForest::from_chart(&chart);
        let roots = forest.get_roots_for("Expr");
        assert!(!roots.is_empty());

        for root in &roots {
            ParseForest::print_forest(root, 0);
        }
    }

    #[test]
    fn parse_forest_addition() {
        let grammar = make_basic_expr_grammar();
        let toks = tokenize("42+3.14");
        let mut chart = Chart::new(&grammar, toks, "Expr");
        chart.recognize("Expr");

        let forest = ParseForest::from_chart(&chart);
        let roots = forest.get_roots_for("Expr");
        assert!(!roots.is_empty());

        for root in &roots {
            ParseForest::print_forest(root, 0);
        }
    }
}
