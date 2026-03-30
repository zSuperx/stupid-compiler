use std::{collections::HashMap, ops::DerefMut};

use crate::types::*;

pub struct Codegen<'src> {
    vr: usize,
    program: String,
    symbols: HashMap<&'src str, usize>,
    label_count: usize,
}

#[derive(Debug)]
pub struct IRNode {
    inputs: Vec<usize>, // input virtual registers
    output: usize,      // output virtual register
}

impl<'src> Codegen<'src> {
    pub fn new() -> Self {
        Self {
            vr: 0,
            label_count: 0,
            program: String::new(),
            symbols: HashMap::new(),
        }
    }

    fn emit_merged_node(
        &mut self,
        nodes: Vec<IRNode>,
        next_program: String,
        target: usize,
    ) -> IRNode {
        let mut inputs = vec![];
        for mut node in nodes {
            inputs.append(&mut node.inputs);
        }
        if !next_program.is_empty() {
            self.program.push_str("\t");
            self.program.push_str(&next_program);
            self.program.push_str("\n");
        }
        IRNode {
            inputs,
            output: target,
        }
    }

    fn emit_label(&mut self, name: Option<&str>) -> String {
        let label = match name {
            Some(name) => format!("F{name}:\n"),
            None => {
                self.label_count += 1;
                format!("L{}:\n", self.label_count - 1)
            }
        };
        self.program.push_str(&label);
        label
    }

    fn next_vr(&mut self) -> usize {
        self.vr += 1;
        self.vr - 1
    }

    pub fn generate(mut self, objs: &[Object<'src, ResolvedType>]) -> String {
        for obj in objs {
            self.emit_object(obj);
        }
        self.program
    }

    fn emit_object(&mut self, obj: &Object<'src, ResolvedType>) {
        match obj {
            Object::Fn {
                name,
                returns,
                args,
                body,
            } => {
                self.emit_label(Some(name));
                self.symbols.clear();
                for arg in args {
                    let vr = self.next_vr();
                    self.symbols.insert(arg.name, vr);
                }
                self.emit_stmt(body);
            }
            Object::Global(symbol) => todo!(),
            Object::Struct { name, fields } => todo!(),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt<'src, ResolvedType>) {
        match stmt {
            Stmt::Let { lhs, rhs } => {
                let target = self.next_vr();
                if self.symbols.insert(lhs.name, target).is_some() {
                    panic!("Duplicate symbol {:?}", lhs);
                }
                let rhs = self.emit_expr(rhs);
                let program = format!("%{target} = %{}", rhs.output);
                self.emit_merged_node(vec![rhs], program, target);
            }
            Stmt::While { cond, body } => todo!(),
            Stmt::Continue => todo!(),
            Stmt::Break => todo!(),
            Stmt::If { cond, then_, else_ } => todo!(),
            Stmt::Return(expr) => todo!(),
            Stmt::Block(stmts) => {
                for stmt in stmts {
                    self.emit_stmt(stmt);
                }
            }
            Stmt::Expr(expr) => _ = self.emit_expr(expr),
        }
    }

    fn emit_expr(&mut self, Expr { kind, ty }: &Expr<'src, ResolvedType>) -> IRNode {
        let target;
        let (template, nodes) = match kind {
            EKind::Symbol(symbol) => {
                target = *self.symbols.get(symbol.name).unwrap();
                (format!(""), vec![])
            }
            EKind::Int(x) => {
                target = self.next_vr();
                (format!("%{target} = int({x})"), vec![])
            }
            EKind::Bool(x) => {
                target = self.next_vr();
                (format!("%{target} = bool({x})"), vec![])
            }
            EKind::Nothing => panic!("Can't emit nothing!"),
            EKind::Str(items) => todo!(),
            EKind::Call { callee, args } => {
                target = self.next_vr();
                let mut nodes: Vec<_> = args.iter().map(|arg| self.emit_expr(arg)).collect();
                let call_instr = match &callee.kind {
                    EKind::Symbol(symbol) => {
                        format!("%{target} = call F{}, {}", symbol.name, args.len())
                    }
                    _ => {
                        let callee = self.emit_expr(callee);
                        let program =
                            format!("%{target} = icall %{}, {}", callee.output, args.len());
                        nodes.push(callee);
                        program
                    }
                };
                let program = nodes
                    .iter()
                    .map(|param| format!("param %{}\n\t", param.output))
                    .collect::<Vec<_>>()
                    .join("")
                    + &call_instr;
                (program, vec![])
            }
            EKind::Unary { op, rhs } => todo!(),
            EKind::Bin { op, lhs, rhs } => {
                let mut lhs = self.emit_expr(lhs);
                let mut rhs = self.emit_expr(rhs);
                match op {
                    BinOp::Assign => {
                        target = lhs.output;
                        (format!("%{target} = %{}", rhs.output), vec![lhs, rhs])
                    }
                    op => {
                        let op_str = match op {
                            BinOp::Add => "add",
                            BinOp::Sub => "sub",
                            BinOp::Mul => "mul",
                            BinOp::Div => "div",
                            BinOp::LogOr => "or",
                            BinOp::LogAnd => "and",
                            BinOp::Gt => "gt",
                            BinOp::Ge => "ge",
                            BinOp::Lt => "lt",
                            BinOp::Le => "le",
                            BinOp::Eq => "eq",
                            BinOp::Ne => "ne",
                            _ => unreachable!(),
                        };
                        target = self.next_vr();
                        (
                            format!("%{target} = {op_str} %{}, %{}", lhs.output, rhs.output),
                            vec![lhs, rhs],
                        )
                    }
                }
            }
            EKind::FieldAccess { lhs, rhs } => todo!(),
            EKind::Index { lhs, rhs } => todo!(),
        };
        self.emit_merged_node(nodes, template, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser, resolver::Resolver, types::*};

    #[test]
    fn test_emitter() {
        let src = br#"
fn bob(a: i32) {
    let y = 69;
}
fn main(argc: u8, argv: **u8) {
    let x = 5;
    x = x + 5;
    let y = 59;
    bob(y);
}

"#;
        println!("Source code:\n{}\n", str::from_utf8(src).unwrap());
        let tokens: Vec<TKind> = Lexer::new(src).map(|t| t.kind).collect();
        let parsed = Parser::new(&tokens).parse_program();
        let resolved = Resolver::new().resolve_program(&parsed);
        let program = Codegen::new().generate(&resolved);
        let generator = println!("Program:\n{}", program);
    }
}
