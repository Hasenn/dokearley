use chumsky::Parser;

use crate::grammar_parser::rules;

#[cfg(test)]
mod invalid_input_tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;

    fn log_errors(
        test_name: &str,
        input: &str,
        errors: impl IntoIterator<Item = impl std::fmt::Display>,
    ) {
        let folder = Path::new("target/test_errors");
        if !folder.exists() {
            fs::create_dir_all(folder).unwrap();
        }
        let file_path = folder.join(format!("{}.log", test_name));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Input:\n{}\n", input).unwrap();
        writeln!(file, "Errors:").unwrap();
        for e in errors {
            writeln!(file, "  - {}", e).unwrap();
        }
        println!("Parse errors logged to {:?}", file_path);
    }

    #[test]
    fn test_unclosed_quote() {
        let input = r#"Rule : "unclosed => Type"#;
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail on unclosed quote"
        );
        log_errors("unclosed_quote", input, result.errors());
    }

    #[test]
    fn test_missing_colon() {
        let input = r#"Rule "pattern" => Type"#;
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail when ':' is missing"
        );
        log_errors("missing_colon", input, result.errors());
    }

    #[test]
    fn test_missing_arrow() {
        let input = r#"Rule : "pattern" Type"#;
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail when '=>' is missing"
        );
        log_errors("missing_arrow", input, result.errors());
    }

    #[test]
    fn test_invalid_field_syntax() {
        let input = r#"Rule : "pattern" => Type{field}"#;
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail on invalid field syntax"
        );
        log_errors("invalid_field_syntax", input, result.errors());
    }

    #[test]
    fn test_empty_placeholder_name() {
        let input = r#"Rule : "{}:Int" => Type"#;
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail on empty placeholder name"
        );
        log_errors("empty_placeholder_name", input, result.errors());
    }

    #[test]
    fn test_invalid_number_in_field() {
        let input = r#"Rule : "pattern" => Type{num:0b102}"#; // '2' invalid in binary
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail on invalid binary number"
        );
        log_errors("invalid_number_in_field", input, result.errors());
    }

    #[test]
    fn test_unexpected_character_in_pattern() {
        let input = r#"Rule : "Hello {" => Type"#; // unclosed brace inside quoted pattern
        let result = rules().parse(input);

        assert!(
            result.has_errors(),
            "Expected parser to fail on unbalanced brace in pattern"
        );
        log_errors("unexpected_char_in_pattern", input, result.errors());
    }
}
