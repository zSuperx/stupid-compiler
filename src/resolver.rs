use std::collections::HashMap;

use crate::types::*;

pub struct Resolver<'src> {
    src: &'src [Object<'src, RawType<'src>>],
    types: HashMap<&'src str, ResolvedType>,
    scope_stack: Vec<HashMap<&'src str, Symbol<'src, ResolvedType>>>,
    return_type_stack: Vec<ResolvedType>,
}

impl<'src> Resolver<'src> {
    pub fn new(src: &'src [Object<'src, RawType<'src>>]) -> Self {
        use ResolvedType::*;
        let types = HashMap::from([
            ("u8", U8),
            ("u16", U16),
            ("u32", U32),
            ("u64", U64),
            ("bool", Bool),
            ("void", Void),
        ]);

        Self {
            src,
            types,
            scope_stack: vec![HashMap::new()],
            return_type_stack: vec![],
        }
    }

    pub fn resolve_program(&mut self) -> Vec<Object<'src, ResolvedType>> {
        self.src.iter().map(|o| self.resolve_object(o)).collect()
    }

    fn resolve_type(&mut self, ty: &RawType<'src>) -> Option<ResolvedType> {
        match ty {
            RawType::Unknown => None,
            RawType::Base(x) => Some(
                self.types
                    .get(x)
                    .expect(&format!("Unrecognized type: {ty:?}"))
                    .clone(),
            ),
            RawType::Pointer(raw_type) => Some(ResolvedType::Pointer(Box::new(
                self.resolve_type(raw_type).unwrap(),
            ))),
        }
    }

    fn resolve_object(&mut self, obj: &Object<'src, RawType<'src>>) -> Object<'src, ResolvedType> {
        match obj {
            Object::Fn {
                name,
                args,
                returns,
                body,
            } => {
                let resolved_return = self
                    .resolve_type(returns)
                    .expect(&format!("Unknown type {:?}", returns));
                let mut resolved_args = vec![];
                let mut scope = HashMap::new();
                for arg in args {
                    let resolved_arg = Symbol {
                        name: arg.name,
                        ty: self
                            .resolve_type(&arg.ty)
                            .expect(&format!("Unknown type {:?}", arg.ty)),
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
                    ty: ResolvedType::Function {
                        args: resolved_args.iter().map(|arg| arg.ty.clone()).collect(),
                        returns: Box::new(resolved_return.clone()),
                    },
                };

                if self.scope_stack[0].insert(name, fn_sym).is_some() {
                    panic!(r#"Function "{name}" already defined"#);
                }

                self.return_type_stack.push(resolved_return.clone());
                self.scope_stack.push(scope);
                let resolved_body = self.resolve_stmt(body);
                self.scope_stack.pop();
                self.return_type_stack.pop();
                Object::Fn {
                    name,
                    returns: resolved_return,
                    args: resolved_args,
                    body: resolved_body,
                }
            }
            Object::Global(symbol) => todo!(),
            Object::Struct { name, fields } => todo!(),
        }
    }

    fn resolve_symbol(&mut self, sym: &Symbol<'src, RawType<'src>>) -> Symbol<'src, ResolvedType> {
        Symbol {
            name: sym.name,
            ty: self.resolve_type(&sym.ty).expect(&format!(
                "Unknown type on variable {}: {:?}",
                sym.name, sym.ty
            )),
        }
    }

    fn resolve_expr(
        &mut self,
        expr: &Expr<'src, RawType<'src>>,
        hint: Option<&ResolvedType>,
    ) -> Expr<'src, ResolvedType> {
        match &expr.kind {
            ExprKind::Nothing => Expr {
                ty: ResolvedType::Void,
                kind: ExprKind::Nothing,
            },
            ExprKind::Symbol(x) => {
                let sym = self
                    .scope_stack
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(x.name))
                    .expect(&format!("Variable used but not defined: {}", x.name))
                    .clone();
                Expr {
                    ty: sym.ty.clone(),
                    kind: ExprKind::Symbol(sym),
                }
            }
            ExprKind::Int(x) => {
                let mut ty = ResolvedType::I32;
                if let Some(hint_ty) = hint {
                    if is_integral(hint_ty) {
                        ty = hint_ty.clone()
                    } else {
                        panic!("Integer literal is not of type: {hint_ty:?}");
                    }
                }
                Expr {
                    kind: ExprKind::Int(*x),
                    ty: ty,
                }
            }
            ExprKind::Bool(x) => {
                if hint.is_some_and(|t| *t != ResolvedType::Bool) {
                    panic!("Mismatched types: Lhs = {hint:?}, Rhs = Bool");
                };
                Expr {
                    kind: ExprKind::Bool(*x),
                    ty: ResolvedType::Bool,
                }
            }
            ExprKind::Str(x) => {
                if let Some(ResolvedType::Pointer(ty)) = hint {
                    if **ty != ResolvedType::U8 {
                        panic!("Mismatched types: Lhs = {hint:?}, Rhs = *u8");
                    }
                }
                Expr {
                    kind: ExprKind::Str(x),
                    ty: ResolvedType::Pointer(Box::new(ResolvedType::U8)),
                }
            }
            ExprKind::Call { callee, args } => {
                let callee = self.resolve_expr(callee, hint);
                let ResolvedType::Function {
                    args: expected_args,
                    returns,
                } = &callee.ty
                else {
                    panic!(
                        "The expression: \"{:?}\" does not resolve to a function",
                        callee
                    );
                };
                if args.len() != expected_args.len() {
                    panic!(
                        "Function \"{:?}\" takes {} arguments, but {} were given",
                        callee,
                        expected_args.len(),
                        args.len()
                    );
                }
                let args = args
                    .iter()
                    .zip(expected_args.iter())
                    .map(|(a, e)| self.resolve_expr(a, Some(e)))
                    .collect();
                Expr {
                    ty: *returns.clone(),
                    kind: ExprKind::Call {
                        callee: Box::new(callee),
                        args,
                    },
                }
            }
            ExprKind::Unary { op, rhs } => match op {
                UnOp::Negate => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if !is_integral(&rhs.ty) || !is_literal(&rhs) {
                        panic!("Negation (-) can only be used on literal integral types");
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: ExprKind::Unary {
                            op: UnOp::Negate,
                            rhs: Box::new(rhs),
                        },
                    }
                }
                UnOp::Not => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if rhs.ty != ResolvedType::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {:?}",
                            rhs.ty
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: ExprKind::Unary {
                            op: UnOp::Not,
                            rhs: Box::new(rhs),
                        },
                    }
                }
                UnOp::AddrOf => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if is_literal(&rhs) {
                        panic!("Cannot take the address of a literal type {:?}", rhs.ty);
                    }
                    Expr {
                        ty: ResolvedType::Pointer(Box::new(rhs.ty.clone())),
                        kind: ExprKind::Unary {
                            op: UnOp::AddrOf,
                            rhs: Box::new(rhs),
                        },
                    }
                }
                UnOp::Deref => {
                    let rhs = self.resolve_expr(rhs, hint);
                    let ResolvedType::Pointer(inner_type) = rhs.ty.clone() else {
                        panic!("Cannot take the address of a literal type {:?}", rhs.ty);
                    };
                    Expr {
                        ty: *inner_type,
                        kind: ExprKind::Unary {
                            op: UnOp::AddrOf,
                            rhs: Box::new(rhs),
                        },
                    }
                }
            },
            ExprKind::Bin { op, lhs, rhs } => match op {
                BinOp::Assign => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: ExprKind::Bin {
                            op: BinOp::Assign,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
                op @ (BinOp::Add | BinOp::Sub | BinOp::Sub | BinOp::Mul | BinOp::Div) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to perform arithmetic on non-integer type: {:?}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: ExprKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
                op @ (BinOp::Gt | BinOp::Ge | BinOp::Lt | BinOp::Le) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to ordered comparison on non-integer type: {:?}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: ExprKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
                op @ (BinOp::Eq | BinOp::Ne) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: ExprKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
                op @ (BinOp::LogOr | BinOp::LogAnd) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    if rhs.ty != ResolvedType::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {:?}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: ExprKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
            },
            ExprKind::FieldAccess { lhs, rhs } => todo!(),
            ExprKind::Index { lhs, rhs } => todo!(),
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt<'src, RawType<'src>>) -> Stmt<'src, ResolvedType> {
        match stmt {
            Stmt::Let { lhs, rhs } => {
                let hint_ty = self.resolve_type(&lhs.ty);
                let rhs = self.resolve_expr(rhs, hint_ty.as_ref());
                if let Some(ref ty) = hint_ty
                    && ty != &rhs.ty
                {
                    panic!("Mismatched types lhs = {ty:?}, rhs = {:?}", rhs.ty)
                }
                let sym = Symbol {
                    name: lhs.name,
                    ty: rhs.ty.clone(),
                };
                self.scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(lhs.name, sym.clone());
                return Stmt::Let { lhs: sym, rhs };
            }
            Stmt::While { cond, body } => {
                let cond = self.resolve_expr(cond, None);
                if cond.ty != ResolvedType::Bool {
                    panic!("Loop conditions must resolve to a Bool");
                }
                return Stmt::While {
                    cond,
                    body: Box::new(self.resolve_stmt(body)),
                };
            }
            Stmt::Continue => return Stmt::Continue,
            Stmt::Break => return Stmt::Break,
            Stmt::If { cond, then_, else_ } => {
                let cond = self.resolve_expr(cond, None);
                if cond.ty != ResolvedType::Bool {
                    panic!("If conditions must resolve to a Bool");
                }
                return Stmt::If {
                    cond,
                    then_: Box::new(self.resolve_stmt(then_)),
                    else_: Box::new(self.resolve_stmt(else_)),
                };
            }
            Stmt::Return(expr) => {
                let expected_return_type = self.return_type_stack.last().unwrap().clone();
                let expr = self.resolve_expr(
                    expr,
                    (expected_return_type != ResolvedType::Void).then(|| &expected_return_type),
                );
                if expr.ty != expected_return_type {
                    panic!(
                        "Function has return type {:?}, but {:?} is returned instead",
                        expected_return_type, expr.ty
                    );
                }
                return Stmt::Return(expr);
            }
            Stmt::Block(stmts) => {
                self.scope_stack.push(HashMap::new());
                let stmts = stmts.iter().map(|s| self.resolve_stmt(s)).collect();
                self.scope_stack.pop();
                return Stmt::Block(stmts);
            }
            Stmt::Expr(expr) => {
                return Stmt::Expr(self.resolve_expr(expr, None));
            }
        }
    }
}

fn is_integral(ty: &ResolvedType) -> bool {
    use ResolvedType::*;
    matches!(ty, U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64)
}

fn is_literal<T>(expr: &Expr<T>) -> bool {
    matches!(
        expr.kind,
        ExprKind::Int(_) | ExprKind::Str(_) | ExprKind::Bool(_)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::Parser, tokenizer::Lexer, types::*};

    #[test]
    fn test_resolve() {
        let src = br#"
fn main() -> u8 {
    let x = main();
}

fn main() {
    let x = 4;
}
"#;
        println!("Source code:\n{}\n", str::from_utf8(src).unwrap());
        let tokens: Vec<Kind> = Lexer::new(src).map(|t| t.kind).collect();
        let parsed = Parser::new(&tokens).parse_program();
        let resolved = Resolver::new(&parsed).resolve_program();
        dbg!(&resolved);
    }
}
