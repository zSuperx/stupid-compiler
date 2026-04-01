use crate::types::*;

pub struct Parser<'src> {
    tokens: &'src [Token<'src>],
    src: &'src [u8],
    last_span: Span<'src>,
}

impl<'src> Parser<'src> {
    pub fn new(tokens: &'src [Token<'src>], src: &'src [u8]) -> Self {
        Self {
            tokens,
            src,
            last_span: Span {
                src,
                ..Default::default()
            },
        }
    }

    fn make_expr(
        &mut self,
        kind: EKind<'src, RawType<'src>>,
        span: Span<'src>,
    ) -> Expr<'src, RawType<'src>> {
        Expr {
            kind,
            ty: RawType::Unknown,
            span,
        }
    }

    fn make_stmt(
        &mut self,
        kind: SKind<'src, RawType<'src>>,
        span: Span<'src>,
    ) -> Stmt<'src, RawType<'src>> {
        Stmt { kind, span }
    }

    /// Peeks the next token and checks its kind
    fn is_next(&self, kind: TKind<'src>) -> bool {
        self.peek().kind == kind
    }

    /// Peeks the next token. This does not consume.
    fn peek(&self) -> Token<'src> {
        self.tokens.first().copied().unwrap_or_default()
    }

    fn consume(&mut self) -> Token<'src> {
        let first = self.tokens.first().copied().unwrap_or_default();
        if !self.tokens.is_empty() {
            self.tokens = &self.tokens[1..];
        }
        self.last_span = first.span;
        first
    }

    fn expect(&mut self, kind: TKind<'src>) {
        let next = self.consume();
        if next.kind != kind {
            panic!("Expected {kind}, found {}\n\n{}", next.kind, next.span);
        }
    }

    fn expect_ident(&mut self) -> &'src str {
        let next = self.consume();
        match next.kind {
            TKind::Ident(s) => s,
            _ => panic!("Expected identifier, found {}\n\n{}", next.kind, next.span),
        }
    }

    fn expect_strlit(&mut self) -> &'src [u8] {
        let next = self.consume();
        match next.kind {
            TKind::Str(s) => s,
            _ => panic!(
                "Expected string literal, found {}\n\n{}",
                next.kind, next.span
            ),
        }
    }

    fn expect_intlit(&mut self) -> u64 {
        let next = self.consume();
        match next.kind {
            TKind::Int(s) => s,
            _ => panic!("Expected int literal, found {}\n\n{}", next.kind, next.span),
        }
    }

    pub fn parse_program(&mut self) -> Vec<Object<'src, RawType<'src>>> {
        let mut objs = vec![];
        while !self.is_next(TKind::Eof) {
            let tok = self.peek();
            match tok.kind {
                TKind::Fn => objs.push(self.parse_fn()),
                TKind::Struct => objs.push(self.parse_struct()),
                TKind::Global => objs.push(self.parse_global()),
                x => panic!("Expected function definition, found {x}\n\n{}", tok.span),
            }
        }
        objs
    }

    fn parse_struct(&mut self) -> Object<'src, RawType<'src>> {
        todo!()
    }

    fn parse_global(&mut self) -> Object<'src, RawType<'src>> {
        self.expect(TKind::Global);
        let span = self.last_span;
        let name = self.expect_ident();
        self.expect(TKind::Colon);
        let ty = self.parse_type();
        self.expect(TKind::Eq);
        let rhs = self.parse_expr();
        Object {
            kind: OKind::Global(Symbol { name, ty }),
            span: span.merge(self.last_span),
        }
    }

    fn parse_fn(&mut self) -> Object<'src, RawType<'src>> {
        self.expect(TKind::Fn);
        let span = self.last_span;
        let name = self.expect_ident();
        self.expect(TKind::LParen);

        let mut args = vec![];
        if !self.is_next(TKind::RParen) {
            let name = self.expect_ident();
            self.expect(TKind::Colon);
            let ty = self.parse_type();
            let var = Symbol { name, ty };
            args.push(var);
        }

        loop {
            let cur = self.peek();
            if matches!(cur.kind, TKind::RParen) {
                self.consume();
                break;
            }

            self.expect(TKind::Comma);

            let name = self.expect_ident();
            self.expect(TKind::Colon);
            let ty = self.parse_type();
            let var = Symbol { name, ty };
            args.push(var);
        }

        let returns = if self.is_next(TKind::Arrow) {
            self.expect(TKind::Arrow);
            self.parse_type()
        } else {
            RawType::Base("void")
        };

        let body = self.parse_block();

        Object {
            kind: OKind::Fn {
                name,
                body,
                args,
                returns,
            },
            span: span.merge(self.last_span),
        }
    }

    fn parse_type(&mut self) -> RawType<'src> {
        let tok = self.peek();
        let mut ty = match tok.kind {
            TKind::Star => {
                self.consume();
                return RawType::Pointer(Box::new(self.parse_type()));
            }
            TKind::Ident(name) => RawType::Base(name),
            x => panic!("Expected type, found {x}\n\n{}", tok.span),
        };
        self.consume();
        ty
    }

    fn parse_prefix(&mut self) -> Expr<'src, RawType<'src>> {
        let tok = self.consume();
        let span = self.last_span;
        let kind = match tok.kind {
            TKind::Ident(s) => EKind::Symbol(Symbol {
                name: s,
                ty: RawType::Unknown,
            }),
            TKind::Bool(x) => EKind::Bool(x),
            TKind::Int(x) => EKind::Int(x),
            TKind::Str(s) => EKind::Str(s),
            op @ (TKind::Minus | TKind::Bang | TKind::And | TKind::Star) => EKind::Unary {
                op: match op {
                    TKind::Minus => UnOp::Negate,
                    TKind::Bang => UnOp::Not,
                    TKind::And => UnOp::AddrOf,
                    TKind::Star => UnOp::Deref,
                    _ => unreachable!(),
                },
                rhs: Box::new(self._parse_expr(prefix_power(op).expect("Invalid operator"))),
            },
            TKind::LParen => {
                let inner = self.parse_expr();
                self.expect(TKind::RParen);
                inner.kind
            }
            x => panic!("Expected start of expression, found \"{x}\"\n\n{}", tok.span),
        };
        self.make_expr(kind, span.merge(self.last_span))
    }

    fn parse_infix(
        &mut self,
        lhs: Expr<'src, RawType<'src>>,
        op: TKind<'src>,
        op_power: f32,
    ) -> Expr<'src, RawType<'src>> {
        let span = lhs.span;
        let kind = match op {
            TKind::LParen => {
                let mut args = vec![];
                while !self.is_next(TKind::RParen) {
                    if !args.is_empty() {
                        self.expect(TKind::Comma);
                    }
                    args.push(self.parse_expr());
                }
                self.expect(TKind::RParen);
                EKind::Call {
                    callee: Box::new(lhs),
                    args,
                }
            }
            op @ (TKind::Eq
            | TKind::Plus
            | TKind::Minus
            | TKind::Star
            | TKind::Slash
            | TKind::AndAnd
            | TKind::OrOr
            | TKind::EqEq
            | TKind::BangEq
            | TKind::Lt
            | TKind::LtEq
            | TKind::Gt
            | TKind::GtEq) => {
                let rhs = self._parse_expr(op_power);
                let binop = match op {
                    TKind::Eq => BinOp::Assign,
                    TKind::Plus => BinOp::Add,
                    TKind::Minus => BinOp::Sub,
                    TKind::Star => BinOp::Mul,
                    TKind::Slash => BinOp::Div,
                    TKind::AndAnd => BinOp::LogAnd,
                    TKind::OrOr => BinOp::LogOr,
                    TKind::EqEq => BinOp::Eq,
                    TKind::BangEq => BinOp::Ne,
                    TKind::Lt => BinOp::Lt,
                    TKind::LtEq => BinOp::Le,
                    TKind::Gt => BinOp::Gt,
                    TKind::GtEq => BinOp::Ge,
                    _ => unreachable!(),
                };
                EKind::Bin {
                    op: binop,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            TKind::LBrack => {
                let index = self._parse_expr(op_power);
                self.expect(TKind::RBrack);
                EKind::Index {
                    lhs: Box::new(lhs),
                    rhs: Box::new(index),
                }
            }
            TKind::Dot => {
                let field = self._parse_expr(op_power);
                if !matches!(field.kind, EKind::Symbol(_)) {
                    panic!("Fields can only be accessed with an identifier, not a {}\n\n{}", field.kind, field.span);
                }
                EKind::FieldAccess {
                    lhs: Box::new(lhs),
                    rhs: Box::new(field),
                }
            }
            x => panic!("Expected infix operator, found {x}\n\n{}", lhs.span),
        };
        self.make_expr(kind, span.merge(self.last_span))
    }

    // Pratt parsing!
    fn _parse_expr(&mut self, min_power: f32) -> Expr<'src, RawType<'src>> {
        let mut lhs = self.parse_prefix();

        loop {
            let op = self.peek().kind;

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

    pub fn parse_expr(&mut self) -> Expr<'src, RawType<'src>> {
        self._parse_expr(0.0)
    }

    fn parse_block(&mut self) -> Stmt<'src, RawType<'src>> {
        self.expect(TKind::LCurly);
        let span = self.last_span;
        let mut stmts = vec![];
        while !self.is_next(TKind::RCurly) {
            let stmt = self.parse_stmt();
            if matches!(stmt.kind, SKind::Block(ref inner) if inner.is_empty()) {
                // Skip nested empty {} blocks, they're useless.
                continue;
            }
            stmts.push(stmt);
        }
        self.expect(TKind::RCurly);
        self.make_stmt(SKind::Block(stmts), span.merge(self.last_span))
    }

    fn parse_stmt(&mut self) -> Stmt<'src, RawType<'src>> {
        let span;
        let tok = self.peek();
        let kind = match tok.kind {
            TKind::Let => {
                let mut ty = RawType::Unknown;
                self.expect(TKind::Let);
                span = self.last_span;
                let name = self.expect_ident();
                if self.is_next(TKind::Colon) {
                    self.expect(TKind::Colon);
                    ty = self.parse_type();
                }
                let lhs = Symbol { name, ty };
                self.expect(TKind::Eq);

                let rhs = self.parse_expr();
                self.expect(TKind::Semi);
                SKind::Let { lhs, rhs }
            }
            TKind::While => {
                self.expect(TKind::While);
                span = self.last_span;
                let cond = self.parse_expr();
                let body = self.parse_block();
                SKind::While {
                    cond,
                    body: Box::new(body),
                }
            }
            TKind::Continue => {
                self.expect(TKind::Continue);
                span = self.last_span;
                self.expect(TKind::Semi);
                SKind::Continue
            }
            TKind::Break => {
                self.expect(TKind::Break);
                span = self.last_span;
                self.expect(TKind::Semi);
                SKind::Break
            }
            TKind::If => {
                self.expect(TKind::If);
                span = self.last_span;
                let cond = self.parse_expr();
                let then_ = self.parse_block();

                let else_ = if self.is_next(TKind::Else) {
                    self.expect(TKind::Else);
                    if self.is_next(TKind::If) {
                        // If is a special case since it's allowed after an else
                        self.parse_stmt()
                    } else {
                        // Otherwise, else must be followed by a block
                        self.parse_block()
                    }
                } else {
                    self.make_stmt(SKind::Block(vec![]), self.last_span)
                };
                SKind::If {
                    cond,
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            }
            TKind::Return => {
                self.expect(TKind::Return);
                span = self.last_span;
                let retval = if !self.is_next(TKind::Semi) {
                    self.parse_expr()
                } else {
                    self.make_expr(EKind::Nothing, span)
                };
                self.expect(TKind::Semi);

                SKind::Return(retval)
            }
            x => {
                let expr = self.parse_expr();
                span = expr.span;
                self.expect(TKind::Semi);
                SKind::Expr(expr)
            }
        };

        self.make_stmt(kind, span.merge(self.last_span))
    }
}

fn prefix_power(kind: TKind) -> Option<f32> {
    let power = match kind {
        TKind::Bang | TKind::Minus | TKind::Star | TKind::And => 9.0,
        _ => return None, // Not an prefix operator
    };
    Some(power)
}

fn infix_power(kind: TKind) -> Option<(f32, f32)> {
    let power = match kind {
        // Assignment: Right-Associative
        // We use a lower Left power so it "gives up" easily,
        // but a higher Right power to "grab" everything to the right.
        TKind::Eq => (2.1, 2.0),

        TKind::AndAnd | TKind::OrOr => (4.1, 4.0),

        TKind::EqEq | TKind::BangEq => (5.0, 5.1),
        TKind::Lt | TKind::Gt => (6.0, 6.1),
        TKind::LtEq | TKind::GtEq => (6.0, 6.1),

        TKind::Plus | TKind::Minus => (7.0, 7.1),

        TKind::Star | TKind::Slash => (8.0, 8.1),

        // Postfix: Highest priority
        // (Call, Indexing, Member Access)
        TKind::LParen | TKind::LBrack | TKind::Dot => (10.0, 10.1),

        _ => return None, // Not an infix operator
    };
    Some(power)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_expr() {
        let src = b"5+5";
        let lexed: Vec<_> = Lexer::new(src).collect();
        // println!("{lexed:#?}");
        let parsed = Parser::new(&lexed, src).parse_expr();
        println!("{parsed:#?}");
    }
}
