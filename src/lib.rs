use crate::{
    grammar_parser::rules,
    recognizer::{Chart, Grammar},
};
use chumsky::Parser;
use thiserror::Error;
mod conversion;
pub mod grammar_parser;

mod parser;
mod recognizer;
mod try_accept;

#[cfg(test)]
mod mock_values;

pub struct Dokearley<'gr> {
    grammar: Grammar<'gr>,
}

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Resource {
        typ: String,
        fields: HashMap<String, Value>,
    },
    Array(Vec<Value>),
    Dictionary(HashMap<String, Value>),
}

impl<'gr, 'inp> From<crate::parser::Value<'gr, 'inp>> for Value {
    fn from(v: crate::parser::Value<'gr, 'inp>) -> Self {
        match v {
            parser::Value::Integer(i) => Value::Integer(i),
            parser::Value::Float(f) => Value::Float(f),
            parser::Value::String(s) => Value::String(s.to_string()),
            parser::Value::Resource { typ, fields } => Value::Resource {
                typ: typ.to_string(),
                fields: fields
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.into()))
                    .collect(),
            },
            parser::Value::Bool(b) => Value::Bool(b),
            parser::Value::Dictionary(fields) => Value::Dictionary({
                fields
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.into()))
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Error)]
pub enum DokearleyError {
    #[error("Error(s) while parsing the grammar : {0}")]
    InvalidDokedef(String),
    #[error("Error while parsing input : {0}")]
    ParseError(#[from] try_accept::ParseError),
    #[error("Could not build parse tree, this is a bug in Dokearley!!")]
    DokearleyBuildParseTreeError,
    #[error("There is an infinite loop of nullable symbols in the provided grammar")]
    InfiniteNullableLoop,
}

/// A parser that recognizes and parses a custom grammar, defined in a `dokedef` file.
impl<'gr> Dokearley<'gr> {
    /// Builds a parser from a `dokedef` grammar string
    pub fn from_dokedef(grammar_string: &'gr str) -> Result<Self, DokearleyError> {
        Ok(Self {
            grammar: {
                let rules = rules::<'gr>().parse(grammar_string);
                if rules.has_errors() {
                    Err(DokearleyError::InvalidDokedef({
                        let errors = rules.errors();
                        let mut error_string = "".to_string();
                        for e in errors {
                            error_string += &("\n".to_string() + &e.to_string());
                        }
                        error_string
                    }))?
                } else {
                    let rules = rules.output();
                    if let Some(rules) = rules {
                        let grammar: Grammar<'gr> = rules.into();
                        if grammar.has_infinite_loop() {
                            Err(DokearleyError::InfiniteNullableLoop)?
                        }
                        grammar
                    } else {
                        Err(DokearleyError::InvalidDokedef("??".to_string()))?
                    }
                }
            },
        })
    }
}

impl<'gr> Dokearley<'gr> {
    /// Parses an input into a `Value`with the parser's grammar, starting from a non-terminal `start`.
    /// The `start` specifies what we are trying to parse.
    pub fn parse<'inp>(
        &'gr self,
        input: &'inp str,
        start: &'inp str,
    ) -> Result<Value, DokearleyError>
    where
        'gr: 'inp,
    {
        let tokens = recognizer::tokenize(input);
        let mut chart = Chart::new(&self.grammar, tokens, start);
        chart.recognize(start);
        chart.try_accept(start)?;
        let tree = chart
            .build_parse_tree()
            .ok_or(DokearleyError::DokearleyBuildParseTreeError)?;
        Ok(tree.compute_value().into())
    }
}

#[cfg(test)]
mod item_effects_tests {
    use super::*;
    use std::collections::HashMap;

    fn make_engine() -> Dokearley<'static> {
        let grammar = r#"
ItemEffect: "deal {amount:Int} damage" -> Damage
ItemEffect: "heal for {amount:Int}" -> Heal
ItemEffect: "apply {status:String}" -> ApplyStatus
ItemEffect: "remove {status:String}" -> RemoveStatus
ItemEffect: "increase {stat:String} by {amount:Int}" -> Buff 
ItemEffect: "decrease {stat:String} by {amount:Int}" -> Debuff 

ItemEffect: "to {target : Target} : {effect : ItemEffect}" -> TargetedEffect

Target: "self" -> Target { kind: "self" }
Target: "an ally" -> Target { kind: "ally" }
Target: "an enemy" -> Target { kind: "enemy" }
Target: "all allies" -> Target { kind: "allies" }
Target: "all enemies" -> Target { kind: "enemies" }
"#;

