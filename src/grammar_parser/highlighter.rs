use std::ops::Range;

use crate::grammar_parser::{Value, Rule, RuleRhs, Str, Symbol};

/// What kind of token this is for highlighting
#[derive(Debug, Clone, Copy)]
pub enum HighlightKind {
    LHS,
    Terminal,
    PlaceholderName,
    PlaceholderType,
    NonTerminal,
    RHS,
    FieldName,
    StringLiteral,
    IntegerLiteral,
    FloatLiteral,
    Identifier,
}

/// A token with a span in the original input
#[derive(Debug, Clone)]
pub struct HighlightToken<'a> {
    pub text: &'a str,
    pub span: Range<usize>,
    pub kind: HighlightKind,
}

fn span_token<'a>(s: &Str<'a>, kind: HighlightKind) -> HighlightToken<'a> {
    HighlightToken {
        text: s.text,
        span: s.span.start..s.span.end,
        kind,
    }
}

/// Produce highlight tokens for the entire input & rules
pub fn highlight_tokens<'a>(input: &'a str, rules: &[Rule<'a>]) -> Vec<HighlightToken<'a>> {
    let mut tokens = Vec::new();

    for rule in rules {
        // LHS
        tokens.push(span_token(&rule.lhs, HighlightKind::LHS));

        // Pattern symbols
        for sym in &rule.pattern {
            match sym {
                Symbol::Terminal(t) => {
                    tokens.push(span_token(t, HighlightKind::Terminal));
                }
                Symbol::Placeholder { name, typ } => {
                    // {name:Type}
                    tokens.push(span_token(name, HighlightKind::PlaceholderName));
                    tokens.push(span_token(typ, HighlightKind::PlaceholderType));
                }
                Symbol::NonTerminal(nt) => {
                    tokens.push(span_token(nt, HighlightKind::NonTerminal));
                }
            }
        }

        // RHS
        if let Some(rhs) = &rule.rhs {
            match rhs {
                RuleRhs::Type(name) => {
                    tokens.push(span_token(name, HighlightKind::RHS));
                }
                RuleRhs::TypeWithFields { name, fields } => {
                    tokens.push(span_token(name, HighlightKind::RHS));
                    for (field_name, field_val) in fields {
                        tokens.push(span_token(field_name, HighlightKind::FieldName));
                        match field_val {
                            Value::Identifier(s) => {
                                tokens.push(span_token(s, HighlightKind::Identifier));
                            }
                            Value::StringLiteral(s) => {
                                // Emit quotes + content
                                let span = s.span.clone();
                                tokens.push(HighlightToken {
                                    text: "\"",
                                    span: (span.start - 1)..span.start,
                                    kind: HighlightKind::StringLiteral,
                                });
                                tokens.push(span_token(s, HighlightKind::StringLiteral));
                                tokens.push(HighlightToken {
                                    text: "\"",
                                    span: span.end..(span.end + 1),
                                    kind: HighlightKind::StringLiteral,
                                });
                            }
                            Value::IntegerLiteral(_) => {
                                // spans not yet carried — TODO
                            }
                            Value::FloatLiteral(_) => {
                                // spans not yet carried — TODO
                            }
                        }
                    }
                }
            }
        }
    }

    tokens
}
