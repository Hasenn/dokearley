pub mod highlighter;
mod numbers;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_rejections;


use chumsky::{
    container::Container,
    prelude::*,
    text::{inline_whitespace, newline},
};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol<'gr> {
    Terminal(Str<'gr>),
    Placeholder { name: Str<'gr>, typ: Str<'gr> },
    NonTerminal(Str<'gr>),
}

#[derive(Debug, Clone)]
pub struct Production<'gr> {
    pub lhs: Str<'gr>,
    pub rhs: Vec<Symbol<'gr>>,
}

#[derive(Debug, Clone)]
pub struct Grammar<'gr> {
    pub productions: Vec<Production<'gr>>,
    pub start: Str<'gr>,
}


#[derive(Debug, Clone)]
pub enum FieldValue<'gr> {
    Identifier(Str<'gr>),
    StringLiteral(Str<'gr>),
    IntegerLiteral(i64),
    FloatLiteral(f64),
}

#[derive(Debug, Clone)]
pub enum RuleRhs<'gr> {
    Type(Str<'gr>),
    TypeWithFields {
        name: Str<'gr>,
        fields: Vec<(Str<'gr>, FieldValue<'gr>)>,
    },
}

#[derive(Debug, Clone)]
pub struct Rule<'gr> {
    pub lhs: Str<'gr>,
    pub pattern: Vec<Symbol<'gr>>,
    pub rhs: Option<RuleRhs<'gr>>,
}

pub fn grammar<'gr>() -> impl Parser<'gr, &'gr str, Vec<Rule<'gr>>, extra::Err<Rich<'gr, char>>> {
    rule()
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

fn rule<'gr>() -> impl Parser<'gr, &'gr str, Rule<'gr>, extra::Err<Rich<'gr, char>>> {
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
        .map_with(|((lhs, pattern), opt_rhs), extra| Rule {
            lhs,
            pattern,
            rhs: opt_rhs,
        })
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
}

fn terminal_text<'gr>() -> impl Parser<'gr, &'gr str, Symbol<'gr>, extra::Err<Rich<'gr, char>>> {
    any()
        .filter(|c: &char| *c != '{' && *c != '"')
        .repeated()
        .at_least(1)
        .to_slice()
        .map_with(|s, extra| Symbol::Terminal(Str::new(s, extra.span())))
}

fn pattern_in_quotes<'gr>()
-> impl Parser<'gr, &'gr str, Vec<Symbol<'gr>>, extra::Err<Rich<'gr, char>>> {
    just('"')
        .ignore_then(
            choice((placeholder(), terminal_text()))
                .repeated()
                .collect(),
        )
        .then_ignore(just('"').padded())
}

fn string_literal<'gr>() -> impl Parser<'gr, &'gr str, FieldValue<'gr>, extra::Err<Rich<'gr, char>>> {
    just('"')
        .ignore_then(any().filter(|c| *c != '"').repeated().to_slice())
        .then_ignore(just('"'))
        .map_with(|s, extra| FieldValue::StringLiteral(Str::new(s, extra.span())))
}

fn number_literal<'gr>() -> impl Parser<'gr, &'gr str, FieldValue<'gr>, extra::Err<Rich<'gr, char>>> {
    numbers::number_literal().map_with(|fv, extra| match fv {
        FieldValue::IntegerLiteral(i) => FieldValue::IntegerLiteral(i),
        FieldValue::FloatLiteral(f) => FieldValue::FloatLiteral(f),
        FieldValue::Identifier(s) => FieldValue::Identifier(s),
        FieldValue::StringLiteral(s) => FieldValue::StringLiteral(s),
    })
}

fn field_value<'gr>() -> impl Parser<'gr, &'gr str, FieldValue<'gr>, extra::Err<Rich<'gr, char>>> {
    choice((string_literal(), number_literal(), ident().map(FieldValue::Identifier)))
}

fn fields_parser<'gr>()
-> impl Parser<'gr, &'gr str, Vec<(Str<'gr>, FieldValue<'gr>)>, extra::Err<Rich<'gr, char>>> {
    ident()
        .padded()
        .then_ignore(just(':').padded())
        .then(field_value())
        .separated_by(just(',').padded())
        .collect()
        .map_with(|fields, _span| fields)
}

fn out_spec_parser<'gr>() -> impl Parser<'gr, &'gr str, RuleRhs<'gr>, extra::Err<Rich<'gr, char>>> {
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
}
