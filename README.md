# Dokearley

Earley parser and grammar parser for `Doke`. Allows game-devs to write DSLs quickly to edit their data in natural-like language.

Includes `dokedef`, a language to write those DSLs, and a parser to parse those DSLs, returning a Resource-like rust enum.

## Usage

```rust
use dokearley::Dokearley;
// An input dokedef file.
let grammar = r#"
ItemEffect: "deal {amount:Int} damage" -> Damage
ItemEffect: "heal for {amount:Int}" -> Heal
ItemEffect: "apply {status:String}" -> ApplyStatus
Target: "self" -> Target { kind: "self" }
Target: "an ally" -> Target { kind: "ally" }
Target: "all enemies" -> Target { kind: "enemies" }
"#;
// Build the parser from the dokedef.
let parser = Dokearley::from_dokedef(grammar).expect("invalid grammar");
// Get a result from an input statement, and a <Start> Nonterminal, which tries to parse the input as a <Start>
let result = parser.parse("heal for 7", "ItemEffect").unwrap();
dbg!(result);
// 
// Resource {
//   typ: "TargetedEffect", 
//   fields: {
//      "target": Resource { typ: "Target", fields: {"kind": String("self")}}, 
//      "effect": Resource { typ: "Heal", fields: {"amount": Integer(7)} }} 
//  }
```
## New features

You can now accept childs in the RHS. This marks fields that will demand Doke to parse
the child Doke statements into the given field as an array. It will try to parse children
as the given Non-Terminal, and collect all matching children into an array

```
Action: "Do the following" -> Action { components <* ActionComponent  }
ActionComponent : ItemEffect | Action
```

You can also accept the first child that matches the non-terminal into a field 
```
Action: "Do this single thing :" -> Action { component < ActionComponent  }
```

These can be combined to, for example, allow some actions to accept only a single damage effect,
and any Components. This aproach would produce some "undefined behaviour" if a child matches two different non-terminals.
This is left up to DokeParser to specify.

All this will allow the dokeparser to parse component-like things like this
(See the documentation of doke-parser itself for more information about this)

```
Do the following:
 - deal 75 damage to the target
 - heal yourself for 7
```

With some clever design (both in the language and in-engine),
components can give your language this sort of functionality in a few simple rules :

*Some kind of vampiric spell*
```
Select two targets :
  - Deal 5 damage in a 3-cross around the first target
  - Heal the second target for half of all damage inflicted.
```

*Auras, on hit effects....*
```
Every turn for 6 turns :
  - Deal 10 damage around yourself. On hit :
    - Apply 2 poison
    - Remove 2 poison for yourself
```

*If statements*
```
If your HP is above 50% :
  - loose half of your HP
  - deal 20 damage for each HP lost.
```

*Reaction passives*
```
Every time you loose more than 20 HP:
  - Heal for 10 HP
```

Implementing this in engine is a matter of cleverly using contexts that the components pass around to the next one.

Parsing this is just some simple rules with the `<` or `<*` children captures.


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
### Dictionaries

You can use dict instead of resources too, they behave the same way without a type name,
and map to a dictionary type in the game engine.

```
Target: "self" -> { kind: "self" }
Target: "an ally" -> { kind: "ally" }
Target: "an enemy" -> Target { kind: "enemy" }

Target: "{name : String}" -> {}
```

As seen in that last example, dictionaries also capture placeholders into themselves.

# Notes

*Strings are written as litterals, this is not exactly superb, i will add config to allow using some other syntax that looks pretty in markdown, like **poison**. *

The String type could also be split into multiple "aliases" and you could specify `{stat : BoldString}` `{stat : WikiLink}` allowing **poison** or [[poison]] (which is good with editors like Obsidian, as it shows you what undocumented status effect you might have used and allows you to link to their definition in the wiki right then and there)
In this case, i would allow `String` to take any of these. This needs more specification though.



It should be said that for ambiguous grammars, this parser is O(n^3). 
Though in practice, this is meant for small statements of bounded size and therefore would still be linear.

Earley is also quite efficient for this use case.
Because this is meant for human-readable languages, there are a lot of terminals that structure the data, often way more than placeholders/non-terminals, and Earley parsers happily chug along the terminals without creating an explosion of items.

Supports nullable rules (at the moment they are tested but don't have a definite syntax in the dokedef language)

Gives a best attempt at a good error message, showing earley items that could have been meant, as we don't know the language in advance.

As of now, in the user language, 102020 is always an int, and 65.5 a float, and they are treated as tokens. You can use 2. for floats that look like ints, but I admit it's not too great. I'll be working on this when I get to using the language.

You will find that there are no bools. I will also add them, though probably only in output specs, as i'm sure anyone's ideal human readable DSL doesn't look like `Do something : true`, and more like `Do something`

Currently case-sensitive. I will maybe add a symbol group syntax like `[aA]` to allow for quick handling of that, or add an optional lowercase matching.

---

## Emoji Support

Dokearley grammars aren’t limited to plain text – you can also define rules using **emoji** or any UTF-8 (not too weird though).
This allows for more compact and friendly syntax in some cases.

For example, you could add emoji aliases for some things:

```
(...)
ItemEffect: "🔥{amount:Int}" -> FireDamage
ItemEffect: "💖{amount:Int}" -> Heal
ItemEffect: "💀" -> ApplyStatus { status: "death" }
ItemEffect: "+{amount:Int}🛡️" -> Buff { stat: "defense" }

Target: "🙂" -> Target { kind: "self" }
Target: "👹" -> Target { kind: "enemy" }
Target: "🤝" -> Target { kind: "ally" }
```

And then statements like:

```
👹 🔥12
🙂 💖7
+5🛡️
```

Would parse into resources like:

```
TargetedEffect {
  target: Target { kind: "enemy" },
  effect: FireDamage { amount: 12 }
}

TargetedEffect {
  target: Target { kind: "self" },
  effect: Heal { amount: 7 }
}

Buff { stat: "defense", amount: 5 }
```

# Dokedef File Format

This project provides a **domain-specific grammar format** for defining game mechanics, actions, and effects. Unlike general-purpose language grammars, this format is **tailored for game-making**, focusing on being simple to use for this use case.

It is designed to generate Resources or Dictionaries from structured text inputs.

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
        
    - A composite Resource, possibly with fixed fields: `PartiallyBuiltType{foo : "bar", bar : 2501}`
        
    - A dictionary, possibly with fixed fields  `{some : "thing"}`

You can do `{foo : bar}` to tie the value of bar in the placeholders, to foo. Note that if `bar = "baz"` is captured, this will produce a resource/dict
with both `foo = "baz"` and `bar : "baz"`

As resources will usually be pruned of unwanted fields, this is okay, 
but it gives some trouble if warning for mis-named fields that
have a typo and mismatch between in-engine and in-grammar

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

### 3. Disjonction

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



