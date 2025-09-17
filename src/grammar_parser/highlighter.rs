use std::ops::Range;

use crate::grammar_parser::{Pattern, Rule, RuleRhs, Str, Symbol, ValueSpec};

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
    ChildName
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
pub fn highlight_tokens<'a>(_input: &'a str, rules: &[Rule<'a>]) -> Vec<HighlightToken<'a>> {
    let mut tokens = Vec::new();

    for rule in rules {
        // LHS
        tokens.push(span_token(&rule.lhs, HighlightKind::LHS));

        // Pattern symbols — handle Pattern::Normal and Pattern::Disjunction
        match &rule.pattern {
            Pattern::Normal(symbols) => {
                for sym in symbols {
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
            }
            Pattern::Disjunction(symbols) => {
                // disjunction is a list of single NonTerminals (as you build them)
                // highlight each nonterminal
                for sym in symbols {
                    match sym {
                        Symbol::NonTerminal(nt) => {
                            tokens.push(span_token(nt, HighlightKind::NonTerminal));
                        }
                        // In case you later allow other kinds in disjunction, handle them too:
                        Symbol::Terminal(t) => {
                            tokens.push(span_token(t, HighlightKind::Terminal));
                        }
                        Symbol::Placeholder { name, typ } => {
                            tokens.push(span_token(name, HighlightKind::PlaceholderName));
                            tokens.push(span_token(typ, HighlightKind::PlaceholderType));
                        }
                    }
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
                            ValueSpec::Identifier(s) => {
                                                        tokens.push(span_token(s, HighlightKind::Identifier));
                                                    }
                            ValueSpec::StringLiteral(s) => {
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
                            ValueSpec::IntegerLiteral(_) => {
                                                        // spans not yet carried — TODO
                                                    }
                            ValueSpec::FloatLiteral(_) => {
                                                        // spans not yet carried — TODO
                                                    }
                            ValueSpec::BoolLiteral(_) => {
                                                        // no spans for bool yet
                                                    }
                            ValueSpec::Child(s) => {
                                tokens.push(span_token(s, HighlightKind::ChildName))
                            },
                            ValueSpec::Children(s) => {
                                tokens.push(span_token(s, HighlightKind::ChildName))
                            },
                        }
                    }
                }
                RuleRhs::Transparent => {
                    // Transparent has no explicit RHS text to highlight.
                    // We already highlighted the pattern (which for transparent rules
                    // is a single nonterminal), so nothing more to do here.
                }
                RuleRhs::Dictionary(fields) => {
                    for (field_name, field_val) in fields {
                        tokens.push(span_token(field_name, HighlightKind::FieldName));
                        match field_val {
                            ValueSpec::Identifier(s) => {
                                                        tokens.push(span_token(s, HighlightKind::Identifier));
                                                    }
                            ValueSpec::StringLiteral(s) => {
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
                            ValueSpec::IntegerLiteral(_) => {
                                                        // spans not yet carried — TODO
                                                    }
                            ValueSpec::FloatLiteral(_) => {
                                                        // spans not yet carried — TODO
                                                    }
                            ValueSpec::BoolLiteral(_) => {
                                                        // no spans for bool yet
                                                    }
                            ValueSpec::Child(s) => {
                                tokens.push(span_token(s, HighlightKind::ChildName))
                            },
                            ValueSpec::Children(s) => {
                                tokens.push(span_token(s, HighlightKind::ChildName))
                            },
                        }
                    }
                }
            }
        }
    }

    tokens
}
