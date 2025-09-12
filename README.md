# Dokearley

Earley parser and grammar parser for `Doke`. Will allow game-devs to write DSLs quickly to edit their data in natural-like language.

Todo : Actual Earley parser, with the nullable fix. 
We don't care about O(n^3) worst-case because a previous parser cuts everything into small statements,
( an N input is divided in O(n) statements of bounded size, so total time is O(n) * Bound^3)

Earley allows for a run-time parser-generation and parses any context-free grammar.

Done : grammar-file parsing and syntax highlighting, for basic cases (no disjunction support yet, and a bug for optional RHS)


## Dokedef specification

```mathematica
Number Literal Examples
-----------------------

Decimal Integer (i64)     : -1627326
Binary Integer (i64)      : +0b1010101010
Octal Integer (i64)       : 0o070707
Hexadecimal Integer (i64) : -0x021F0
Floating-Point (f64)      : 1.54e-10 or -120. , etc...
```

Example grammar file ( Format tailored for game-making, not general-purpose lang-dev)

```
Effect : "Deal {dmg : int}" => DamageEffect
Effect : "{first : Effect}, then : {then : Effect}" => EffectThenEffect

NonTerminal : "pattern with {place : holders}" -> Type
NonTerminal : "pattern with {place : holders}" -> PartiallyBuiltType{foo : "bar", bar : 2501}
TerminalAndTypeAreTheSame : "No RHS needed to {put : Things} in the {type : Type}"

Hey : "ho"; Ho : "separators are optional";

# Comments are not yet supported, but will be like GDscript does it

YouCan : "use multiple lines" => In {
  places : "where it's not too weird",
  hopefully : "the syntax highlighter and the (future) linter will help you"
}


```
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
        
    - Propagation of fields from placeholders: `{}` giving all the fields to the placeholder's rule.
        

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



