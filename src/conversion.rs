use crate::{
    grammar_parser::{self, Rule},
    recognizer::{self},
};

// In recognizer or a conversion module
impl<'gr> From<&grammar_parser::Str<'gr>> for &'gr str {
    fn from(s: &grammar_parser::Str<'gr>) -> Self {
        s.text
    }
}

impl<'gr> From<grammar_parser::Symbol<'gr>> for Vec<recognizer::Symbol<'gr>> {
    fn from(sym: grammar_parser::Symbol<'gr>) -> Self {
        use grammar_parser::Symbol::*;
        match sym {
            Terminal(s) => {
                let text = s.text;
                text.char_indices()
                    .map(|(i, ch)| {
                        let end = i + ch.len_utf8();
                        recognizer::Symbol::Terminal(&text[i..end])
                    })
                    .collect()
            }
            Placeholder { name, typ } => vec![recognizer::Symbol::Placeholder {
                name: name.text,
                typ: typ.text,
            }],
            NonTerminal(s) => vec![recognizer::Symbol::NonTerminal(s.text)],
        }
    }
}

impl<'gr> From<grammar_parser::Production<'gr>> for recognizer::Production<'gr> {
    fn from(prod: grammar_parser::Production<'gr>) -> Self {
        recognizer::Production {
            lhs: prod.lhs.text,
            rhs: prod
                .rhs
                .into_iter()
                .flat_map(Into::<Vec<recognizer::Symbol>>::into)
                .collect(),
            out: prod.out,
        }
    }
}

impl<'gr> From<grammar_parser::Grammar<'gr>> for recognizer::Grammar<'gr> {
    fn from(g: grammar_parser::Grammar<'gr>) -> Self {
        recognizer::Grammar {
            productions: g.productions.into_iter().map(Into::into).collect(),
        }
    }
}

impl<'gr> From<&Vec<Rule<'gr>>> for recognizer::Grammar<'gr> {
    fn from(rules: &Vec<Rule<'gr>>) -> Self {
        Into::<grammar_parser::Grammar>::into(rules).into()
    }
}
