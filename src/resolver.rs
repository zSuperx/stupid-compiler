use std::collections::HashMap;

use crate::types::*;

pub struct Resolver {
    scope_stack: Vec<HashMap<Symbol, SymbolId>>,
    return_stack: Vec<Type>,
    loop_stack: usize,
}

pub type Terminates = bool;

impl Resolver {
    pub fn new() -> Self {
        Self {
            scope_stack: vec![HashMap::new()],
            return_stack: vec![],
            loop_stack: 0,
        }
    }

    pub fn resolve_program(&mut self, ctx: &mut Context, objs: &[Object]) -> Vec<Object> {
        objs.iter().map(|o| self.resolve_object(ctx, o)).collect()
    }

    fn resolve_type(&mut self, ctx: &mut Context, ty: &Type) -> Option<Type> {
        todo!()
    }

    pub fn resolve_object(&mut self, ctx: &mut Context, obj: &Object) -> Object {
        let kind = match &obj.kind {
            OKind::Fn {
                name,
                args,
                returns,
                body,
            } => {
                let Some(resolved_return) = self.resolve_type(ctx, returns) else {
                    panic!("Unknown type {:?}", returns);
                };
                let mut resolved_args = vec![];
                let mut scope = HashMap::new();
                for arg_id in args {
                    let arg = &ctx.symbols[*arg_id];
                    if scope.contains_key(&arg.name) {
                        panic!("Argument {} already defined", ctx.lookup_symbol(arg.name));
                    } else {
                        scope.insert(arg.name, *arg_id);
                    }
                    resolved_args.push(*arg_id);
                }

                let fn_ty = resolved_args.iter().map(|a| {
                    ctx.symbols[*a].ty
                })

                if self.scope_stack[0].insert(name, fn_sym).is_some() {
                    panic!(r#"Function "{name}" already defined"#);
                }

                self.return_stack.push(resolved_return.clone());
                self.scope_stack.push(scope);
                let (resolved_body, terminates) = self.resolve_stmt(ctx, body);
                if !terminates && Type_return != Type::Void {
                    panic!(
                        "Function {} expects return type {}, but not all paths return a value {}",
                        name, resolved_return, obj.span
                    );
                }
                self.scope_stack.pop();
                let locals = HashMap::new();
                self.return_stack.pop();
                OKind::Fn {
                    name,
                    returns: resolved_return,
                    args: resolved_args,
                    locals,
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

    pub fn resolve_expr(&mut self, ctx: &mut Context, expr: &Expr, hint: &Type) -> Expr {
        match &expr.kind {
            EKind::Nothing => Expr {
                ty: TyKind::Void,
                kind: EKind::Nothing,
                span: expr.span,
            },
            EKind::Unresolved(x) => {
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
                let mut ty = Type::I32;
                if *hint != Type::Infer {
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
                if *hint != Type::Bool {
                    panic!(
                        "Mismatched types. Expected {hint}, found bool {}",
                        expr.span
                    );
                };
                Expr {
                    kind: EKind::Bool(*x),
                    ty: Type::Bool,
                    span: expr.span,
                }
            }
            EKind::Str(x) => {
                if let Type::Pointer(ty) = hint
                    && **ty != Type::U8
                {
                    panic!("Mismatched types. Expected {ty}, found *u8 {}", expr.span);
                }
                Expr {
                    kind: EKind::Str(x),
                    ty: Type::Pointer(Box::new(Type::U8)),
                    span: expr.span,
                }
            }
            EKind::Call { callee, args } => {
                let callee = self.resolve_expr(ctx, callee, hint);
                let Type::Function {
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
                        let r = self.resolve_expr(ctx, a, e);
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
                    let rhs = self.resolve_expr(ctx, rhs, hint);
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
                    let rhs = self.resolve_expr(ctx, rhs, hint);
                    if rhs.ty != Type::Bool {
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
                    let mut rhs = self.resolve_expr(ctx, rhs, hint);
                    if let EKind::Symbol(sym) = &mut rhs.kind {
                        sym.addressed = true;
                    } else {
                        panic!("Cannot take the address of an invalid LVALUE {}", rhs.span);
                    }
                    Expr {
                        ty: Type::Pointer(Box::new(rhs.ty.clone())),
                        kind: EKind::Unary {
                            op: UnOp::AddrOf,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::Deref => {
                    let rhs = self.resolve_expr(ctx, rhs, hint);
                    let Type::Pointer(inner_type) = rhs.ty.clone() else {
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
                    let lhs = self.resolve_expr(ctx, lhs, hint);
                    let rhs = self.resolve_expr(ctx, rhs, &lhs.ty);
                    if rhs.ty == Type::Infer {
                        panic!("Variable may be used uninitialized {}", lhs.span);
                    }
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types. Expected {}, found {}", lhs.ty, rhs.ty);
                    }
                    let valid_lvalue = match &lhs.kind {
                        EKind::Symbol(symbol) => !matches!(symbol.ty, Type::Function { .. }),
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
                    let lhs = self.resolve_expr(ctx, lhs, hint);
                    let rhs = self.resolve_expr(ctx, rhs, &lhs.ty);
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
                    let lhs = self.resolve_expr(ctx, lhs, hint);
                    let rhs = self.resolve_expr(ctx, rhs, &lhs.ty);
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
                        ty: Type::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Eq | BinOp::Ne) => {
                    let lhs = self.resolve_expr(ctx, lhs, hint);
                    let rhs = self.resolve_expr(ctx, rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    Expr {
                        ty: Type::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::LogOr | BinOp::LogAnd) => {
                    let lhs = self.resolve_expr(ctx, lhs, hint);
                    let rhs = self.resolve_expr(ctx, rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if rhs.ty != Type::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {} {}",
                            lhs.ty, expr.span
                        );
                    }
                    Expr {
                        ty: Type::Bool,
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

    fn resolve_stmt(&mut self, ctx: &mut Context, stmt: &Stmt) -> (Stmt, Terminates) {
        let span = stmt.span;
        let (kind, terminates) = match &stmt.kind {
            SKind::Let { lhs, rhs } => {
                let Some(hint) = self.resolve_type(ctx, &lhs.ty) else {
                    panic!("Unknown type {} {}", lhs.ty, span);
                };
                let rhs = self.resolve_expr(ctx, rhs, &hint);
                if let Type::Function { .. } = rhs.ty {
                    panic!(
                        "Cannot bind raw function types to variables. {}\nConsider taking the address of the function instead ",
                        rhs.span
                    );
                };
                if hint != Type::Infer && hint != rhs.ty {
                    panic!(
                        "Mismatched types. Expected {}, found {} {}",
                        hint, rhs.ty, rhs.span
                    )
                }
                let sym = Symbol {
                    name: lhs.name,
                    ty: rhs.ty.clone(),
                    addressed: false,
                };
                self.scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(lhs.name, sym.clone());
                (SKind::Let { lhs: sym, rhs }, false)
            }
            SKind::While { cond, body } => {
                let cond = self.resolve_expr(ctx, &cond, &Type::Bool);
                if cond.ty != Type::Bool {
                    panic!(
                        "Loop condition resolves to a non-boolean type: {} {}",
                        cond.ty, cond.span
                    );
                }
                self.loop_stack += 1;
                let (body, _) = self.resolve_stmt(ctx, body.as_ref());
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
                let cond = self.resolve_expr(ctx, &cond, &Type::Bool);
                if cond.ty != Type::Bool {
                    panic!(
                        "If condition resolves to a non-boolean type: {} {}",
                        cond.ty, cond.span
                    );
                }

                let (then_, then_terminates) = self.resolve_stmt(ctx, &then_);
                let (else_, else_terminates) = self.resolve_stmt(ctx, &else_);
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
                let expr = self.resolve_expr(ctx, &expr, &expected_return_type);
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
                        let (s, t) = self.resolve_stmt(ctx, s);
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
                SKind::Expr(self.resolve_expr(ctx, &expr, &Type::Void)),
                false,
            ),
        };
        (Stmt { kind, span }, terminates)
    }
}

fn is_integral(ty: &Type) -> bool {
    use Type::*;
    matches!(ty, U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64)
}

fn is_literal<T>(expr: &Expr<T>) -> bool {
    matches!(expr.kind, EKind::Int(_) | EKind::Str(_) | EKind::Bool(_))
}
