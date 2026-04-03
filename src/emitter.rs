use std::collections::HashMap;

use crate::types::*;

pub struct Emitter<'src> {
    vr_count: VReg,
    if_label: usize,
    loop_label: usize,
    program: Vec<Ir>,
    symbols: HashMap<&'src str, VReg>,
}

type VReg = usize;


pub struct Ir {
    pub kind: IKind,
    pub width: u8,
}

pub enum IKind {
    /* FUNCTIONS */
    GetParam(VReg, usize),
    SetParam(VReg, usize),
    Call(VReg, String, usize),
    Icall(VReg, VReg, usize),
    Ret(Option<VReg>),
    Local(VReg),

    /* DATA */
    Imm(VReg, i64),
    Addr(VReg, usize),
    Load(VReg, VReg),
    Store(VReg, VReg),
    Move(VReg, VReg),

    /* ARITHMETIC */
    Add(VReg, VReg, VReg),
    Sub(VReg, VReg, VReg),
    Mul(VReg, VReg, VReg),
    Sdiv(VReg, VReg, VReg),
    Udiv(VReg, VReg, VReg),

    /* ORDERING */
    Sgt(VReg, VReg, VReg),
    Sge(VReg, VReg, VReg),
    Ugt(VReg, VReg, VReg),
    Uge(VReg, VReg, VReg),
    Slt(VReg, VReg, VReg),
    Sle(VReg, VReg, VReg),
    Ult(VReg, VReg, VReg),
    Ule(VReg, VReg, VReg),

    /* EQUALITY */
    Eq(VReg, VReg, VReg),
    Ne(VReg, VReg, VReg),

    /* LOGIC */
    And(VReg, VReg, VReg),
    Or(VReg, VReg, VReg),
    Xor(VReg, VReg, VReg),

    /* CONTROL FLOW */
    Label(String),
    Beq(VReg, VReg, String),
    Bne(VReg, VReg, String),
    Br(String),
}

impl std::fmt::Display for Ir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !matches!(self.kind, IKind::Label(_)) {
            f.write_str("\t")?;
        }
        self.kind.fmt(f, self.width)
    }
}

