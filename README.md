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

# Feature to do at some point

```
Effect : EffectComponent (that maps nicely to inheritance, should one decide to use some)
```