        Dokearley::from_dokedef(grammar).expect("invalid grammar")
    }

    #[test]
    fn parse_heal_self() {
        let engine = make_engine();
        let result = engine.parse("to self : heal for 7", "ItemEffect").unwrap();
        print!("{:?}", &result);
        match result {
            Value::Resource { typ, fields } => {
                assert_eq!(typ, "TargetedEffect");
                assert_eq!(
                    fields["target"],
                    Value::Resource {
                        typ: "Target".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("kind".into(), Value::String("self".into()));
                            m
                        }
                    }
                );
                assert_eq!(
                    fields["effect"],
                    Value::Resource {
                        typ: "Heal".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("amount".into(), Value::Integer(7));
                            m
                        }
                    }
                );
            }
            _ => panic!("unexpected parse output: {:?}", result),
        }
    }

    #[test]
    fn parse_damage_enemy() {
        let engine = make_engine();
        let result = engine
            .parse("to an enemy : deal 7 damage", "ItemEffect")
            .unwrap();
        match result {
            Value::Resource { typ, fields } => {
                assert_eq!(typ, "TargetedEffect");
                assert_eq!(
                    fields["target"],
                    Value::Resource {
                        typ: "Target".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("kind".into(), Value::String("enemy".into()));
                            m
                        }
                    }
                );
                assert_eq!(
                    fields["effect"],
                    Value::Resource {
                        typ: "Damage".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("amount".into(), Value::Integer(7));
                            m
                        }
                    }
                );
            }
            _ => panic!("unexpected parse output: {:?}", result),
        }
    }

    #[test]
    fn parse_buff_allies() {
        let engine = make_engine();
        let result = engine
            .parse("to all allies : increase \"strength\" by 5", "ItemEffect")
            .unwrap();
        match result {
            Value::Resource { typ, fields } => {
                assert_eq!(typ, "TargetedEffect");
                assert_eq!(
                    fields["target"],
                    Value::Resource {
                        typ: "Target".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("kind".into(), Value::String("allies".into()));
                            m
                        }
                    }
                );
                assert_eq!(
                    fields["effect"],
                    Value::Resource {
                        typ: "Buff".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("stat".into(), Value::String("strength".into()));
                            m.insert("amount".into(), Value::Integer(5));
                            m
                        }
                    }
                );
            }
            _ => panic!("unexpected parse output: {:?}", result),
        }
    }

    #[test]
    fn parse_remove_status() {
        let engine = make_engine();
        let result = engine.parse("remove \"poison\"", "ItemEffect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "RemoveStatus".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("status".into(), Value::String("poison".into()));
                    m
                }
            }
        );
    }
}

#[cfg(test)]
mod emoji_effects_tests {
    use super::*;
    use std::collections::HashMap;

    fn make_engine() -> Dokearley<'static> {
        // Grammar that directly uses emojis as tokens
        let grammar = r#"
ItemEffect: "🔥 {amount:Int}" -> FireDamage
ItemEffect: "💖 {amount:Int}" -> Heal
ItemEffect: "💀" -> ApplyStatus { status: "death" }
ItemEffect: "😡" -> ApplyStatus { status: "rage" }
ItemEffect: "🛡️+{amount:Int}" -> Buff { stat: "defense" }
ItemEffect: "🗡️+{amount:Int}" -> Buff { stat: "attack" }

ItemEffect: "{target:Target} {effect:ItemEffect}" -> TargetedEffect

Target: "🙂" -> Target { kind: "self" }
Target: "🤝" -> Target { kind: "ally" }
Target: "👹" -> Target { kind: "enemy" }
Target: "👨‍👩‍👦" -> Target { kind: "allies" }
Target: "👥" -> Target { kind: "enemies" }
"#;

        Dokearley::from_dokedef(grammar).expect("invalid emoji grammar")
    }

    #[test]
    fn parse_fire_damage_enemy() {
        let engine = make_engine();
        let result = engine.parse("👹 🔥 10", "ItemEffect").unwrap();
        match result {
            Value::Resource { typ, fields } => {
                assert_eq!(typ, "TargetedEffect");
                assert_eq!(
                    fields["target"],
                    Value::Resource {
                        typ: "Target".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("kind".into(), Value::String("enemy".into()));
                            m
                        }
                    }
                );
                assert_eq!(
                    fields["effect"],
                    Value::Resource {
                        typ: "FireDamage".into(),
                        fields: {
                            let mut m = HashMap::new();
                            m.insert("amount".into(), Value::Integer(10));
                            m
                        }
                    }
                );
            }
            _ => panic!("unexpected parse output: {:?}", result),
        }
    }

    #[test]
    fn parse_heal_self() {
        let engine = make_engine();
        let result = engine.parse("🙂 💖 7", "ItemEffect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "TargetedEffect".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert(
                        "target".into(),
                        Value::Resource {
                            typ: "Target".into(),
                            fields: {
                                let mut m = HashMap::new();
                                m.insert("kind".into(), Value::String("self".into()));
                                m
                            },
                        },
                    );
                    m.insert(
                        "effect".into(),
                        Value::Resource {
                            typ: "Heal".into(),
                            fields: {
                                let mut m = HashMap::new();
                                m.insert("amount".into(), Value::Integer(7));
                                m
                            },
                        },
                    );
                    m
                }
            }
        );
    }

    #[test]
    fn parse_apply_status_skull() {
        let engine = make_engine();
        let result = engine.parse("💀", "ItemEffect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "ApplyStatus".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("status".into(), Value::String("death".into()));
                    m
                }
            }
        );
    }

    #[test]
    fn parse_buff_attack() {
        let engine = make_engine();
        let result = engine.parse("🗡️+5", "ItemEffect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "Buff".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("stat".into(), Value::String("attack".into()));
                    m.insert("amount".into(), Value::Integer(5));
                    m
                }
            }
        );
    }
}

