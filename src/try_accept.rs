use crate::recognizer::{
    Chart, Grammar, Symbol, Token, tokenize,
};

/// A hint about where the parser was stuck.
#[derive(Debug, Clone)]
pub struct RuleHint<'gr> {
    pub lhs: &'gr str,
    pub remaining_rhs: Vec<Symbol<'gr>>,
    pub start_pos: usize,
}

/// Rich error type returned by `try_accept`.
#[derive(Debug, Clone)]
pub struct ParseError<'gr, 'inp> {
    pub pos: usize,                  // index in token stream
    pub found: Option<&'inp str>,    // offending token, if any
    pub hints: Vec<RuleHint<'gr>>,   // expected continuations
}

impl<'gr, 'inp> std::fmt::Display for ParseError<'gr, 'inp> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parse error at pos {}: found {:?}", self.pos, self.found)?;

        if !self.hints.is_empty() {
            writeln!(f, "\nYou could have meant:")?;
            for h in &self.hints {
                if let Some(sym) = h.remaining_rhs.first() {
                    writeln!(f, "  {} ?", sym)?;
                }
            }

            writeln!(f, "\nPossible continuations:")?;
            for h in &self.hints {
                let rhs_str: Vec<String> =
                    h.remaining_rhs.iter().map(|s| format!("{}", s)).collect();
                writeln!(f, "  {} -> {}", h.lhs, rhs_str.join(" "))?;
            }
        }

        Ok(())
    }
}

impl<'gr, 'inp> std::error::Error for ParseError<'gr, 'inp> {}

impl<'gr, 'inp> Chart<'gr, 'inp> {
    /// Attempt to parse with detailed error reporting.
    pub fn try_accept(&self, start: &str) -> Result<(), ParseError<'gr, 'inp>> {
        let n = self.tokens.len();

        // Check for full acceptance
        let accepted = self.sets[n].values().any(|it| {
            it.key.start == 0
                && it.key.dot == self.grammar.productions[it.key.prod_id].rhs.len()
                && self.grammar.productions[it.key.prod_id].lhs == start
        });
        if accepted {
            return Ok(());
        }

        // --- Not accepted: compute furthest progress ---
        let mut furthest_pos = 0;
        for (i, set) in self.sets.iter().enumerate() {
            if !set.is_empty() {
                furthest_pos = i;
            }
        }

        let found = if furthest_pos < self.tokens.len() {
            Some(self.tokens[furthest_pos].text)
        } else {
            None
        };

        // Collect continuation hints
        let mut hints = Vec::new();
        for item in self.sets[furthest_pos].values() {
            let prod = &self.grammar.productions[item.key.prod_id];
            if item.key.dot < prod.rhs.len() {
                let remaining_rhs = prod.rhs[item.key.dot..].to_vec();
                hints.push(RuleHint {
                    lhs: prod.lhs,
                    remaining_rhs,
                    start_pos: item.key.start,
                });
            }
        }

        Err(ParseError {
            pos: furthest_pos,
            found,
            hints,
        })
    }
}


#[cfg(test)]
mod try_accept_file_tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use crate::grammar_parser::{OutSpec, ValueSpec};
    use crate::recognizer::Production;

    fn dummy_outspec<'gr>() -> OutSpec<'gr> {
        OutSpec::Value(ValueSpec::FloatLiteral(0.))
    }

    fn make_expr_grammar<'gr>() -> Grammar<'gr> {
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
                    rhs: vec![Symbol::Placeholder { name: "n", typ: "Int" }],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![Symbol::Placeholder { name: "x", typ: "Float" }],
                    out: dummy_outspec(),
                },
                Production {
                    lhs: "Term",
                    rhs: vec![Symbol::Placeholder { name: "s", typ: "String" }],
                    out: dummy_outspec(),
                },
            ],
        }
    }

    fn write_parse_error<'gr, 'inp>(test_name: &str, err: &ParseError<'gr, 'inp>) {
        let dir = Path::new("./target/test_user_errors");
        fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{}.txt", test_name));
        let mut content = format!("Parse error at position {}\n", err.pos);
        if let Some(found) = err.found {
            content.push_str(&format!("Found token: '{}'\n", found));
        } else {
            content.push_str("Found token: <EOF>\n");
        }
        content.push_str("Expected hints:\n");
        for hint in &err.hints {
            let rhs: Vec<String> = hint.remaining_rhs.iter().map(|s| s.to_string()).collect();
            content.push_str(&format!("  {} -> {}\n", hint.lhs, rhs.join(" ")));
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn try_accept_incomplete_addition() {
        let grammar = make_expr_grammar();
        let input = "42+";
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Expr");
        chart.recognize("Expr");

        assert!(!chart.accepted("Expr"));

        if let Err(err) = chart.try_accept("Expr") {
            write_parse_error("try_accept_incomplete_addition", &err);
        }
    }

    #[test]
    fn try_accept_after_int() {
        let grammar = make_expr_grammar();
        let input = "42";
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Expr");
        chart.recognize("Expr");

        assert!(chart.accepted("Expr"));

        if let Err(err) = chart.try_accept("Expr") {
            write_parse_error("try_accept_after_int", &err);
        }
    }

    #[test]
    fn try_accept_string_literal() {
        let grammar = make_expr_grammar();
        let input = r#""abc""#;
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Expr");
        chart.recognize("Expr");

        assert!(chart.accepted("Expr"));

        if let Err(err) = chart.try_accept("Expr") {
            write_parse_error("try_accept_string_literal", &err);
        }
    }

    #[test]
    fn try_accept_nested_addition() {
        let grammar = make_expr_grammar();
        let input = "3+";
        let tokens = tokenize(input);
        let mut chart = Chart::new(&grammar, tokens, "Expr");
        chart.recognize("Expr");

        assert!(!chart.accepted("Expr"));

        if let Err(err) = chart.try_accept("Expr") {
            write_parse_error("try_accept_nested_addition", &err);
        }
    }
}

