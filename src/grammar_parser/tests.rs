use super::*;
use chumsky::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn unwrap_normal<'gr>(pat: &'gr Pattern<'gr>) -> &'gr Vec<Symbol<'gr>> {
        match pat {
            Pattern::Normal(v) => v,
            _ => panic!("Expected Normal pattern"),
        }
    }

    fn unwrap_disjunction<'gr>(pat: &'gr Pattern<'gr>) -> &'gr Vec<Symbol<'gr>> {
        match pat {
            Pattern::Disjunction(v) => v,
            _ => panic!("Expected Disjunction pattern"),
        }
    }

    #[test]
    fn test_simple_terminal_rule() {
        let input = r#"Greeting : "Hello" => Message"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rules = result.output().expect("Should have output");
        assert_eq!(rules.len(), 1);

        let rule = &rules[0];
        let pattern = unwrap_normal(&rule.pattern);
        assert_eq!(rule.lhs, "Greeting");
        assert_eq!(pattern.len(), 1);

        if let Symbol::Terminal(text) = &pattern[0] {
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
        let rule = &result.output().unwrap()[0];
        let pattern = unwrap_normal(&rule.pattern);
        assert_eq!(pattern.len(), 1);

        if let Symbol::Placeholder { name, typ } = &pattern[0] {
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
        let rule = &result.output().unwrap()[0];
        let pattern = unwrap_normal(&rule.pattern);
        assert_eq!(pattern.len(), 1);

        if let Symbol::Placeholder { name, typ } = &pattern[0] {
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
        let rule = &result.output().unwrap()[0];
        let pattern = unwrap_normal(&rule.pattern);
        assert_eq!(pattern.len(), 3);

        if let Symbol::Placeholder { name, typ } = &pattern[0] {
            assert_eq!(*name, "verb");
            assert_eq!(*typ, "String");
        } else {
            panic!("Expected first placeholder");
        }

        if let Symbol::Terminal(text) = &pattern[1] {
            assert_eq!(*text, " ");
        } else {
            panic!("Expected space terminal");
        }

        if let Symbol::Placeholder { name, typ } = &pattern[2] {
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
        let rule = &result.output().unwrap()[0];
        assert_eq!(rule.lhs, "Person");

        if let Some(RuleRhs::TypeWithFields { name, fields }) = &rule.rhs {
            assert_eq!(*name, "Person");
            assert_eq!(fields.len(), 2);

            assert_eq!(fields[0].0, "name");
            if let ValueSpec::StringLiteral(val) = &fields[0].1 {
                assert_eq!(val, "defaultName");
            }

            assert_eq!(fields[1].0, "age");
            if let ValueSpec::StringLiteral(val) = &fields[1].1 {
                assert_eq!(val, "defaultAge");
            }
        }
    }

    #[test]
    fn test_type_with_mixed_field_values() {
        let input = r#"SomeThing : "a pattern {name:Name}" => AnyThing{surname:"hey", num:2506, flt:12.565, ref:someRef}"#;
        let result = rules().parse(input);

        assert!(!result.has_errors());
        let rule = &result.output().unwrap()[0];
        let pattern = unwrap_normal(&rule.pattern);
        assert_eq!(pattern.len(), 2);

        if let Some(RuleRhs::TypeWithFields { name, fields }) = &rule.rhs {
            assert_eq!(*name, "AnyThing");
            assert_eq!(fields.len(), 4);
        }
    }

    #[test]
    fn test_implicit_output_type() {
        let input = r#"Something : "pattern with {place:Holders}""#;
        let result = rules().parse(input);
        let rule = &result.output().unwrap()[0];

        if let Some(RuleRhs::Type(name)) = &rule.rhs {
            assert_eq!(name, "Something");
        }
    }

    #[test]
    fn test_multiple_rules() {
        let input = r#"
Greeting : "Hello" => Message
Greeting : "Hi" => Message
"#;
        let result = rules().parse(input).unwrap();
        assert_eq!(result.len(), 2);

        let pattern1 = unwrap_normal(&result[0].pattern);
        let pattern2 = unwrap_normal(&result[1].pattern);

        if let (Symbol::Terminal(t1), Symbol::Terminal(t2)) = (&pattern1[0], &pattern2[0]) {
            assert_eq!(*t1, "Hello");
            assert_eq!(*t2, "Hi");
        }
    }

    #[test]
    fn test_empty_pattern() {
        let input = r#"Empty : "" => Nothing"#;
        let result = rules().parse(input).unwrap();
        let pattern = unwrap_normal(&result[0].pattern);
        assert!(pattern.is_empty());
    }

    #[test]
    fn test_disjunction_rule() {
        let input = r#"Foo : Bar | Baz | Bez"#;
        let result = rules().parse(input).unwrap();
        let rule = &result[0];

        let alts = unwrap_disjunction(&rule.pattern);
        assert_eq!(alts.len(), 3);
        for sym in alts {
            if let Symbol::NonTerminal(nt) = sym {
                assert!(["Bar", "Baz", "Bez"].contains(&nt.as_ref()));
            } else {
                panic!("Expected nonterminal in disjunction");
            }
        }
    }
}
