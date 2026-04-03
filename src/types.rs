use std::collections::HashMap;

pub struct Context {
    // The original source code
    pub source: Vec<u8>,
    // A String<->Symbol map that lets us refer to strings with usize
    pub interner: StringStore,
    pub symbols: Vec<SymbolData>,
}

impl Context {
    pub fn new(src: Vec<u8>) -> Self {
        Self {
            source: src,
            interner: StringStore::new(),
            symbols: vec![],
        }
    }

    pub fn to_symbol(&mut self, name: &str) -> Symbol {
        // If we've already added the string to the store, get its ID
        if let Some(id) = self.interner.mapping.get(name) {
            return *id;
        } else {
            // Else add it
            let next_id = Symbol(self.interner.store.len());
            self.interner.mapping.insert(name.to_string(), next_id);
            self.interner.store.push(name.to_string());
            next_id
        }
    }

    pub fn lookup_symbol(&self, id: Symbol) -> &str {
        &self.interner.store[id.0]
    }

    pub fn declare_local(&mut self, name: Symbol, ty: Type) -> usize {
        let id = self.symbols.len();
        self.symbols.push(SymbolData {
            id,
            name,
            ty,
            addressed: true,
        });
        id
    }

    pub fn declare_function(&mut self, name: Symbol, ty: Type) -> usize {
        let id = self.symbols.len();
        self.symbols.push(SymbolData {
            id,
            name,
            ty,
            addressed: true,
        });
        id
    }

