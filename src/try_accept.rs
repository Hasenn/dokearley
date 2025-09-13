use thiserror::Error;

use crate::recognizer::{Chart};
use crate::recognizer::{Grammar,Symbol};
use std::collections::{HashMap, HashSet};

/// A parse error with both user-friendly and developer-friendly details
#[derive(Debug, Error)]
pub struct ParseError {
    pub pos: usize,
    pub found: Option<String>,
    pub expected: Vec<String>, // user-facing terminals
    pub items: Vec<String>,    // developer-facing Earley items
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(
            f,
            "Parse error at pos {}: around {:?}",
            self.pos,
            self.found.clone().unwrap_or("<EOF>".to_string())
        )?;

        if !self.expected.is_empty() {
            writeln!(f, "Expected one of: {}", self.expected.join(", "))?;
        }

        if !self.items.is_empty() {
            writeln!(f, "Related rules (dot at fail point):")?;
            for it in &self.items {
                writeln!(f, "  {}", it)?;
            }
        }

        Ok(())
    }
}

/// Formatting helper: show an item with a dot
fn format_item(lhs: &str, rhs: &[Symbol], dot: usize) -> String {
    let mut parts = Vec::new();
    for (i, sym) in rhs.iter().enumerate() {
        if i == dot {
            parts.push("•".to_string());
        }
        parts.push(format!("{}", sym));
    }
    if dot == rhs.len() {
        parts.push("•".to_string());
    }
    format!("{} -> {}", lhs, parts.join(""))
}

impl<'gr> Grammar<'gr> {
    /// Compute FIRST sets for all nonterminals and placeholders.
    pub fn compute_first_sets(&self) -> HashMap<&'gr str, HashSet<Symbol<'gr>>> {
        let mut first: HashMap<&'gr str, HashSet<Symbol<'gr>>> = HashMap::new();

        // Initialize nonterminals and placeholders with empty sets
        for prod in &self.productions {
            first.entry(prod.lhs).or_default();

            for sym in &prod.rhs {
                if let Symbol::Placeholder { typ, .. } = sym {
                    first.entry(typ).or_default();
                } else if let Symbol::NonTerminal(nt) = sym {
                    first.entry(nt).or_default();
                }
            }
        }

        let mut changed = true;
        while changed {
            changed = false;

            // Temporary map to accumulate updates
            let mut updates: HashMap<&'gr str, HashSet<Symbol<'gr>>> = HashMap::new();

            for prod in &self.productions {
                let lhs = prod.lhs;
                let mut new_syms = HashSet::new();

                if let Some(sym) = prod.rhs.first() {
                    match sym {
                        Symbol::Terminal(_) => {
                            new_syms.insert(sym.clone());
                        }
                        Symbol::NonTerminal(nt) => {
                            if let Some(rhs_first) = first.get(nt) {
                                new_syms.extend(rhs_first.iter().cloned());
                            }
                        }
                        Symbol::Placeholder { typ, .. } => {
                            if let Some(rhs_first) = first.get(typ) {
                                new_syms.extend(rhs_first.iter().cloned());
                            }
                        }
                    }
                }

                updates.entry(lhs).or_default().extend(new_syms);
            }

            // Merge updates into the main FIRST map
            for (lhs, syms) in updates {
                let lhs_set = first.get_mut(lhs).unwrap();
                let old_len = lhs_set.len();
                lhs_set.extend(syms);
                if lhs_set.len() > old_len {
                    changed = true;
                }
            }
        }

        first
    }
}

/// Expand a symbol into expected tokens (terminal names)
/// Expand a symbol into expected tokens (terminal names)
fn expected_tokens<'a>(
    sym: &Symbol<'a>,
    first_sets: &HashMap<&'a str, HashSet<Symbol<'a>>>,
) -> Vec<String> {
    match sym {
        Symbol::Terminal(s) => vec![s.to_string()],
        Symbol::NonTerminal(nt) => first_sets
            .get(nt)
            .map(|set| set.iter().map(|s| format!("{}", s)).collect())
            .unwrap_or_default(),
        Symbol::Placeholder { .. } => vec![], // placeholders don't expand to terminals
    }
}
impl<'gr, 'inp> Chart<'gr, 'inp> {
    pub fn try_accept(&self, start: &str) -> Result<(), ParseError> {
        if self.accepted(start) {
            return Ok(());
        }

        let first_sets = self.grammar.compute_first_sets();

        // 1️⃣ Find furthest index with some in-progress items (dot < rhs.len())
        let mut furthest_pos = 0;
        let mut expected = Vec::new();
        let mut items = Vec::new();

        for (i, set) in self.sets.iter().enumerate() {
            for item in set.values() {
                let prod = &self.grammar.productions[item.key.prod_id];
                if item.key.dot < prod.rhs.len() {
                    furthest_pos = i;
                }
            }
        }

        // 2️⃣ Offending token is the one *at* furthest_pos
        let found = self.tokens.get(furthest_pos).map(|t| t.text.to_string());

        // 3️⃣ Collect expectations/items from that point
        if let Some(set) = self.sets.get(furthest_pos) {
            for item in set.values() {
                let prod = &self.grammar.productions[item.key.prod_id];
                if item.key.dot < prod.rhs.len() {
                    let next_sym = &prod.rhs[item.key.dot];
                    expected.extend(expected_tokens(next_sym, &first_sets));
                    items.push(format_item(prod.lhs, &prod.rhs, item.key.dot));
                }
            }
        }

        expected.sort();
        expected.dedup();

        Err(ParseError {
            pos: furthest_pos,
            found,
            expected,
            items,
        })
    }
}

