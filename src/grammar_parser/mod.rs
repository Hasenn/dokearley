pub mod highlighter;
mod numbers;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_rejections;

use chumsky::{
    prelude::*,
    text::{inline_whitespace, newline},
};
use std::{collections::HashMap, hash::Hash};

use crate::parser::OutSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Str<'gr> {
    pub text: &'gr str,
    pub span: SimpleSpan,
}

impl<'gr> std::ops::Deref for Str<'gr> {
    type Target = &'gr str;
    fn deref(&self) -> &Self::Target {
        &self.text
    }
}

impl<'gr> AsRef<str> for Str<'gr> {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl<'gr> std::fmt::Display for Str<'gr> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}

impl<'gr> Str<'gr> {
    pub fn new(text: &'gr str, span: SimpleSpan) -> Self {
        Self { text, span }
    }
}

impl<'gr> PartialEq<str> for Str<'gr> {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl<'gr> PartialEq<&str> for Str<'gr> {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol<'gr> {
    Terminal(Str<'gr>),
    Placeholder { name: Str<'gr>, typ: Str<'gr> },
    NonTerminal(Str<'gr>),
}

#[derive(Debug, Clone)]
pub struct Production<'gr> {
    pub lhs: Str<'gr>,
    pub rhs: Vec<Symbol<'gr>>,
    pub out: OutSpec<'gr>,
}

impl<'gr> From<Option<RuleRhs<'gr>>> for OutSpec<'gr> {
    fn from(value: Option<RuleRhs<'gr>>) -> Self {
        match value {
            Some(value) => match value {
                RuleRhs::Type(typ) => OutSpec::Resource {
                    typ: *typ,
                    fields: HashMap::new(),
                },
                RuleRhs::TypeWithFields {
                    name: typ,
                    fields: rule_fields,
                } => {
                    let mut hash: HashMap<&'gr str, ValueSpec<'gr>> = HashMap::new();
                    rule_fields.iter().for_each(|(k, v)| {
                        hash.insert(&k, *v);
                    });
                    OutSpec::Resource {
                        typ: *typ,
                        fields: hash,
                    }
                }
                RuleRhs::Transparent => OutSpec::Transparent,
                RuleRhs::Dictionary(items) => {
                    let mut hash: HashMap<&'gr str, ValueSpec<'gr>> = HashMap::new();
                    items.iter().for_each(|(k, v)| {
                        hash.insert(&k, *v);
                    });
                    OutSpec::Dict(hash)
                }
            },
            None => Self::Dict(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Grammar<'gr> {
    pub productions: Vec<Production<'gr>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ValueSpec<'gr> {
    Identifier(Str<'gr>),
    StringLiteral(Str<'gr>),
    IntegerLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
}

#[derive(Debug, Clone)]
pub enum RuleRhs<'gr> {
    Type(Str<'gr>),
    TypeWithFields {
        name: Str<'gr>,
        fields: Vec<(Str<'gr>, ValueSpec<'gr>)>,
    },
    Dictionary(Vec<(Str<'gr>, ValueSpec<'gr>)>),
    Transparent,
}

#[derive(Debug, Clone)]
pub struct Rule<'gr> {
    pub lhs: Str<'gr>,
    pub pattern: Pattern<'gr>,
    pub rhs: Option<RuleRhs<'gr>>,
}
#[derive(Debug, Clone)]
pub enum Pattern<'gr> {
    Normal(Vec<Symbol<'gr>>),
    Disjunction(Vec<Symbol<'gr>>),
}

impl<'gr> From<&Vec<Rule<'gr>>> for Grammar<'gr> {
    fn from(value: &Vec<Rule<'gr>>) -> Self {
        let mut productions: Vec<Production<'gr>> = vec![];
        for rule in value {
            match &rule.pattern {
                Pattern::Normal(symbols) => productions.push(Production {
                    lhs: rule.lhs,
                    rhs: symbols.clone(),
                    out: OutSpec::from(rule.rhs.clone()),
                }),
                Pattern::Disjunction(symbols) => {
                    productions.extend(symbols.iter().map(|nt| Production {
                        lhs: rule.lhs,
                        rhs: vec![*nt],
                        out: OutSpec::Transparent,
                    }))
                }
            }
        }
        Self { productions }
    }
}

/// Chumsky Parser for a Vec of Rules, applying defaults for optional RHS (You can expect RHS to be Some)
pub fn rules<'gr>() -> impl Parser<'gr, &'gr str, Vec<Rule<'gr>>, extra::Err<Rich<'gr, char>>> {
    rules_raw().map_with(|r, _extra| {
        r.clone().iter_mut().for_each(|r| {
            if let None = r.rhs {
                r.rhs = Some(RuleRhs::Type(r.lhs.clone()))
            }
        });
        r
    })
}

pub fn rules_raw<'gr>() -> impl Parser<'gr, &'gr str, Vec<Rule<'gr>>, extra::Err<Rich<'gr, char>>> {
    choice((normal_rule(), transparent_rule()))
        .padded_by(inline_whitespace())
        .separated_by(
            just(';')
                .padded()
                .ignored()
                .or(newline().repeated().at_least(1)),
        )
        .allow_trailing()
        .allow_leading()
        .collect()
}

fn transparent_rule<'gr>() -> impl Parser<'gr, &'gr str, Rule<'gr>, extra::Err<Rich<'gr, char>>> {
    ident()
        .then_ignore(just(':').padded())
        .then(ident().separated_by(just('|').padded()).collect::<Vec<_>>())
        .padded_by(inline_whitespace())
        .map_with(|(lhs, pattern), _extra| Rule {
            lhs,
            pattern: Pattern::Disjunction(
                pattern.iter().map(|x| Symbol::NonTerminal(*x)).collect(),
            ),
            rhs: Some(RuleRhs::Transparent),
        })
        .labelled("rule")
}

fn normal_rule<'gr>() -> impl Parser<'gr, &'gr str, Rule<'gr>, extra::Err<Rich<'gr, char>>> {
    ident()
        .then_ignore(just(':').padded())
        .then(pattern_in_quotes().padded())
        .padded_by(inline_whitespace())
        .then(
            choice((just("=>"), just("->")))
                .padded()
                .ignore_then(out_spec_parser())
                .or_not(),
        )
        .map_with(|((lhs, pattern), opt_rhs), _extra| Rule {
            lhs,
            pattern: Pattern::Normal(pattern),
            rhs: opt_rhs,
        })
        .labelled("rule")
}

fn ident<'gr>() -> impl Parser<'gr, &'gr str, Str<'gr>, extra::Err<Rich<'gr, char>>> {
    text::ident().map_with(|s, extra| Str::new(s, extra.span()))
}

fn placeholder<'gr>() -> impl Parser<'gr, &'gr str, Symbol<'gr>, extra::Err<Rich<'gr, char>>> {
    just('{')
        .ignore_then(ident().padded())
        .then_ignore(just(':').padded())
        .then(ident().padded())
        .then_ignore(just('}'))
        .map(|(name, typ)| Symbol::Placeholder { name, typ })
        .labelled("placeholder")
}

