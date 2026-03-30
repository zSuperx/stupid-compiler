use crate::types::*;

pub struct Parser<'src> {
    src: &'src [Kind<'src>],
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src [Kind<'src>]) -> Self {
        Self { src }
    }

    fn is_next(&self, kind: Kind<'src>) -> bool {
        self.peek() == kind
    }

    fn expect(&mut self, kind: Kind<'src>) {
        let next = self.consume();
        if next != kind {
            panic!("Expected {kind}, found {next}");
        }
    }

    fn expect_ident(&mut self) -> &'src str {
        let next = self.consume();
        match next {
            Kind::Ident(s) => s,
            _ => panic!("Expected identifier, found {next}"),
        }
    }

    fn expect_strlit(&mut self) -> &'src [u8] {
        let next = self.consume();
        match next {
            Kind::Str(s) => s,
            _ => panic!("Expected string literal, found {next}"),
        }
    }

    fn expect_intlit(&mut self) -> u64 {
        let next = self.consume();
        match next {
            Kind::Int(s) => s,
            _ => panic!("Expected int literal, found {next}"),
        }
    }

    fn consume(&mut self) -> Kind<'src> {
        let first = self.src.first().unwrap_or(&Kind::Eof);
        if !self.src.is_empty() {
            self.src = &self.src[1..];
        }
        *first
    }

    fn peek(&self) -> Kind<'src> {
        *self.src.first().unwrap_or(&Kind::Eof)
    }

    pub fn parse_program(&mut self) -> Vec<Object<'src, RawType<'src>>> {
        let mut objs = vec![];
        while !self.is_next(Kind::Eof) {
            match self.peek() {
                Kind::Fn => objs.push(self.parse_fn()),
                Kind::Struct => objs.push(self.parse_struct()),
                Kind::Global => objs.push(self.parse_global()),
                x => panic!("Expected function definition, found {x}"),
            }
        }
        objs
    }

    fn parse_struct(&mut self) -> Object<'src, RawType<'src>> {
        todo!()
    }

    fn parse_global(&mut self) -> Object<'src, RawType<'src>> {
        self.expect(Kind::Global);
        let name = self.expect_ident();
        self.expect(Kind::Colon);
        let ty = self.parse_type();
        self.expect(Kind::Eq);
        let rhs = self.parse_expr();
        Object::Global(Symbol { name, ty })
    }

    fn parse_fn(&mut self) -> Object<'src, RawType<'src>> {
        self.expect(Kind::Fn);
        let name = self.expect_ident();
        self.expect(Kind::LParen);

        let mut args = vec![];
        if !self.is_next(Kind::RParen) {
            let name = self.expect_ident();
            self.expect(Kind::Colon);
            let ty = self.parse_type();
            let var = Symbol { name, ty };
            args.push(var);
        }

        loop {
            let cur = self.peek();
            if matches!(cur, Kind::RParen) {
                self.consume();
                break;
            }

            self.expect(Kind::Comma);

            let name = self.expect_ident();
            self.expect(Kind::Colon);
            let ty = self.parse_type();
            let var = Symbol { name, ty };
            args.push(var);
        }

        let returns = if self.is_next(Kind::Arrow) {
            self.expect(Kind::Arrow);
            self.parse_type()
        } else {
            RawType::Base("void")
        };

        let body = self.parse_block();

        Object::Fn {
            name,
            body,
            args,
            returns,
        }
    }

    fn parse_type(&mut self) -> RawType<'src> {
        let mut ty = match self.peek() {
            Kind::Star => {
                self.consume();
                return RawType::Pointer(Box::new(self.parse_type()));
            }
            Kind::Ident(name) => RawType::Base(name),
            x => panic!("Expected type, found {x}"),
        };
        self.consume();
        ty
    }

    fn parse_prefix(&mut self) -> ExprKind<'src, RawType<'src>> {
        match self.consume() {
            Kind::Ident(s) => ExprKind::Symbol(Symbol {
                name: s,
                ty: RawType::Unknown,
            }),
            Kind::Bool(x) => ExprKind::Bool(x),
            Kind::Int(x) => ExprKind::Int(x),
            Kind::Str(s) => ExprKind::Str(s),
            op @ (Kind::Minus | Kind::Bang | Kind::And | Kind::Star) => ExprKind::Unary {
                op: match op {
                    Kind::Minus => UnOp::Negate,
                    Kind::Bang => UnOp::Not,
                    Kind::And => UnOp::AddrOf,
                    Kind::Star => UnOp::Deref,
                    _ => unreachable!(),
                },
                rhs: Box::new(expr(
                    self.parse_expr_kind(prefix_power(op).expect("Invalid operator")),
                )),
            },
            Kind::LParen => {
                let inner = self.parse_expr_kind(0.0);
                self.expect(Kind::RParen);
                inner
            }
            x => panic!("Expected start of expression, found \"{x}\""),
        }
    }

    fn parse_infix(
        &mut self,
        lhs: ExprKind<'src, RawType<'src>>,
        op: Kind<'src>,
        op_power: f32,
    ) -> ExprKind<'src, RawType<'src>> {
        match op {
            Kind::LParen => {
                let mut args = vec![];
                while !self.is_next(Kind::RParen) {
                    if !args.is_empty() {
                        self.expect(Kind::Comma);
                    }
                    args.push(expr(self.parse_expr_kind(0.0)));
                }
                self.expect(Kind::RParen);
                ExprKind::Call {
                    callee: Box::new(expr(lhs)),
                    args,
                }
            }
            op @ (Kind::Eq
            | Kind::Plus
            | Kind::Minus
            | Kind::Star
            | Kind::Slash
            | Kind::AndAnd
            | Kind::OrOr
            | Kind::EqEq
            | Kind::BangEq
            | Kind::Lt
            | Kind::LtEq
            | Kind::Gt
            | Kind::GtEq) => {
                let rhs = self.parse_expr_kind(op_power);
                let binop = match op {
                    Kind::Eq => BinOp::Assign,
                    Kind::Plus => BinOp::Add,
                    Kind::Minus => BinOp::Sub,
                    Kind::Star => BinOp::Mul,
                    Kind::Slash => BinOp::Div,
                    Kind::AndAnd => BinOp::LogAnd,
                    Kind::OrOr => BinOp::LogOr,
                    Kind::EqEq => BinOp::Eq,
                    Kind::BangEq => BinOp::Ne,
                    Kind::Lt => BinOp::Lt,
                    Kind::LtEq => BinOp::Le,
                    Kind::Gt => BinOp::Gt,
                    Kind::GtEq => BinOp::Ge,
                    _ => unreachable!(),
                };
                ExprKind::Bin {
                    op: binop,
                    lhs: Box::new(expr(lhs)),
                    rhs: Box::new(expr(rhs)),
                }
            }
            Kind::LBrack => {
                let index = self.parse_expr_kind(op_power);
                self.expect(Kind::RBrack);
                ExprKind::Index {
                    lhs: Box::new(expr(lhs)),
                    rhs: Box::new(expr(index)),
                }
            }
            Kind::Dot => {
                let field = self.parse_expr_kind(op_power);
                if !matches!(field, ExprKind::Symbol(_)) {
                    panic!("Fields can only be accessed with an identifier, not a {field:?}");
                }
                ExprKind::FieldAccess {
                    lhs: Box::new(expr(lhs)),
                    rhs: Box::new(expr(field)),
                }
            }
            x => panic!("Expected infix operator, found {x}"),
        }
    }

    // Pratt parsing!
    fn parse_expr_kind(&mut self, min_power: f32) -> ExprKind<'src, RawType<'src>> {
        let mut lhs = self.parse_prefix();

        loop {
            let op = self.peek();

            let Some(op_power) = infix_power(op) else {
                break;
            };

            if op_power.0 < min_power {
                break;
            }

            self.consume();

            lhs = self.parse_infix(lhs, op, op_power.1);
        }

        lhs
    }

    fn parse_expr(&mut self) -> Expr<'src, RawType<'src>> {
        Expr {
            kind: self.parse_expr_kind(0.0),
            ty: RawType::Unknown,
        }
    }

    fn parse_block(&mut self) -> Stmt<'src, RawType<'src>> {
        self.expect(Kind::LCurly);
        let mut stmts = vec![];
        while !self.is_next(Kind::RCurly) {
            let stmt = self.parse_stmt();
            if matches!(stmt, Stmt::Block(ref inner) if inner.is_empty()) {
                // Skip nested empty {} blocks, they're useless.
                continue;
            }
            stmts.push(stmt);
        }
        self.expect(Kind::RCurly);
        Stmt::Block(stmts)
    }

    fn parse_stmt(&mut self) -> Stmt<'src, RawType<'src>> {
        match self.peek() {
            Kind::Let => {
                let mut ty = RawType::Unknown;
                self.expect(Kind::Let);
                let name = self.expect_ident();
                if self.is_next(Kind::Colon) {
                    self.expect(Kind::Colon);
                    ty = self.parse_type();
                }
                let lhs = Symbol { name, ty };
                self.expect(Kind::Eq);

                let rhs = self.parse_expr();
                self.expect(Kind::Semi);
                Stmt::Let { lhs, rhs }
            }
            Kind::While => {
                self.expect(Kind::While);
                let cond = self.parse_expr();
                let body = self.parse_block();
                Stmt::While {
                    cond,
                    body: Box::new(body),
                }
            }
            Kind::Continue => {
                self.expect(Kind::Continue);
                Stmt::Continue
            }
            Kind::Break => {
                self.expect(Kind::Break);
                Stmt::Break
            }
            Kind::If => {
                self.expect(Kind::If);
                let cond = self.parse_expr();
                let then_ = self.parse_block();

                let else_ = if self.is_next(Kind::Else) {
                    self.expect(Kind::Else);
                    if self.is_next(Kind::If) {
                        // If is a special case since it's allowed after an else
                        self.parse_stmt()
                    } else {
                        // Otherwise, else must be followed by a block
                        self.parse_block()
                    }
                } else {
                    Stmt::Block(vec![])
                };
                Stmt::If {
                    cond,
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            }
            Kind::Return => {
                self.expect(Kind::Return);
                let retval = if !self.is_next(Kind::Semi) {
                    self.parse_expr()
                } else {
                    expr(ExprKind::Nothing)
                };
                self.expect(Kind::Semi);

                Stmt::Return(retval)
            }
            _ => {
                let expr = self.parse_expr();
                self.expect(Kind::Semi);
                Stmt::Expr(expr)
            }
        }
    }
}

