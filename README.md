# Dokearley

Earley parser and grammar parser for `Doke`. Allows game-devs to write DSLs quickly to edit their data in natural-like language.

Includes `dokedef`, a language to write those DSLs, and a parser to parse those DSLs, returning a Resource-like rust enum.


## Example

Define a grammar for RPG-style item effects in a file, for example:

```
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
```

Then statements like these :

```
to self : heal for 7
to an enemy : deal 7 damage
to all allies : increase "strength" by 5
remove "poison"
```

Will give you resources like these

```
TargetedEffect {
  target: Target { kind: "self" },
  effect: Heal { amount: 7 }
}

TargetedEffect {
  target: Target { kind: "enemy" },
  effect: Damage { amount: 7 }
}

TargetedEffect {
  target: Target { kind: "allies" },
  effect: Buff { stat: "strength", amount: 5 }
}

RemoveStatus { status: "poison" }
```
*Strings are written as litterals, this is not exactly superb, i will add config to allow using some other syntax that looks pretty in markdown, like **poison**. *

The String type could also be split into multiple "aliases" and you could specify `{stat : BoldString}` `{stat : WikiLink}` allowing **poison** or [[poison]] (which is good with editors like Obsidian, as it shows you what undocumented status effect you might have used and allows you to link to their definition in the wiki right then and there)
In this case, i would allow `String` to take any of these. This needs more specification though.

# Notes

It should be said that for ambiguous grammars, this parser is O(n^3). 
Though in practice, this is meant for small statements of bounded size and therefore would still be linear.

Earley is also quite efficient for this use case.
Because this is meant for human-readable languages, there are a lot of terminals that structure the data, often way more than placeholders/non-terminals, and Earley parsers happily chug along the terminals without creating an explosion of items.

Supports nullable rules (at the moment they are tested but don't have a definite syntax in the dokedef language)

Gives a best attempt at a good error message, showing earley items that could have been meant, as we don't know the language in advance.

As of now, in the user language, 102020 is always an int, and 65.5 a float, and they are treated as tokens. You can use 2. for floats that look like ints, but I admit it's not too great. I'll be working on this when I get to using the language.

You will find that there are no bools. I will also add them, though probably only in output specs, as i'm sure anyone's ideal human readable DSL doesn't look like `Do something : true`, and more like `Do something`

Currently case-sensitive. I will maybe add a symbol group syntax like `[aA]` to allow for quick handling of that, or add an optional lowercase matching.

# Dokedef File Format

This project provides a **domain-specific grammar format** for defining game mechanics, actions, and effects. Unlike general-purpose language grammars, this format is **tailored for game-making**, focusing on structured actions, placeholders, and output values.

It is designed to work with our parser and recognizer, generating Resources (effects, actions, etc.) from structured text inputs.

---

## File Structure

A grammar file consists of **productions**, each of which has:

- **LHS (Left-Hand Side)** – the "nonterminal" being defined. You can think of it like an abstract type.
    
- **RHS (Right-Hand Side)** – a pattern, including placeholders, and nonterminals
    
- **Output Spec** – the type or expression produced by this rule
    

The general format is:

```
NonTerminal : "literal text with optional {things : Type}" -> OutputSpec
```

### Key Concepts

#### Terminals

- **Literal strings** in double quotes represent fixed tokens that must appear in the input.
    
- Example: `"Deal"` or `"then"`
    

#### Placeholders

- **Curly-braced fields** represent values captured from the input.
    
- Syntax: `{name : Type}`
    
- Types can be **built-in** (`Int`, `Float`, `String`) or **user-defined non-terminals**.
    
- Example: `{dmg : Int}` or `{then : Effect}`
    

#### Nonterminals

- Represent composable rules defined elsewhere in the grammar.
    
- Example: `Effect`, `DamageEffect`, `EffectThenEffect`
    

#### Output Specification

- Determines what the parser produces when the rule matches.
    
- Can be:
    
    - A single type: `DamageEffect`
        
    - A composite Resource with fixed fields: `PartiallyBuiltType{foo : "bar", bar : 2501}`
        
    - Propagation of fields from placeholders: `{}` giving all the fields to the placeholder's rule. (UNSTABLE, will become dicts instead !)
        

---

## Examples

### 1. Defining Simple Effects

```
Effect : "Deal {dmg : Int} damage" -> DamageEffect
```

- Matches input like: `"Deal 42 damage"`
    
- Produces a `DamageEffect` object with the field `dmg = 42`.
    

---

### 2. Sequenced Effects

```
Effect : "{first : Effect}, then : {then : Effect}" -> EffectThenEffect
```

- Matches input like: `"Deal 10, then Deal 5"`
    
- Produces `EffectThenEffect { first, then }` where first is DamageEffect {dmg : 10}, etc...
    

---

### 3. Propagating Fields

```
NonTerminal : "pattern with {place : holders}" -> {}
```

- Automatically propagates captured fields into the output.
    
- Useful when intermediate structures are not needed, but you want to forward fields.
    

---
### 5. Disjonction

```
Effect : DamageEffect | SomeEffect | AnotherThing
```

- Allows **multiple alternatives**.
    
- The parser accepts any of the listed types, producing the corresponding value.
    
- Useful to separate Effects into types that can be accepted with more granularity.
    For example, allowing only damage effects somewhere while still being able to allow any effect somewhere else.

---

## Unstable / Not supported yet

  - Ambiguous grammars will have *some* output chosen. That derivation might be quite bad. Spurious derivations should not happen (?) but I cannot test everything and making parser-generators is quite new to me. 
  
- **Nullable Rules**: Rules with empty RHS that can accept an empty string. Support for these is baked in the recognizer and the parsing, but has not been tested yet and no syntax for those is provided at the moment. This will allow optional members in a sentence.

- Raw placeholders outside disjunctions. Hard to specify right now what they should do exactly : `Foo : Bar Baz` 
  
`MultiEffect : "{effects : Effect*}"` Could be a thing, yielding an Array, maybe with helpers for delimiters

raw placeholders for syntax :

`Effect : "{Effect}."` (allows ending sentences with . in effect descriptions)



