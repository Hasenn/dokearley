use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}



impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{} - {}]", self.start, self.end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Symbol<'gram> {
    Terminal(&'gram str),
    Placeholder { name: &'gram str, typ: &'gram str },
    NonTerminal(&'gram str),
}

#[derive(Debug, Clone)]
pub struct Production<'gram> {
    pub lhs: &'gram str,
    pub rhs: Vec<Symbol<'gram>>,
}

#[derive(Debug, Clone)]
pub struct Grammar<'gram> {
    pub productions: Vec<Production<'gram>>,
    pub start: &'gram str,
}

impl<'gram> Grammar<'gram> {
    pub fn prods_for(&'_ self, name: &str) -> Vec<(usize, &Production<'gram>)> {
        self.productions
            .iter()
            .enumerate()
            .filter(|(_, p)| p.lhs == name)
            .map(|(i, p)| (i, p))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemKey {
    pub prod_id: usize,
    pub dot: usize,
    pub start: usize,
}

#[derive(Debug, Clone)]
pub struct BackPtr {
    pub child: ItemKey,
    pub at: usize,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub key: ItemKey,
    pub bps: Vec<Vec<BackPtr>>,
}

impl Item {
    pub fn new(prod_id: usize, dot: usize, start: usize) -> Self {
        Item {
            key: ItemKey {
                prod_id,
                dot,
                start,
            },
            bps: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Word,
    Int,
    Float,
    StringLit,
    Punct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token<'inp> {
    pub kind: TokenKind,
    pub text: &'inp str,
    pub span: Span,
}

pub fn tokenize(input: &str) -> Vec<Token<'_>> {
    let mut tokens = vec![];
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' {
            let start = i;
            i += 1;
            let str_start = i;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            let str_end = i;
            let text: &str = &input[str_start..str_end];
            tokens.push(Token {
                kind: TokenKind::StringLit,
                text,
                span: Span::new(start, i + 1),
            });
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '"' {
            i += 1;
        }
        let raw: &str = &input[start..i];
        if raw.parse::<i64>().is_ok() {
            tokens.push(Token {
                kind: TokenKind::Int,
                text: raw,
                span: Span::new(start, i),
            });
            continue;
        }
        if raw.parse::<f64>().is_ok() {
            tokens.push(Token {
                kind: TokenKind::Float,
                text: raw,
                span: Span::new(start, i),
            });
            continue;
        }
        if raw.chars().all(|c| !c.is_alphanumeric()) {
            tokens.push(Token {
                kind: TokenKind::Punct,
                text: raw,
                span: Span::new(start, i),
            });
            continue;
        }
        tokens.push(Token {
            kind: TokenKind::Word,
            text: raw,
            span: Span::new(start, i),
        });
    }
    tokens
}

fn is_builtin(typ: &str, tok: &Token<'_>) -> bool {
    match typ.to_ascii_lowercase().as_str() {
        "int" => tok.kind == TokenKind::Int,
        "float" => tok.kind == TokenKind::Float,
        "string" | "str" => tok.kind == TokenKind::StringLit,
        "word" => tok.kind == TokenKind::Word,
        _ => false,
    }
}

pub struct Chart<'gram, 'inp> {
    pub sets: Vec<HashMap<ItemKey, Item>>,
    pub tokens: Vec<Token<'inp>>,
    pub grammar: &'gram Grammar<'gram>,
}

impl<'gram, 'inp> Chart<'gram, 'inp> {
    pub fn new(grammar: &'gram Grammar<'gram>, tokens: Vec<Token<'inp>>) -> Self {
        let n = tokens.len();
        let mut sets = Vec::with_capacity(n + 1);
        for _ in 0..=n {
            sets.push(HashMap::new());
        }
        Self {
            sets,
            tokens,
            grammar,
        }
    }

    pub fn add_item(&mut self, pos: usize, item: Item) -> bool {
        let key = item.key.clone();
        if let Some(existing) = self.sets[pos].get_mut(&key) {
            existing.bps.extend(item.bps);
            false
        } else {
            self.sets[pos].insert(key, item);
            true
        }
    }

    pub fn recognize(&mut self) {
        for (pid, _) in self.grammar.prods_for(self.grammar.start) {
            let it = Item::new(pid, 0, 0);
            self.add_item(0, it);
        }

        let n = self.tokens.len();
        for pos in 0..=n {
            let mut changed = true;
            while changed {
                changed = false;
                let keys: Vec<ItemKey> = self.sets[pos].keys().cloned().collect();
                for key in keys {
                    let item = match self.sets[pos].get(&key) {
                        Some(it) => it.clone(),
                        None => continue,
                    };
                    let prod = &self.grammar.productions[item.key.prod_id];
                    if item.key.dot < prod.rhs.len() {
                        let next = &prod.rhs[item.key.dot];
                        match next {
                            Symbol::NonTerminal(nt) => {
                                for (pid, _) in self.grammar.prods_for(nt) {
                                    let new_it = Item::new(pid, 0, pos);
                                    if self.add_item(pos, new_it) {
                                        changed = true;
                                    }
                                }
                            }
                            Symbol::Terminal(lit) => {
                                if pos < self.tokens.len() && self.tokens[pos].text == *lit {
                                    let mut new_it = Item::new(
                                        item.key.prod_id,
                                        item.key.dot + 1,
                                        item.key.start,
                                    );
                                    let child = ItemKey {
                                        prod_id: usize::MAX,
                                        dot: 0,
                                        start: pos,
                                    };
                                    let bp = BackPtr {
                                        child,
                                        at: item.key.dot,
                                    };
                                    new_it.bps.push(vec![bp]);
                                    if self.add_item(pos + 1, new_it) {
                                        changed = true;
                                    }
                                }
                            }
                            Symbol::Placeholder { name: _, typ } => {
                                if pos < self.tokens.len() && is_builtin(typ, &self.tokens[pos]) {
                                    let mut new_it = Item::new(
                                        item.key.prod_id,
                                        item.key.dot + 1,
                                        item.key.start,
                                    );
                                    let child = ItemKey {
                                        prod_id: usize::MAX - 1,
                                        dot: 0,
                                        start: pos,
                                    };
                                    let bp = BackPtr {
                                        child,
                                        at: item.key.dot,
                                    };
                                    new_it.bps.push(vec![bp]);
                                    if self.add_item(pos + 1, new_it) {
                                        changed = true;
                                    }
                                } else {
                                    for (pid, _) in self.grammar.prods_for(typ) {
                                        let new_it = Item::new(pid, 0, pos);
                                        if self.add_item(pos, new_it) {
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        let lhs = prod.lhs;
                        let waiting_keys: Vec<ItemKey> = self.sets[item.key.start]
                            .keys()
                            .filter(|k| {
                                let p = &self.grammar.productions[k.prod_id];
                                if k.dot < p.rhs.len() {
                                    match &p.rhs[k.dot] {
                                        Symbol::NonTerminal(name) => name == &lhs,
                                        Symbol::Placeholder { name: _, typ } => **typ == *lhs,
                                        _ => false,
                                    }
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect();

                        for wk in waiting_keys {
                            let mut new_it = Item::new(wk.prod_id, wk.dot + 1, wk.start);
                            let bp = BackPtr {
                                child: item.key.clone(),
                                at: wk.dot,
                            };
                            new_it.bps.push(vec![bp]);
                            if self.add_item(pos, new_it) {
                                changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn accepted(&self) -> bool {
        let n = self.tokens.len();
        self.sets[n].values().any(|it| {
            it.key.start == 0
                && it.key.dot == self.grammar.productions[it.key.prod_id].rhs.len()
                && self.grammar.productions[it.key.prod_id].lhs == self.grammar.start
        })
    }
}
