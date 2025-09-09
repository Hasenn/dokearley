use chumsky::Parser;
use dokearley::grammar_parser;
use grammar_parser::grammar;
use grammar_parser::highlighter::{highlight_tokens, HighlightKind};
use colored::*;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read input");

    let result = grammar().parse(&input);

    if result.has_errors() {
        let errors: Vec<_> = result.errors().collect();
        for e in errors {
            println!("Error: {} at {}", e, e.span());
        }
        println!("--- continuing to highlight valid parts ---");
    }

    let rules = match result.output() {
        Some(r) => r,
        None => {
            println!("No rules parsed.");
            return;
        }
    };

    // Get highlight tokens
    let mut tokens = highlight_tokens(&input, &rules);

    // Sort tokens by start position
    tokens.sort_by_key(|t| t.span.start);

    let mut cursor = 0;
    for tok in &tokens {
        // Print any text before this token
        if tok.span.start > cursor {
            print!("{}", &input[cursor..tok.span.start].dimmed());
        }

        let colored_text = match tok.kind {
            HighlightKind::LHS => tok.text.blue().bold(),
            HighlightKind::Terminal => tok.text.white(),
            HighlightKind::PlaceholderName => tok.text.cyan().bold(),
            HighlightKind::PlaceholderType => tok.text.bright_green(),
            HighlightKind::NonTerminal => tok.text.cyan(),
            HighlightKind::RHS => tok.text.bright_green().bold(),
            HighlightKind::FieldName => tok.text.cyan().bold(),
            HighlightKind::StringLiteral => tok.text.yellow(),
            HighlightKind::IntegerLiteral => tok.text.cyan().dimmed(),
            HighlightKind::FloatLiteral => tok.text.cyan().dimmed(),
            HighlightKind::Identifier => tok.text.white(),
        };

        print!("{}", colored_text);
        cursor = tok.span.end;
    }

    // Print remaining text
    if cursor < input.len() {
        print!("{}", &input[cursor..]);
    }

    println!();
}