#[cfg(test)]
mod try_accept_file_tests {
    use super::*;
    use crate::grammar_parser::{OutSpec, ValueSpec};
    use crate::recognizer::{tokenize, Production};
    use std::fs;
    use std::path::Path;

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(ValueSpec::FloatLiteral(0.))
    }

    // --- helpers ---

    fn chars(s: &str) -> Vec<Symbol<'_>> {
        s.chars()
            .map(|c| Symbol::Terminal(Box::leak(c.to_string().into_boxed_str())))
            .collect()
    }

    /// Write input and ParseError Display directly
    fn write_parse_error<'inp>(test_name: &str, input: &str, err: &ParseError) {
        let dir = Path::new("./target/test_user_errors");
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{}.txt", test_name));

        let mut content = String::new();
        content.push_str("Input:\n");
        content.push_str(input);
        content.push_str("\n\nParseError:\n");
        content.push_str(&format!("{}", err));

        fs::write(path, content).unwrap();
    }

    // --- game-like grammar ---

    fn make_game_grammar<'gr>() -> Grammar<'gr> {
        Grammar {
            productions: vec![
                // Level ::= "level" String "{" Items "}"
                Production {
                    lhs: "Level",
                    rhs: {
                        let mut rhs = vec![];
                        rhs.extend(chars("level "));
                        rhs.push(Symbol::Placeholder {
                            name: "name",
                            typ: "String",
                        });
                        rhs.extend(chars(" "));
                        rhs.push(Symbol::Terminal("{"));
                        rhs.push(Symbol::NonTerminal("Items"));
                        rhs.push(Symbol::Terminal("}"));
                        rhs
                    },
                    out: dummy_outspec(),
                },
                // Items ::= Item Items | ε
                Production {
                    lhs: "Items",
                    rhs: vec![Symbol::NonTerminal("Item"), Symbol::NonTerminal("Items")],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Items",
                    rhs: vec![],
                    out: dummy_outspec(),
                },
                // Item ::= "enemy" String | "treasure" String
                Production {
                    lhs: "Item",
                    rhs: {
                        let mut rhs = chars("enemy");
                        rhs.push(Symbol::Placeholder {
                            name: "id",
                            typ: "String",
                        });
                        rhs
                    },
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Item",
                    rhs: {
                        let mut rhs = chars("treasure");
                        rhs.push(Symbol::Placeholder {
                            name: "id",
                            typ: "String",
                        });
                        rhs
                    },
                    out: dummy_outspec(),
                },
            ],
        }
    }

    // --- game-like tests with input written to files ---

    #[test]
    fn try_accept_incomplete_level() {
        let grammar = make_game_grammar();
        let input = r#"level "Dungeon" { enemy "orc" treasure"#; // missing string
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Level");
        chart.recognize("Level");

        assert!(!chart.accepted("Level"));

        if let Err(err) = chart.try_accept("Level") {
            write_parse_error("try_accept_incomplete_level", input, &err);
        }
    }

    #[test]
    fn try_accept_missing_brace() {
        let grammar = make_game_grammar();
        let input = r#"level "Dungeon"{ enemy "orc" treasure "gold""#; // missing }
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Level");
        chart.recognize("Level");

        assert!(!chart.accepted("Level"));

        if let Err(err) = chart.try_accept("Level") {
            write_parse_error("try_accept_missing_brace", input, &err);
        }
    }

    #[test]
    fn try_accept_wrong_level() {
        let grammar = make_game_grammar();
        let input = r#"level "Dungeon" { enemy "orc" tre asure "gold" }"#; // typo in 'treasure'
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Level");
        chart.recognize("Level");
        chart.print_chart();
        if let Err(err) = chart.try_accept("Level") {
            write_parse_error("try_accept_wrong_level", input, &err);
        }
    }
}
