#[cfg(test)]
mod integration_tests {
    use chumsky::Parser;
    use dokearley::recognizer::Chart;
    use dokearley::{grammar_parser, recognizer};

    fn parse_and_recognize(grammar_text: &str, start: &str, input: &str) -> bool {
        let result = grammar_parser::rules().parse(grammar_text);
        if result.has_errors() {
            for e in result.errors() {
                println!("Error: {} at {}", e, e.span());
            }
        }
        dbg!(&result);
        if let Some(parsed_rules) = result.output() {
            // Convert parser grammar into recognizer grammar
            let grammar: recognizer::Grammar = parsed_rules.into();
            let tokens = recognizer::tokenize(input);
            let mut chart = Chart::new(&grammar, tokens, start);
            chart.recognize(start);
            chart.accepted(start)
        } else {
            false
        }
    }

    #[test]
    fn effect_simple_int() {
        let grammar_text = r#"
Effect : "Deal {dmg : int}" -> DamageEffect
"#;
        assert!(parse_and_recognize(grammar_text, "Effect", "Deal 42"));
    }

    #[test]
    fn effect_then_effect_sequence() {
        // Include the base rule for recursion
        let grammar_text = r#"
Effect : "Deal {dmg : int}" -> DamageEffect
Effect : "{first : Effect}, then {then : Effect}" -> EffectThenEffect
"#;
        assert!(parse_and_recognize(
            grammar_text,
            "Effect",
            "Deal 10, then Deal 20"
        ));
    }

    #[test]
    fn nested_effect_sequence() {
        let grammar_text = r#"
Effect : "Deal {dmg : int}" -> DamageEffect
Effect : "{first : Effect}, then {then : Effect}" -> EffectThenEffect
Effect : "{first : Effect}, then {second : Effect}, then {third : Effect}" -> TripleEffect
"#;
        assert!(parse_and_recognize(
            grammar_text,
            "Effect",
            "Deal 1, then Deal 2, then Deal 3"
        ));
    }

    #[test]
    fn multiple_placeholders() {
        let grammar_text = r#"
Command : "Move {x : Int} steps to {y : Int}" -> MoveCommand
Command : "Move {x : Int} steps" -> MoveXCommand
"#;
        assert!(parse_and_recognize(
            grammar_text,
            "Command",
            "Move 10 steps to 20"
        ));
        assert!(parse_and_recognize(grammar_text, "Command", "Move 5 steps"));
    }
}
