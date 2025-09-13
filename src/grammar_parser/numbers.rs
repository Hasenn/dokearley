use super::ValueSpec;
use chumsky::prelude::*;

#[derive(Debug, Clone)]
enum NumLit<'gr> {
    DecInt(&'gr str, Option<char>),
    BinInt(&'gr str, Option<char>),
    OctInt(&'gr str, Option<char>),
    HexInt(&'gr str, Option<char>),
    Float(&'gr str), // full literal (with sign included)
}

pub(crate) fn number_literal<'gr>(
) -> impl Parser<'gr, &'gr str, ValueSpec<'gr>, extra::Err<Rich<'gr, char>>> {
    let sign = just('-').or(just('+')).or_not();

    let bin = sign
        .then_ignore(just("0b"))
        .then(text::digits(2).to_slice())
        .map(|(s, d)| NumLit::BinInt(d, s));

    let oct = sign
        .then_ignore(just("0o"))
        .then(text::digits(8).to_slice())
        .map(|(s, d)| NumLit::OctInt(d, s));

    let hex = sign
        .then_ignore(just("0x"))
        .then(text::digits(16).to_slice())
        .map(|(s, d)| NumLit::HexInt(d, s));

    let dec = sign
        .then(text::digits(10).to_slice())
        .map(|(s, d)| NumLit::DecInt(d, s));

    // Floats: optional sign + digits + '.' + digits + optional exponent
    let float = sign
        .then(
            text::digits(10)
                .or_not()
                .then_ignore(just('.'))
                .then(text::digits(10).or_not())
                .then(
                    just('e')
                        .or(just('E'))
                        .ignore_then(just('-').or(just('+')).or_not().then(text::digits(10)))
                        .or_not(),
                ),
        )
        .to_slice()
        .map(NumLit::Float);

    choice((float, bin, oct, hex, dec)).try_map(|num, span| match num {
        NumLit::Float(lit) => lit
            .parse::<f64>()
            .map(ValueSpec::FloatLiteral)
            .map_err(|e| Rich::custom(span, format!("Invalid float: {}", e))),

        NumLit::DecInt(digits, sign) => {
            let mut val = i64::from_str_radix(digits, 10)
                .map_err(|e| Rich::custom(span, format!("Invalid decimal int: {}", e)))?;
            if sign == Some('-') {
                val = -val;
            }
            Ok(ValueSpec::IntegerLiteral(val))
        }

        NumLit::BinInt(digits, sign) => {
            let mut val = i64::from_str_radix(digits, 2)
                .map_err(|e| Rich::custom(span, format!("Invalid binary int: {}", e)))?;
            if sign == Some('-') {
                val = -val;
            }
            Ok(ValueSpec::IntegerLiteral(val))
        }

        NumLit::OctInt(digits, sign) => {
            let mut val = i64::from_str_radix(digits, 8)
                .map_err(|e| Rich::custom(span, format!("Invalid octal int: {}", e)))?;
            if sign == Some('-') {
                val = -val;
            }
            Ok(ValueSpec::IntegerLiteral(val))
        }

        NumLit::HexInt(digits, sign) => {
            let mut val = i64::from_str_radix(digits, 16)
                .map_err(|e| Rich::custom(span, format!("Invalid hex int: {}", e)))?;
            if sign == Some('-') {
                val = -val;
            }
            Ok(ValueSpec::IntegerLiteral(val))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_decimal_integers() {
        let cases = [
            ("0", 0),
            ("42", 42),
            ("-42", -42),
            ("+7", 7),
            ("1627326", 1627326),
        ];

        for (input, expected) in cases {
            let result = number_literal().parse(input);

            // Inline Chumsky error handling
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("{} at {}", e, e.span());
            }
            assert!(
                !result.has_errors(),
                "Expected parser to succeed for '{}'",
                input
            );

            match result.output().unwrap() {
                ValueSpec::IntegerLiteral(n) => {
                    assert_eq!(*n, expected, "Wrong value for '{}'", input)
                }
                other => panic!("Expected integer literal for '{}', got {:?}", input, other),
            }
        }
    }

    #[test]
    fn test_binary_integers() {
        let cases = [
            ("0b1010", 0b1010),
            ("-0b1010", -0b1010),
            ("+0b1111", 0b1111),
        ];

        for (input, expected) in cases {
            let result = number_literal().parse(input);
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("{} at {}", e, e.span());
            }
            assert!(
                !result.has_errors(),
                "Expected parser to succeed for '{}'",
                input
            );

            match result.output().unwrap() {
                ValueSpec::IntegerLiteral(n) => {
                    assert_eq!(*n, expected, "Wrong binary value for '{}'", input)
                }
                other => panic!(
                    "Expected integer literal for binary '{}', got {:?}",
                    input, other
                ),
            }
        }
    }

    #[test]
    fn test_octal_integers() {
        let cases = [("0o70", 0o70), ("-0o123", -0o123), ("+0o777", 0o777)];

        for (input, expected) in cases {
            let result = number_literal().parse(input);
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("{} at {}", e, e.span());
            }
            assert!(
                !result.has_errors(),
                "Expected parser to succeed for '{}'",
                input
            );

            match result.output().unwrap() {
                ValueSpec::IntegerLiteral(n) => {
                    assert_eq!(*n, expected, "Wrong octal value for '{}'", input)
                }
                other => panic!(
                    "Expected integer literal for octal '{}', got {:?}",
                    input, other
                ),
            }
        }
    }

    #[test]
    fn test_hex_integers() {
        let cases = [("0x1A", 0x1A), ("-0x1A", -0x1A), ("+0xFF", 0xFF)];

        for (input, expected) in cases {
            let result = number_literal().parse(input);
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("{} at {}", e, e.span());
            }
            assert!(
                !result.has_errors(),
                "Expected parser to succeed for '{}'",
                input
            );

            match result.output().unwrap() {
                ValueSpec::IntegerLiteral(n) => {
                    assert_eq!(*n, expected, "Wrong hex value for '{}'", input)
                }
                other => panic!(
                    "Expected integer literal for hex '{}', got {:?}",
                    input, other
                ),
            }
        }
    }

    #[test]
    fn test_float_literals() {
        let cases = [
            ("1.5", 1.5),
            ("1510151.", 1510151.0),
            ("0.001", 0.001),
            ("-1.515", -1.515),
            ("+3.14", 3.14),
        ];

        for (input, expected) in cases {
            let result = number_literal().parse(input);
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("{} at {}", e, e.span());
            }
            assert!(
                !result.has_errors(),
                "Expected parser to succeed for '{}'",
                input
            );

            match result.output().unwrap() {
                ValueSpec::FloatLiteral(f) => {
                    assert_eq!(*f, expected, "Wrong float value for '{}'", input)
                }
                other => panic!("Expected float literal for '{}', got {:?}", input, other),
            }
        }
    }

    #[test]
    fn test_scientific_floats() {
        let cases = [
            ("1.5252e10", 1.5252e10),
            ("1.54e-10", 1.54e-10),
            ("-1.2e3", -1.2e3),
            ("+3.4E5", 3.4e5),
        ];

        for (input, expected) in cases {
            let result = number_literal().parse(input);
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("{} at {}", e, e.span());
            }
            assert!(
                !result.has_errors(),
                "Expected parser to succeed for '{}'",
                input
            );

            match result.output().unwrap() {
                ValueSpec::FloatLiteral(f) => {
                    assert_eq!(*f, expected, "Wrong scientific float for '{}'", input)
                }
                other => panic!("Expected float literal for '{}', got {:?}", input, other),
            }
        }
    }

    #[test]
    fn test_invalid_numbers() {
        let cases = ["0b102", "0o89", "0x1G", "1.2.3", "--42"];

        for input in cases {
            let result = number_literal().parse(input);
            let errors: Vec<_> = result.errors().collect();
            for e in &errors {
                println!("Expected parse error: {} at {}", e, e.span());
            }
            assert!(
                result.has_errors(),
                "Expected parser to fail for '{}'",
                input
            );
        }
    }
}