fn prefix_power(kind: Kind) -> Option<f32> {
    let power = match kind {
        Kind::Bang | Kind::Minus | Kind::Star | Kind::And => 9.0,
        _ => return None, // Not an prefix operator
    };
    Some(power)
}

fn infix_power(kind: Kind) -> Option<(f32, f32)> {
    let power = match kind {
        // Assignment: Right-Associative
        // We use a lower Left power so it "gives up" easily,
        // but a higher Right power to "grab" everything to the right.
        Kind::Eq => (2.1, 2.0),

        Kind::AndAnd | Kind::OrOr => (4.1, 4.0),

        Kind::EqEq | Kind::BangEq => (5.0, 5.1),
        Kind::Lt | Kind::Gt => (6.0, 6.1),
        Kind::LtEq | Kind::GtEq => (6.0, 6.1),

        Kind::Plus | Kind::Minus => (7.0, 7.1),

        Kind::Star | Kind::Slash => (8.0, 8.1),

        // Postfix: Highest priority
        // (Call, Indexing, Member Access)
        Kind::LParen | Kind::LBrack | Kind::Dot => (10.0, 10.1),

        _ => return None, // Not an infix operator
    };
    Some(power)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tokenizer::Lexer, types::*};
    #[test]
    fn test_parse_type() {
        let src = b"u32 *billy";
        let tokens: Vec<Kind> = Lexer::new(src).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let x = parser.parse_type();
        dbg!(x);
    }

    #[test]
    fn test_parse_expr() {
        let input = b"x = y = hi";
        println!("input = {}", str::from_utf8(input).unwrap());
        let tokens: Vec<Kind> = Lexer::new(input).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let output = parser.parse_expr();
        dbg!(output);
    }

    #[test]
    fn test_parse_stmt() {
        let input = b"if true";
        println!("input = {}", str::from_utf8(input).unwrap());
        let tokens: Vec<Kind> = Lexer::new(input).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let output = parser.parse_expr();
        dbg!(output);
    }

    #[test]
    fn test_parse_fn() {
        let src = br#"
fn bob(arg1: *u32, arg2: u8) -> u32 {
    foo.bar();
}
"#;
        let tokens: Vec<Kind> = Lexer::new(src).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let parsed = parser.parse_fn();
        println!("Source code: {}", str::from_utf8(src).unwrap());
        dbg!(parsed);
    }

    #[test]
    fn test_parse_program() {
        let src = std::fs::read("tests/test.stupid").unwrap();
        let tokens: Vec<Kind> = Lexer::new(&src).map(|t| t.kind).collect();
        let mut parser = Parser::new(&tokens);
        let parsed = parser.parse_program();
        println!("Source code:\n{}", str::from_utf8(&src).unwrap());
        dbg!(parsed);
    }
}

/// Helper to assign an unknown type to an ExprKind
fn expr<'src>(kind: ExprKind<'src, RawType<'src>>) -> Expr<'src, RawType<'src>> {
    Expr {
        kind,
        ty: RawType::Unknown,
    }
}
