use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::prelude::*;

    #[test]
    fn test_simple_terminal_rule() {
        let input = r#"Greeting : "Hello" => Message"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "Greeting");
        assert_eq!(rule.pattern.len(), 1);

        if let Symbol::Terminal(text) = &rule.pattern[0] {
            assert_eq!(*text, "Hello");
        } else {
            panic!("Expected terminal symbol");
        }

        if let Some(RuleRhs::Type(name)) = &rule.rhs {
            assert_eq!(name, "Message");
        } else {
            panic!("Expected Some(Type)");
        }
    }

    #[test]
    fn test_placeholder_rule() {
        let input = r#"DoSomething : "{action:String}" => Action"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "DoSomething");
        assert_eq!(rule.pattern.len(), 1);

        if let Symbol::Placeholder { name, typ } = &rule.pattern[0] {
            assert_eq!(*name, "action");
            assert_eq!(*typ, "String");
        } else {
            panic!("Expected placeholder symbol");
        }
    }

    #[test]
    fn test_placeholder_rule_other_arrow() {
        let input = r#"DoSomething : "{action:String}" -> Action"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "DoSomething");
        assert_eq!(rule.pattern.len(), 1);

        if let Symbol::Placeholder { name, typ } = &rule.pattern[0] {
            assert_eq!(*name, "action");
            assert_eq!(*typ, "String");
        } else {
            panic!("Expected placeholder symbol");
        }
    }

    #[test]
    fn test_mixed_pattern() {
        let input = r#"DoSomethingElse : "{verb:String} {object:String}" => Action"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "DoSomethingElse");
        assert_eq!(rule.pattern.len(), 3); // verb, space, object

        if let Symbol::Placeholder { name, typ } = &rule.pattern[0] {
            assert_eq!(*name, "verb");
            assert_eq!(*typ, "String");
        } else {
            panic!("Expected first placeholder");
        }

        if let Symbol::Terminal(text) = &rule.pattern[1] {
            assert_eq!(*text, " ");
        } else {
            panic!("Expected space terminal");
        }

        if let Symbol::Placeholder { name, typ } = &rule.pattern[2] {
            assert_eq!(*name, "object");
            assert_eq!(*typ, "String");
        } else {
            panic!("Expected second placeholder");
        }
    }

    #[test]
    fn test_type_with_fields_identifiers() {
        let input = r#"Person : "Default Person" => Person{name:"defaultName", age:"defaultAge"}"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "Person");

        if let Some(RuleRhs::TypeWithFields { name, fields }) = &rule.rhs {
            assert_eq!(*name, "Person");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].0, "name");
            if let Value::StringLiteral(val) = &fields[0].1 {
                assert_eq!(val, "defaultName");
            } else {
                panic!("Expected string literal for name field");
            }

            assert_eq!(fields[1].0, "age");
            if let Value::StringLiteral(val) = &fields[1].1 {
                assert_eq!(val, "defaultAge");
            } else {
                panic!("Expected string literal for age field");
            }
        } else {
            panic!("Expected Some(TypeWithFields)");
        }
    }

    #[test]
    fn test_type_with_mixed_field_values() {
        let input = r#"SomeThing : "a pattern {name:Name}" => AnyThing{surname:"hey", num:2506, flt:12.565, ref:someRef}"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "SomeThing");

        assert_eq!(rule.pattern.len(), 2); // "a pattern " and {name:Name}

        if let Some(RuleRhs::TypeWithFields { name, fields }) = &rule.rhs {
            assert_eq!(*name, "AnyThing");
            assert_eq!(fields.len(), 4);

            assert_eq!(fields[0].0, "surname");
            if let Value::StringLiteral(val) = &fields[0].1 {
                assert_eq!(val, "hey");
            } else {
                panic!("Expected string literal for surname");
            }

            assert_eq!(fields[1].0, "num");
            if let Value::IntegerLiteral(val) = &fields[1].1 {
                assert_eq!(*val, 2506);
            } else {
                panic!("Expected integer literal for num");
            }

            assert_eq!(fields[2].0, "flt");
            if let Value::FloatLiteral(val) = &fields[2].1 {
                assert_eq!(*val, 12.565);
            } else {
                panic!("Expected float literal for flt");
            }

            assert_eq!(fields[3].0, "ref");
            if let Value::Identifier(val) = &fields[3].1 {
                assert_eq!(val, "someRef");
            } else {
                panic!("Expected identifier for ref");
            }
        } else {
            panic!("Expected Some(TypeWithFields)");
        }
    }

    #[test]
    fn test_implicit_output_type() {
        let input = r#"Something : "pattern with {place:Holders}""#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "Something");

        if let Some(RuleRhs::Type(name)) = &rule.rhs {
            assert_eq!(name, "Something");
        }
    }

    #[test]
    fn test_multiple_rules() {
        let input = r#"
Greeting : "Hello" => Message
Greeting : "Hi" => Message

DoSomething : "{action:String}" => Action
DoSomethingElse : "{verb:String} {object:String}" => Action

Person : "Default Person" => Person{name:"name", age:"age"}
"#;
        let result = rules().parse(input);
        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 5);

        assert_eq!(rules[0].lhs, "Greeting");
        assert_eq!(rules[1].lhs, "Greeting");

        if let (Symbol::Terminal(t1), Symbol::Terminal(t2)) =
            (&rules[0].pattern[0], &rules[1].pattern[0])
        {
            assert_eq!(*t1, "Hello");
            assert_eq!(*t2, "Hi");
        } else {
            panic!("Expected terminal symbols");
        }
    }

    #[test]
    fn test_whitespace_handling() {
        let input = r#"  Rule   :   "pattern"   =>   Type  "#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "Rule");

        if let Some(RuleRhs::Type(name)) = &rule.rhs {
            assert_eq!(name, "Type");
        } else {
            panic!("Expected Some(Type)");
        }
    }

    #[test]
    fn test_fields_with_whitespace() {
        let input = r#"Test : "test" => Type{ field1 : "value1" , field2 : 42 , field3 : 3.14 }"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        if let Some(RuleRhs::TypeWithFields { fields, .. }) = &rule.rhs {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0].0, "field1");
            assert_eq!(fields[1].0, "field2");
            assert_eq!(fields[2].0, "field3");
        } else {
            panic!("Expected Some(TypeWithFields)");
        }
    }

    #[test]
    fn test_rule_with_no_rhs_type_defaults_to_lhs() {
        let input = r#"Thing : "foo""#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().unwrap();
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        assert_eq!(rule.lhs, "Thing");

        if let Some(RuleRhs::Type(name)) = &rule.rhs {
            assert_eq!(name, "Thing");
        }
    }

    #[test]
    fn test_rule_with_structured_rhs() {
        let input = r#"Entity : "create" => Entity{name:"Bob", age:42}"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().unwrap();
        assert_eq!(rules.len(), 1);

        if let Some(RuleRhs::TypeWithFields { name, fields }) = &rules[0].rhs {
            assert_eq!(*name, "Entity");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].0, "name");
            match &fields[0].1 {
                Value::StringLiteral(s) => assert_eq!(*s, "Bob"),
                _ => panic!("Expected string literal"),
            }

            assert_eq!(fields[1].0, "age");
            match &fields[1].1 {
                Value::IntegerLiteral(n) => assert_eq!(*n, 42),
                _ => panic!("Expected integer literal"),
            }
        } else {
            panic!("Expected Some(TypeWithFields)");
        }
    }

    #[test]
    fn test_float_literal_in_fields() {
        let input = r#"Measure : "m" => Measure{value:3.14}"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().unwrap();
        assert_eq!(rules.len(), 1);

        if let Some(RuleRhs::TypeWithFields { fields, .. }) = &rules[0].rhs {
            assert_eq!(fields[0].0, "value");
            match &fields[0].1 {
                Value::FloatLiteral(f) => assert!((*f - 3.14).abs() < 1e-6),
                _ => panic!("Expected float literal"),
            }
        } else {
            panic!("Expected Some(TypeWithFields)");
        }
    }

    #[test]
    fn test_multiple_placeholders_no_space() {
        let input = r#"Cmd : "{first:Int}{second:Int}" => Action"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().unwrap();
        let pattern = &rules[0].pattern;
        assert_eq!(pattern.len(), 2);

        if let Symbol::Placeholder { name, typ } = &pattern[0] {
            assert_eq!(*name, "first");
            assert_eq!(*typ, "Int");
        } else {
            panic!("Expected first placeholder");
        }

        if let Symbol::Placeholder { name, typ } = &pattern[1] {
            assert_eq!(*name, "second");
            assert_eq!(*typ, "Int");
        } else {
            panic!("Expected second placeholder");
        }
    }

    #[test]
    fn test_empty_pattern() {
        let input = r#"Empty : "" => Nothing"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());

        let rules = result.output().unwrap();
        let pattern = &rules[0].pattern;
        assert!(pattern.is_empty(), "Expected empty pattern");
    }

    #[test]
    fn test_all_number_types_in_fields() {
        let input = r#"Numbers : "nums" => Numbers{
        dec:42,
        bin:0b1010,
        oct:0o77,
        hex:0xFF,
        flt:3.14,
        sci:1.5e2,
        neg:-42,
        negflt:-2.5e-1
    }"#;

        let result = rules().parse(input);
        assert!(!result.has_errors());

        let fields = if let Some(RuleRhs::TypeWithFields { fields, .. }) =
            &result.output().unwrap()[0].rhs
        {
            fields
        } else {
            panic!("Expected Some(TypeWithFields)");
        };

        assert_eq!(fields.len(), 8);
    }
}