#[cfg(test)]
mod transparent_rules_tests {
    use super::*;
    use std::collections::HashMap;

    fn make_engine() -> Dokearley<'static> {
        // Transparent rules: Effect can be either DamageEffect or HealEffect
        let grammar = r#"
Effect : DamageEffect
Effect : HealEffect

DamageEffect : "deal {amount:Int} damage" -> Damage
HealEffect   : "heal for {amount:Int}"    -> Heal
"#;

        Dokearley::from_dokedef(grammar).expect("invalid grammar")
    }

    #[test]
    fn parse_damage_effect_through_effect() {
        let engine = make_engine();
        let result = engine.parse("deal 10 damage", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "Damage".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("amount".into(), Value::Integer(10));
                    m
                }
            }
        );
    }

    #[test]
    fn parse_heal_effect_through_effect() {
        let engine = make_engine();
        let result = engine.parse("heal for 7", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "Heal".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("amount".into(), Value::Integer(7));
                    m
                }
            }
        );
    }
}

#[cfg(test)]
mod disjunction_rules_tests {
    use super::*;
    use std::collections::HashMap;

    fn make_engine() -> Dokearley<'static> {
        // Transparent rules: Effect can be either DamageEffect or HealEffect
        let grammar = r#"
Effect : DamageEffect | HealEffect

DamageEffect : "deal {amount:Int} damage" -> Damage
HealEffect   : "heal for {amount:Int}"    -> Heal
"#;

        Dokearley::from_dokedef(grammar).expect("invalid grammar")
    }

    #[test]
    fn parse_damage_effect_through_effect() {
        let engine = make_engine();
        let result = engine.parse("deal 10 damage", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "Damage".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("amount".into(), Value::Integer(10));
                    m
                }
            }
        );
    }

    #[test]
    fn parse_heal_effect_through_effect() {
        let engine = make_engine();
        let result = engine.parse("heal for 7", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Resource {
                typ: "Heal".into(),
                fields: {
                    let mut m = HashMap::new();
                    m.insert("amount".into(), Value::Integer(7));
                    m
                }
            }
        );
    }
}

#[cfg(test)]
mod dictionary_outspecs_tests {
    use super::*;
    use std::collections::HashMap;

    fn make_engine() -> Dokearley<'static> {
        // Grammar where RHS directly produces dictionaries
        let grammar = r#"
Effect: "gain {amount:Int} gold" -> { kind: "gain_gold"}
Effect: "lose {amount:Int} health" -> { kind: "lose_health"}
Effect: "status {status:String}" -> { kind: "status", value: status}
"#;

        Dokearley::from_dokedef(grammar).expect("invalid dictionary grammar")
    }

    #[test]
    fn parse_gain_gold() {
        let engine = make_engine();
        let result = engine.parse("gain 5 gold", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Dictionary({
                let mut m = HashMap::new();
                m.insert("kind".into(), Value::String("gain_gold".into()));
                m.insert("amount".into(), Value::Integer(5));
                m
            })
        );
    }

    #[test]
    fn parse_lose_health() {
        let engine = make_engine();
        let result = engine.parse("lose 3 health", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Dictionary({
                let mut m = HashMap::new();
                m.insert("kind".into(), Value::String("lose_health".into()));
                m.insert("amount".into(), Value::Integer(3));
                m
            })
        );
    }

    #[test]
    fn parse_status() {
        let engine = make_engine();
        let result = engine.parse("status \"burned\"", "Effect").unwrap();
        assert_eq!(
            result,
            Value::Dictionary({
                let mut m = HashMap::new();
                m.insert("value".into(), Value::String("burned".into()));
                m.insert("kind".into(), Value::String("status".into()));
                m.insert("status".into(), Value::String("burned".into()));
                m
            })
        );
    }
}