impl IKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>, width: u8) -> std::fmt::Result {
        let suffix = match width {
            8 => ".8",
            16 => ".16",
            32 => ".32",
            64 => ".64",
            _ => "",
        };
        #[rustfmt::skip]
        let s = match self {
            /* FUNCTIONS */
            IKind::GetParam(dst, index)   => format!("getp{suffix}\t %{dst}, {index}"),
            IKind::SetParam(src, index)   => format!("setp{suffix}\t %{src}, {index}"),
            IKind::Call(dst, name, args)  => format!("call\t %{dst}, {name}, {args}"),
            IKind::Icall(dst, src, args)  => format!("icall\t %{dst}, %{src}, {args}"),
            IKind::Ret(src)               => format!("ret{suffix}\t {}", src.map(|r| format!("%{r}")).unwrap_or("".to_string())),

            /* DATA */
            IKind::Imm(dst, imm)          => format!("imm{suffix}\t %{dst}, {width}, {imm}"),
            IKind::Addr(dst, index)       => format!("addr\t %{dst}, {index}"),
            IKind::Load(dst, src)         => format!("load{suffix}\t %{dst}, {width}, %{src}"),
            IKind::Store(src2, src1)      => format!("store{suffix}\t %{src1}, {width}, %{src2}"),
            IKind::Move(dst, src)         => format!("move\t %{dst}, %{src}"),
            IKind::Local(dst)             => format!("loc{suffix}\t %{dst}"),

            /* ARITHMETIC */
            IKind::Add(dst, src1, src2)   => format!("add{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Sub(dst, src1, src2)   => format!("sub{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Mul(dst, src1, src2)   => format!("mul{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Sdiv(dst, src1, src2)  => format!("sdiv{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Udiv(dst, src1, src2)  => format!("udiv{suffix}\t %{dst}, %{src1}, %{src2}"),

            /* ORDERING */
            IKind::Sgt(dst, src1, src2)   => format!("sgt{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Sge(dst, src1, src2)   => format!("sge{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Ugt(dst, src1, src2)   => format!("ugt{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Uge(dst, src1, src2)   => format!("uge{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Slt(dst, src1, src2)   => format!("slt{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Sle(dst, src1, src2)   => format!("sle{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Ult(dst, src1, src2)   => format!("ult{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Ule(dst, src1, src2)   => format!("ule{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Eq(dst, src1, src2)    => format!("eq{suffix}\t %{dst}, %{src1}, %{src2}"),
            IKind::Ne(dst, src1, src2)    => format!("ne{suffix}\t %{dst}, %{src1}, %{src2}"),

            /* LOGIC */
            IKind::And(dst, src1, src2)   => format!("and\t %{dst}, %{src1}, %{src2}"),
            IKind::Or(dst, src1, src2)    => format!("or\t %{dst}, %{src1}, %{src2}"),
            IKind::Xor(dst, src1, src2)   => format!("xor\t %{dst}, %{src1}, %{src2}"),

            /* CONTROL FLOW */
            IKind::Label(label)           => format!("{label}:"),
            IKind::Beq(src1, src2, label) => format!("beq{suffix}\t %{src1}, %{src2}, {label}"),
            IKind::Bne(src1, src2, label) => format!("bne{suffix}\t %{src1}, %{src2}, {label}"),
            IKind::Br(label)              => format!("br\t {label}"),
        };
        f.write_str(&s)
    }
}

impl<'src> Emitter<'src> {
    pub fn new() -> Self {
        Self {
            vr_count: 0,
            if_label: 0,
            loop_label: 0,
            program: Vec::new(),
            symbols: HashMap::new(),
        }
    }

    fn emit_inst(&mut self, kind: IKind, width: u8) {
        self.program.push(Ir {
            kind,
            width,
        });
    }

    fn next_label(&mut self, stmt: &Stmt<'src, Type>) -> String {
        match &stmt.kind {
            SKind::While { .. } => {
                self.loop_label += 1;
                format!("L{}", self.loop_label)
            }
            SKind::If { .. } => {
                self.if_label += 1;
                format!("I{}", self.if_label)
            }
            _ => unreachable!(),
        }
    }

    fn next_vr(&mut self) -> VReg {
        self.vr_count += 1;
        self.vr_count
    }

    pub fn emit_program(mut self, objs: &[Object<'src, Type>]) -> Vec<Ir> {
        for obj in objs {
            self.emit_object(obj);
        }
        self.program
    }

    fn emit_object(&mut self, obj: &Object<'src, Type>) {
        match &obj.kind {
            OKind::Fn {
                name,
                returns,
                args,
                locals: _,
                body,
            } => {
                // START FUNCTION
                let label = format!("Fn_{name}");
                self.emit_inst(IKind::Label(label), 0);
                self.symbols.clear();
                self.vr_count = 0;
                for (i, arg) in args.iter().enumerate() {
                    let target = self.next_vr();
                    self.emit_inst(IKind::GetParam(target, i), arg.ty.width());
                    self.symbols.insert(arg.name, target);
                }

                self.emit_stmt(body);

                // END FUNCTION
                if *returns == Type::Void && !matches!(self.program.last().unwrap().kind, IKind::Ret(_)) {
                    self.emit_inst(IKind::Ret(None), 0);
                }
                self.emit_inst(IKind::Label(format!("FnEnd_{name}")), 0);
            }
            OKind::Global { .. } => todo!(),
            OKind::Struct { .. } => todo!(),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt<'src, Type>) {
        match &stmt.kind {
            SKind::Let { lhs, rhs } => {
                let target = self.next_vr();
                if self.symbols.insert(lhs.name, target).is_some() {
                    panic!("Duplicate symbol {:?}", lhs);
                }
                let target = self.next_vr();
                self.emit_inst(IKind::Local(target), lhs.ty.width());
                let rhs_vr = self.emit_expr(rhs);
                self.emit_inst(IKind::Move(target, rhs_vr), 0);
            }
            SKind::While { cond, body } => {
                let label = self.next_label(stmt);
                let start_label = label.clone() + "_loop";
                let end_label = label + "_end";
                let cond_vr = self.emit_expr(cond);
                let target = self.next_vr();
                self.emit_inst(IKind::Imm(target, 0x0), Type::Bool.width());
                self.emit_inst(IKind::Bne(target, cond_vr, end_label.clone()), cond.ty.width());
                self.emit_inst(IKind::Label(start_label.clone()), 0);
                self.emit_stmt(body);
                self.emit_inst(IKind::Br(start_label), 0);
                self.emit_inst(IKind::Label(end_label), 0);
            }
            // Find the current loop label
            SKind::Continue => {
                // Jump to start label
                self.emit_inst(IKind::Br(format!("L{}_loop", self.if_label)), 0);
            }
            SKind::Break => {
                // Jump to end label
                self.emit_inst(IKind::Br(format!("L{}_endloop", self.if_label)), 0);
            }
            SKind::If { cond, then_, else_ } => {
                let label = self.next_label(stmt);
                let then_label = label.clone() + "_then";
                let else_label = label.clone() + "_else";
                let end_label = label + "_end";
                let cond_vr = self.emit_expr(cond);
                let target = self.next_vr();
                self.emit_inst(IKind::Imm(target, 0x0), Type::Bool.width());
                self.emit_inst(IKind::Beq(target, cond_vr, end_label.clone()), cond.ty.width());
                self.emit_inst(IKind::Label(then_label), 0);
                self.emit_stmt(then_);
                if let SKind::Block(v) = &else_.as_ref().kind
                    && !v.is_empty()
                {
                    self.emit_inst(IKind::Br(end_label.clone()), 0);
                    self.emit_inst(IKind::Label(else_label), 0);
                    self.emit_stmt(else_);
                    self.emit_inst(IKind::Br(end_label.clone()), 0);
                }
                self.emit_inst(IKind::Label(end_label), 0);
            }
            SKind::Return(expr) => {
                if expr.ty != Type::Void {
                    let ret = self.emit_expr(expr);
                    self.emit_inst(IKind::Ret(Some(ret)), expr.ty.width());
                } else {
                    self.emit_inst(IKind::Ret(None), 0);
                }
            }
            SKind::Block(stmts) => {
                for stmt in stmts {
                    self.emit_stmt(stmt);
                }
            }
            SKind::Expr(expr) => _ = self.emit_expr(expr),
        }
    }

    fn emit_expr(&mut self, Expr { kind, ty, .. }: &Expr<'src, Type>) -> VReg {
        let target;
        match kind {
            EKind::Symbol(symbol) => {
                target = *self.symbols.get(symbol.name).unwrap();
            }
            EKind::Int(x) => {
                target = self.next_vr();
                self.emit_inst(IKind::Imm(target, *x as i64), ty.width());
            }
            EKind::Bool(x) => {
                target = self.next_vr();
                self.emit_inst(IKind::Imm(target, *x as i64), ty.width());
            }
            EKind::Nothing => {
                target = self.vr_count;
            }
            EKind::Str(_) => todo!(),
            EKind::Call { callee, args } => {
                target = self.next_vr();
                let nodes: Vec<_> = args.iter().map(|arg| self.emit_expr(arg)).collect();
                let call_instr = match &callee.kind {
                    EKind::Symbol(symbol) => {
                        IKind::Call(target, format!("Fn_{}", symbol.name), args.len())
                    }
                    _ => {
                        let callee = self.emit_expr(callee);
                        let program = IKind::Icall(target, callee, args.len());
                        program
                    }
                };
                for (i, arg_vr) in nodes.iter().enumerate() {
                    self.emit_inst(IKind::SetParam(*arg_vr, i),  args[i].ty.width());
                }
                self.emit_inst(call_instr, 0);
            }
            EKind::Unary { op, rhs } => match op {
                UnOp::Negate => {
                    let rhs_vr = self.emit_expr(rhs);
                    let imm = self.next_vr();
                    self.emit_inst(IKind::Imm(imm, 0x1), rhs.ty.width());
                    target = self.next_vr();
                    self.emit_inst(IKind::Mul(target, rhs_vr, imm), rhs.ty.width());
                }
                UnOp::Not => {
                    let rhs_vr = self.emit_expr(rhs);
                    let imm = self.next_vr();
                    self.emit_inst(IKind::Imm(imm, 0x1), rhs.ty.width());
                    target = self.next_vr();
                    self.emit_inst(IKind::Xor(target, rhs_vr, imm), rhs.ty.width());
                }
                UnOp::Deref => {
                    let rhs_vr = self.emit_expr(rhs);
                    target = self.next_vr();
                    self.emit_inst(IKind::Load(target, rhs_vr), rhs.ty.width());
                }
                UnOp::AddrOf => {
                    target = self.next_vr();
                    match &rhs.kind {
                        EKind::Symbol(symbol) => self.emit_inst(IKind::Addr(target, *self.symbols.get(symbol.name).unwrap()), 0),
                        x => panic!("Haven't done {x:?}"),
                    };
                }
            },
            EKind::Bin { op, lhs, rhs } => match op {
                BinOp::Assign => match &lhs.kind {
                    EKind::Unary {
                        op: UnOp::Deref,
                        rhs: lrhs, // Left right hand side (deref's inner)
                    } => {
                        target = self.emit_expr(lrhs); // This will be derived from AddrOf
                        let rhs_vr = self.emit_expr(rhs);
                        self.emit_inst(IKind::Store(target, rhs_vr), rhs.ty.width());
                    }
                    _ => {
                        target = self.emit_expr(lhs);
                        let rhs_vr = self.emit_expr(rhs);
                        self.emit_inst(IKind::Move(target, rhs_vr), 0);
                    }
                },
                op => {
                    let lhs_vr = self.emit_expr(lhs);
                    let rhs_vr = self.emit_expr(rhs);
                    target = self.next_vr();
                    #[rustfmt::skip]
                    let instruction = match op {
                        BinOp::LogOr  => IKind::Or(target, lhs_vr, rhs_vr),
                        BinOp::LogAnd => IKind::And(target, lhs_vr, rhs_vr),
                        BinOp::Add    => IKind::Add(target, lhs_vr, rhs_vr),
                        BinOp::Sub    => IKind::Sub(target, lhs_vr, rhs_vr),
                        BinOp::Mul    => IKind::Mul(target, lhs_vr, rhs_vr),
                        BinOp::Eq     => IKind::Eq(target, lhs_vr, rhs_vr),
                        BinOp::Ne     => IKind::Ne(target, lhs_vr, rhs_vr),
                        // The following instructions have unsigned counterparts
                        BinOp::Div => if lhs.ty.signed() {
                            IKind::Sdiv(target, lhs_vr, rhs_vr)
                        } else { 
                            IKind::Udiv(target, lhs_vr, rhs_vr)
                        }
                        BinOp::Gt => if lhs.ty.signed() {
                            IKind::Sgt(target, lhs_vr, rhs_vr)
                        } else { 
                            IKind::Ugt(target, lhs_vr, rhs_vr)
                        }
                        BinOp::Ge => if lhs.ty.signed() {
                            IKind::Sge(target, lhs_vr, rhs_vr)
                        } else { 
                            IKind::Uge(target, lhs_vr, rhs_vr)
                        }
                        BinOp::Lt => if lhs.ty.signed() {
                            IKind::Slt(target, lhs_vr, rhs_vr)
                        } else { 
                            IKind::Ult(target, lhs_vr, rhs_vr)
                        }
                        BinOp::Le => if lhs.ty.signed() {
                            IKind::Sle(target, lhs_vr, rhs_vr)
                        } else { 
                            IKind::Ule(target, lhs_vr, rhs_vr)
                        }
                        _ => unreachable!(),
                    };
                    self.emit_inst(instruction, rhs.ty.width());
                }
            },
            EKind::FieldAccess { .. } => todo!(),
            EKind::Index { .. } => todo!(),
        }
        target
    }
}
