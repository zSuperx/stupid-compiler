use std::collections::HashMap;

use crate::types::*;

pub struct Resolver<'src> {
    types: HashMap<&'src str, Resolved>,
    scope_stack: Vec<HashMap<&'src str, Symbol<'src, Resolved>>>,
    return_stack: Vec<Resolved>,
    loop_stack: usize,
}

pub type Terminates = bool;

impl<'src> Resolver<'src> {
    pub fn new() -> Self {
        use Resolved::*;
        let types = HashMap::from([
            ("u8", U8),
            ("u16", U16),
            ("u32", U32),
            ("u64", U64),
            ("i8", I8),
            ("i16", I16),
            ("i32", I32),
            ("i64", I64),
            ("bool", Bool),
            ("void", Void),
        ]);

        Self {
            types,
            scope_stack: vec![HashMap::new()],
            return_stack: vec![],
            loop_stack: 0,
        }
    }

    pub fn resolve_program(
        &mut self,
        objs: &[Object<'src, Raw<'src>>],
    ) -> Vec<Object<'src, Resolved>> {
        objs.iter().map(|o| self.resolve_object(o)).collect()
    }

    fn resolve_type(&mut self, ty: &Raw<'src>) -> Option<Resolved> {
        match ty {
            Raw::Infer => Some(Resolved::Infer),
            Raw::Base(x) => self.types.get(x).cloned(),
            Raw::Pointer(raw_type) => self
                .resolve_type(raw_type)
                .map(|t| Resolved::Pointer(Box::new(t))),
        }
    }

    pub fn resolve_object(&mut self, obj: &Object<'src, Raw<'src>>) -> Object<'src, Resolved> {
        let kind = match &obj.kind {
            OKind::Fn {
                name,
                args,
                returns,
                body,
            } => {
                let Some(resolved_return) = self.resolve_type(returns) else {
                    panic!("Unknown type {} {}", returns, obj.span);
                };
                let mut resolved_args = vec![];
                let mut scope = HashMap::new();
                for arg in args {
                    let resolved_arg = Symbol {
                        name: arg.name,
                        ty: self
                            .resolve_type(&arg.ty)
                            .expect(&format!("Unknown type {}", arg.ty)),
                    };
                    if scope.contains_key(arg.name) {
                        panic!("Argument {} already defined", arg.name);
                    } else {
                        scope.insert(arg.name, resolved_arg.clone());
                    }
                    resolved_args.push(resolved_arg);
                }

                let fn_sym = Symbol {
                    name,
                    ty: Resolved::Function {
                        args: resolved_args.iter().map(|arg| arg.ty.clone()).collect(),
                        returns: Box::new(resolved_return.clone()),
                    },
                };

                if self.scope_stack[0].insert(name, fn_sym).is_some() {
                    panic!(r#"Function "{name}" already defined"#);
                }

                self.return_stack.push(resolved_return.clone());
                self.scope_stack.push(scope);
                let (resolved_body, terminates) = self.resolve_stmt(body);
                if !terminates && resolved_return != Resolved::Void {
                    panic!(
                        "Function {} expects return type {}, but not all paths return a value {}",
                        name, resolved_return, obj.span
                    );
                }
                self.scope_stack.pop();
                self.return_stack.pop();
                OKind::Fn {
                    name,
                    returns: resolved_return,
                    args: resolved_args,
                    body: resolved_body,
                }
            }
            OKind::Global { .. } => todo!(),
            OKind::Struct { .. } => todo!(),
        };

        Object {
            kind,
            span: obj.span,
        }
    }

    pub fn resolve_expr(
        &mut self,
        expr: &Expr<'src, Raw<'src>>,
        hint: &Resolved,
    ) -> Expr<'src, Resolved> {
        match &expr.kind {
            EKind::Nothing => Expr {
                ty: Resolved::Void,
                kind: EKind::Nothing,
                span: expr.span,
            },
            EKind::Symbol(x) => {
                let Some(sym) = self
                    .scope_stack
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(x.name))
                    .cloned()
                else {
                    panic!("Variable used but not defined: {} {}", x.name, expr.span)
                };
                Expr {
                    ty: sym.ty.clone(),
                    kind: EKind::Symbol(sym),
                    span: expr.span,
                }
            }
            EKind::Int(x) => {
                let mut ty = Resolved::I32;
                if *hint != Resolved::Infer {
                    if is_integral(&hint) {
                        ty = hint.clone()
                    } else {
                        panic!(
                            "Mismatched types. Expected {hint}, found integer type {}",
                            expr.span
                        );
                    }
                }
                Expr {
                    kind: EKind::Int(*x),
                    ty,
                    span: expr.span,
                }
            }
            EKind::Bool(x) => {
                if *hint != Resolved::Bool {
                    panic!(
                        "Mismatched types. Expected {hint}, found bool {}",
                        expr.span
                    );
                };
                Expr {
                    kind: EKind::Bool(*x),
                    ty: Resolved::Bool,
                    span: expr.span,
                }
            }
            EKind::Str(x) => {
                if let Resolved::Pointer(ty) = hint
                    && **ty != Resolved::U8
                {
                    panic!("Mismatched types. Expected {ty}, found *u8 {}", expr.span);
                }
                Expr {
                    kind: EKind::Str(x),
                    ty: Resolved::Pointer(Box::new(Resolved::U8)),
                    span: expr.span,
                }
            }
            EKind::Call { callee, args } => {
                let callee = self.resolve_expr(callee, hint);
                let Resolved::Function {
                    args: expected_args,
                    returns,
                } = &callee.ty
                else {
                    panic!("Expression does not resolve to a function {}", callee.span);
                };
                if args.len() != expected_args.len() {
                    panic!(
                        "Function takes {} arguments, but {} were given {}",
                        expected_args.len(),
                        args.len(),
                        callee.span,
                    );
                }
                let args = args
                    .iter()
                    .zip(expected_args.iter())
                    .map(|(a, e)| {
                        let r = self.resolve_expr(a, e);
                        if r.ty != *e {
                            panic!(
                                "Mismatched types. Expected {} but got {} {}",
                                e, r.ty, r.span
                            );
                        }
                        r
                    })
                    .collect();
                Expr {
                    ty: *returns.clone(),
                    kind: EKind::Call {
                        callee: Box::new(callee),
                        args,
                    },
                    span: expr.span,
                }
            }
            EKind::Unary { op, rhs } => match op {
                UnOp::Negate => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if !is_integral(&rhs.ty) {
                        panic!("Negation can only be used on integral types {}", rhs.span);
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Unary {
                            op: UnOp::Negate,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::Not => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if rhs.ty != Resolved::Bool {
                        panic!(
                            "Logical operations can only be used on booleans {}",
                            rhs.span
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Unary {
                            op: UnOp::Not,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::AddrOf => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if is_literal(&rhs) {
                        panic!(
                            "Cannot take the address of a literal type {} {}",
                            rhs.ty, rhs.span
                        );
                    }
                    Expr {
                        ty: Resolved::Pointer(Box::new(rhs.ty.clone())),
                        kind: EKind::Unary {
                            op: UnOp::AddrOf,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::Deref => {
                    let rhs = self.resolve_expr(rhs, hint);
                    let Resolved::Pointer(inner_type) = rhs.ty.clone() else {
                        panic!("Cannot dereference a literal type {} {}", rhs.ty, rhs.span);
                    };
                    Expr {
                        ty: *inner_type,
                        kind: EKind::Unary {
                            op: UnOp::Deref,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
            },
            EKind::Bin { op, lhs, rhs } => match op {
                BinOp::Assign => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if rhs.ty == Resolved::Infer {
                        panic!("Variable may be used uninitialized {}", lhs.span);
                    }
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types. Expected {}, found {}", lhs.ty, rhs.ty);
                    }
                    let valid_lvalue = match &lhs.kind {
                        EKind::Symbol(symbol) => !matches!(symbol.ty, Resolved::Function { .. }),
                        EKind::Unary {
                            op: UnOp::Deref, ..
                        } => true,
                        EKind::FieldAccess { .. } => true,
                        _ => false,
                    };
                    if !valid_lvalue {
                        panic!("Expression is not assignable {}", lhs.span);
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Bin {
                            op: BinOp::Assign,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to perform arithmetic on non-integer type: {}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Gt | BinOp::Ge | BinOp::Lt | BinOp::Le) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to ordered comparison on non-integer type: {}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: Resolved::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Eq | BinOp::Ne) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    Expr {
                        ty: Resolved::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::LogOr | BinOp::LogAnd) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if rhs.ty != Resolved::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {} {}",
                            lhs.ty, expr.span
                        );
                    }
                    Expr {
                        ty: Resolved::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
            },
            EKind::FieldAccess { .. } => todo!(),
            EKind::Index { .. } => todo!(),
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt<'src, Raw<'src>>) -> (Stmt<'src, Resolved>, Terminates) {
        let span = stmt.span;
        let (kind, terminates) = match &stmt.kind {
            SKind::Let { lhs, rhs } => {
                let Some(hint) = self.resolve_type(&lhs.ty) else {
                    panic!("Unknown type {} {}", lhs.ty, span);
                };
                let rhs = self.resolve_expr(rhs, &hint);
                if let Resolved::Function { .. } = rhs.ty {
                    panic!(
                        "Cannot bind raw function types to variables. {}\nConsider taking the address of the function instead ",
                        rhs.span
                    );
                };
                if hint != Resolved::Infer && hint != rhs.ty {
                    panic!(
                        "Mismatched types. Expected {}, found {} {}",
                        hint, rhs.ty, rhs.span
                    )
                }
                let sym = Symbol {
                    name: lhs.name,
                    ty: rhs.ty.clone(),
                };
                self.scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(lhs.name, sym.clone());
                (SKind::Let { lhs: sym, rhs }, false)
            }
            SKind::While { cond, body } => {
                let cond = self.resolve_expr(&cond, &Resolved::Bool);
                if cond.ty != Resolved::Bool {
                    panic!(
                        "Loop condition resolves to a non-boolean type: {} {}",
                        cond.ty, cond.span
                    );
                }
                self.loop_stack += 1;
                let (body, _) = self.resolve_stmt(body.as_ref());
                self.loop_stack -= 1;
                (
                    SKind::While {
                        cond,
                        body: Box::new(body),
                    },
                    false,
                )
            }
            SKind::Continue => {
                if self.loop_stack == 0 {
                    panic!(
                        "'continue' statements can only be called from within loops {}",
                        span
                    );
                }
                (SKind::Continue, true)
            }
            SKind::Break => {
                if self.loop_stack == 0 {
                    panic!(
                        "'break' statements can only be called from within loops {}",
                        span
                    );
                }
                (SKind::Break, true)
            }
            SKind::If { cond, then_, else_ } => {
                let cond = self.resolve_expr(&cond, &Resolved::Bool);
                if cond.ty != Resolved::Bool {
                    panic!(
                        "If condition resolves to a non-boolean type: {} {}",
                        cond.ty, cond.span
                    );
                }

                let (then_, then_terminates) = self.resolve_stmt(&then_);
                let (else_, else_terminates) = self.resolve_stmt(&else_);
                (
                    SKind::If {
                        cond,
                        then_: Box::new(then_),
                        else_: Box::new(else_),
                    },
                    then_terminates && else_terminates,
                )
            }
            SKind::Return(expr) => {
                let expected_return_type = self.return_stack.last().unwrap().clone();
                let expr = self.resolve_expr(&expr, &expected_return_type);
                if expr.ty != expected_return_type {
                    panic!(
                        "Function has return type {}, but {} was returned instead {}",
                        expected_return_type, expr.ty, expr.span
                    );
                }
                (SKind::Return(expr), true)
            }
            SKind::Block(stmts) => {
                self.scope_stack.push(HashMap::new());
                let mut terminates = false;
                let mut last_span = span;
                let stmts = stmts
                    .iter()
                    .map(|s| {
                        let (s, t) = self.resolve_stmt(s);
                        if terminates {
                            panic!("Unreachable code after this statement {}", last_span);
                        }
                        last_span = s.span;
                        terminates = t;
                        s
                    })
                    .collect();
                self.scope_stack.pop();
                (SKind::Block(stmts), terminates)
            }
            SKind::Expr(expr) => (
                SKind::Expr(self.resolve_expr(&expr, &Resolved::Void)),
                false,
            ),
        };
        (Stmt { kind, span }, terminates)
    }
}

fn is_integral(ty: &Resolved) -> bool {
    use Resolved::*;
    matches!(ty, U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64)
}

fn is_literal<T>(expr: &Expr<T>) -> bool {
    matches!(expr.kind, EKind::Int(_) | EKind::Str(_) | EKind::Bool(_))
}
