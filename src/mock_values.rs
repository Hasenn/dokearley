use crate::grammar_parser::Str;
use crate::recognizer::ValueSpec;
use chumsky::span::SimpleSpan;
use std::collections::HashMap;
use std::ops::Range;

// Helper functions for creating Value instances with proper spans
impl<'gr> ValueSpec<'gr> {
    /// Create an Identifier Value with a span covering the entire text
    pub fn mock_identifier(text: &'gr str) -> Self {
        ValueSpec::Identifier(Str {
            text,
            span: SimpleSpan::from(0..text.len()),
        })
    }

    /// Create a StringLiteral Value with a span covering the entire text
    pub fn mock_string_literal(text: &'gr str) -> Self {
        ValueSpec::StringLiteral(Str {
            text,
            span: SimpleSpan::from(0..text.len()),
        })
    }

    /// Create an Identifier Value with a custom span
    pub fn identifier_with_span(text: &'gr str, span: Range<usize>) -> Self {
        ValueSpec::Identifier(Str {
            text,
            span: SimpleSpan::from(span),
        })
    }

    /// Create a StringLiteral Value with a custom span
    pub fn string_literal_with_span(text: &'gr str, span: Range<usize>) -> Self {
        ValueSpec::StringLiteral(Str {
            text,
            span: SimpleSpan::from(span),
        })
    }

    /// Create an IntegerLiteral Value
    pub fn mock_integer_literal(value: i64) -> Self {
        ValueSpec::IntegerLiteral(value)
    }

    /// Create a FloatLiteral Value
    pub fn mock_float_literal(value: f64) -> Self {
        ValueSpec::FloatLiteral(value)
    }
}

// Extension trait for easy conversion from &str to Value
pub trait MockValue<'gr> {
    fn as_identifier(&self) -> ValueSpec<'gr>;
    fn as_string_literal(&self) -> ValueSpec<'gr>;
    fn as_identifier_with_span(&self, span: Range<usize>) -> ValueSpec<'gr>;
    fn as_string_literal_with_span(&self, span: Range<usize>) -> ValueSpec<'gr>;
}

impl<'gr> MockValue<'gr> for &'gr str {
    fn as_identifier(&self) -> ValueSpec<'gr> {
        ValueSpec::mock_identifier(self)
    }

    fn as_string_literal(&self) -> ValueSpec<'gr> {
        ValueSpec::mock_string_literal(self)
    }

    fn as_identifier_with_span(&self, span: Range<usize>) -> ValueSpec<'gr> {
        ValueSpec::identifier_with_span(self, span)
    }

    fn as_string_literal_with_span(&self, span: Range<usize>) -> ValueSpec<'gr> {
        ValueSpec::string_literal_with_span(self, span)
    }
}

// Helper function to create HashMap for OutSpec::Resource
pub fn mock_resource_fields<'gr, const N: usize>(
    fields: [(&'gr str, ValueSpec<'gr>); N],
) -> HashMap<&'gr str, ValueSpec<'gr>> {
    fields.into_iter().collect()
}

// Additional helper for creating more realistic test scenarios
pub fn create_realistic_span(text: &str, start: usize) -> SimpleSpan<usize> {
    SimpleSpan::from(start..start + text.len())
}

// Example usage in tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_value_creation_with_spans() {
        // Create Value instances with automatic spans
        let ident = ValueSpec::mock_identifier("name");
        let string_lit = ValueSpec::mock_string_literal("hello");

        // Verify spans are correctly created
        if let ValueSpec::Identifier(Str { span, .. }) = ident {
            assert_eq!(span.start, 0);
            assert_eq!(span.end, 4); // "name" is 4 characters
        }

        if let ValueSpec::StringLiteral(Str { span, .. }) = string_lit {
            assert_eq!(span.start, 0);
            assert_eq!(span.end, 5); // "hello" is 5 characters
        }
    }

    #[test]
    fn test_custom_span_creation() {
        // Create Value instances with custom spans
        let ident = "name".as_identifier_with_span(10..14);
        let string_lit = "hello".as_string_literal_with_span(20..25);

        if let ValueSpec::Identifier(Str { span, .. }) = ident {
            assert_eq!(span.start, 10);
            assert_eq!(span.end, 14);
        }

        if let ValueSpec::StringLiteral(Str { span, .. }) = string_lit {
            assert_eq!(span.start, 20);
            assert_eq!(span.end, 25);
        }
    }

    #[test]
    fn test_realistic_span_creation() {
        let text = "test";
        let start_pos = 42;
        let span = create_realistic_span(text, start_pos);

        assert_eq!(span.start, 42);
        assert_eq!(span.end, 46); // 42 + 4
    }
}