fn terminal_text<'gr>() -> impl Parser<'gr, &'gr str, Symbol<'gr>, extra::Err<Rich<'gr, char>>> {
    any()
        .filter(|c: &char| *c != '{' && *c != '"')
        .repeated()
        .at_least(1)
        .to_slice()
        .map_with(|s, extra| Symbol::Terminal(Str::new(s, extra.span())))
        .labelled("terminal text")
}

fn pattern_in_quotes<'gr>(
) -> impl Parser<'gr, &'gr str, Vec<Symbol<'gr>>, extra::Err<Rich<'gr, char>>> {
    just('"')
        .ignore_then(
            choice((placeholder(), terminal_text()))
                .repeated()
                .collect(),
        )
        .then_ignore(just('"').padded())
        .labelled("pattern in quotes")
}

fn string_literal<'gr>() -> impl Parser<'gr, &'gr str, ValueSpec<'gr>, extra::Err<Rich<'gr, char>>>
{
    just('"')
        .ignore_then(any().filter(|c| *c != '"').repeated().to_slice())
        .then_ignore(just('"'))
        .map_with(|s, extra| ValueSpec::StringLiteral(Str::new(s, extra.span())))
        .labelled("string literal")
}

fn number_literal<'gr>() -> impl Parser<'gr, &'gr str, ValueSpec<'gr>, extra::Err<Rich<'gr, char>>>
{
    numbers::number_literal().labelled("number literal")
}

fn field_value<'gr>() -> impl Parser<'gr, &'gr str, ValueSpec<'gr>, extra::Err<Rich<'gr, char>>> {
    choice((
        string_literal(),
        number_literal(),
        ident().map(ValueSpec::Identifier),
    ))
}

fn fields_parser<'gr>(
) -> impl Parser<'gr, &'gr str, Vec<(Str<'gr>, ValueSpec<'gr>)>, extra::Err<Rich<'gr, char>>> {
    ident()
        .padded()
        .then_ignore(just(':').padded())
        .then(field_value())
        .separated_by(just(',').padded())
        .collect()
        .map_with(|fields, _span| fields)
        .labelled("fields")
}

fn res_out_spec<'gr>() -> impl Parser<'gr, &'gr str, RuleRhs<'gr>, extra::Err<Rich<'gr, char>>> {
    ident()
        .padded_by(inline_whitespace())
        .then(
            just('{')
                .padded()
                .ignore_then(fields_parser())
                .padded()
                .then_ignore(just('}'))
                .or_not(),
        )
        .map_with(|(name, opt_fields), _span| match opt_fields {
            Some(fields) => RuleRhs::TypeWithFields { name, fields },
            None => RuleRhs::Type(name),
        })
        .labelled("output specification")
}

fn dict_out_spec<'gr>() -> impl Parser<'gr, &'gr str, RuleRhs<'gr>, extra::Err<Rich<'gr, char>>> {
    just('{')
        .padded()
        .ignore_then(fields_parser())
        .padded()
        .then_ignore(just('}'))
        .map_with(|opt_fields, _span| match opt_fields {
            fields => RuleRhs::Dictionary(fields),
        })
        .labelled("output specification")
}

fn out_spec_parser<'gr>() -> impl Parser<'gr, &'gr str, RuleRhs<'gr>, extra::Err<Rich<'gr, char>>> {
    choice((dict_out_spec(), res_out_spec()))
}