    pub fn declare_global(&mut self, name: Symbol, ty: Type) -> usize {
        let id = self.symbols.len();
        self.symbols.push(SymbolData {
            id,
            name,
            ty,
            addressed: true,
        });
        id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symbol(pub usize);

pub struct SymbolData {
    pub id: usize,
    pub name: Symbol,
    pub ty: Type,
    pub addressed: bool,
}

pub struct StringStore {
    mapping: HashMap<String, Symbol>,
    store: Vec<String>,
}

impl StringStore {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            store: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Token {
    pub kind: TKind,
    pub span: Span,
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum TKind {
    #[default]
    Eof,
    Int(u64),
    Bool(bool),
    Str(Symbol),
    Ident(Symbol),

    // Declarator keywords
    Let,
    Fn,
    Struct,
    Global,

    // Control flow keywords
    While,
    Continue,
    Break,
    If,
    Else,
    Return,

    // Delimiters
    LParen, // (
    RParen, // )
    LCurly, // {
    RCurly, // }
    LBrack, // [
    RBrack, // ]

    // Separators
    Comma, // ,
    Dot,   // .
    Colon, // :
    Semi,  // ;
    Arrow, // ->

    // Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    And,     // &
    Or,      // |
    Caret,   // ^
    Bang,    // !
    Eq,      // =
    AndAnd,  // &&
    OrOr,    // ||

    // Relationals
    EqEq,   // ==
    BangEq, // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=
}

type SymbolId = usize;

/// Represents top-level "things", which includes:
/// - function definitions
/// - type definitions
/// - global variables
#[derive(Debug, Clone)]
pub enum OKind {
    Fn {
        name: SymbolId,
        returns: Type,
        args: Vec<SymbolId>,
        body: Stmt,
    },
    Global {
        lhs: SymbolId,
        rhs: Expr,
    },
    Struct {
        name: SymbolId,
        fields: Vec<SymbolId>,
    },
}

#[derive(Debug, Clone)]
pub struct Object {
    pub kind: OKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TyKind {
    // After the parsing stage, MOST nodes will have an Infer type
    // After the resolve stage, ALL nodes should have a concrete type
    Infer,
    Unresolved(Symbol),

    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    Bool,
    Void,

    Function {
        args: Vec<TyKind>,
        returns: Box<TyKind>,
    },
    Pointer(Box<TyKind>),
    // Struct(Vec<Field>),
    // Custom(&'src, Box<Type>)
}

impl std::fmt::Display for TyKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TyKind::Infer => "{unknown}".to_string(),
            TyKind::Unresolved(_) => "{unknown custom type}".to_string(),
            TyKind::U8 => "u8".to_string(),
            TyKind::U16 => "u16".to_string(),
            TyKind::U32 => "u32".to_string(),
            TyKind::U64 => "u64".to_string(),
            TyKind::I8 => "i8".to_string(),
            TyKind::I16 => "i16".to_string(),
            TyKind::I32 => "i32".to_string(),
            TyKind::I64 => "i64".to_string(),
            TyKind::Bool => "bool".to_string(),
            TyKind::Void => "void".to_string(),
            TyKind::Function { args, returns } => {
                let args = args
                    .iter()
                    .map(|arg| format!("{arg}"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("fn({args}) -> {returns}")
            }
            TyKind::Pointer(ty) => format!("*{}", ty),
        };
        f.write_str(&s)
    }
}

impl TyKind {
    pub fn width(&self) -> u8 {
        use TyKind::*;
        match self {
            U8 | I8 => 8,
            U16 | I16 => 16,
            U32 | I32 | Bool => 32,
            U64 | I64 | Pointer(_) => 64,
            rest @ (Unresolved(_) | Infer | Void | Function { .. }) => {
                panic!("{rest} does not have a width")
            }
        }
    }

    pub fn signed(&self) -> bool {
        use TyKind::*;
        match self {
            U8 | U16 | U32 | U64 | Bool | Pointer(_) => false,
            I8 | I16 | I32 | I64 => true,
            rest @ (Unresolved(_) | Infer | Void | Function { .. }) => {
                panic!("{rest} cannot be a signed/unsigned")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: SKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: SKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum SKind {
    Let {
        lhs: SymbolId,
        rhs: Expr,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    Continue,
    Break,
    If {
        cond: Expr,
        then_: Box<Stmt>,
        else_: Box<Stmt>,
    },
    Return(Expr),
    Block(Vec<Stmt>),
    Expr(Expr),
}

#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
    pub row: usize,
    pub col: usize,
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("lo", &self.lo)
            .field("hi", &self.hi)
            // Omit `internal_secret` entirely
            .finish()
    }
}

impl Span {
    fn print_span(&self, src: &str) {
        let line_start = src[..self.lo]
            .chars()
            .rev()
            .position(|b| b == '\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let line_end = src[self.lo..]
            .chars()
            .position(|b| b == '\n')
            .map(|pos| self.lo + pos)
            .unwrap_or(src.len());

        let line_text = &src[line_start..line_end];

        let before_span_count = self.lo.saturating_sub(line_start);

        let is_multiline = self.hi > line_end;
        let effective_hi = if is_multiline { line_end } else { self.hi };

        let caret_count = effective_hi.saturating_sub(self.lo).max(1);

        let spaces = " ".repeat(before_span_count);
        let mut carets = "^".repeat(caret_count);

        if is_multiline {
            carets.push_str("...");
        }

        println!(
            "\n\n{}:{}:\n{}\n{}{}",
            self.row + 1,
            self.col,
            line_text,
            spaces,
            carets
        )
    }
}

impl Span {
    pub fn merge(self, other: Self) -> Self {
        let smaller = if self.lo <= other.lo { &self } else { &other };
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            row: smaller.row,
            col: smaller.col,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: EKind,
    pub ty: TyKind,
    pub span: Span,
}

impl std::fmt::Display for EKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EKind::Unresolved(symbol) => "unresolved variable",
            EKind::Variable(_) => "variable",
            EKind::Int(_) => "int",
            EKind::Bool(_) => "bool",
            EKind::Nothing => "nothing",
            EKind::Str(_) => "string",
            EKind::Call { .. } => "function call",
            EKind::Unary { .. } => "unary operation",
            EKind::Bin { .. } => "binary operation",
            EKind::FieldAccess { .. } => "field access",
            EKind::Index { .. } => "array index",
        };
        f.write_str(s)
    }
}

impl Expr {
    pub fn new(kind: EKind, ty: TyKind, span: Span) -> Self {
        Self { kind, ty, span }
    }
}

#[derive(Debug, Clone)]
pub enum EKind {
    Unresolved(Symbol), // This should resolve to a Variable after resolver
    Variable(SymbolId),

    Int(u64),
    Bool(bool),
    Nothing,
    Str(Symbol),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnOp,
        rhs: Box<Expr>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    FieldAccess {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Index {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Negate,
    Not,
    AddrOf,
    Deref,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    LogOr,
    LogAnd,
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
    Ne,
}
