use std::fmt::Display;

use crate::lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type {
    I32,
    Bool,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::Bool => write!(f, "bool"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    NotEq,
    Gt,
    GtEq,
    Lt,
    LtEq,
    And,
    Or,
}

#[derive(Debug)]
pub enum Expr {
    Int(u64, Span),
    Bool(bool, Span),
    Var(String, Span),
    Neg(Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
    Call {
        path: Vec<String>,
        args: Vec<Expr>,
        /// The type of each argument. The parser leaves this empty,
        /// and the checker fills it in.
        arg_types: Vec<Type>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, span)
            | Expr::Bool(_, span)
            | Expr::Var(_, span)
            | Expr::Neg(_, span)
            | Expr::Not(_, span)
            | Expr::Binary(_, _, _, span)
            | Expr::Call { span, .. } => *span,
        }
    }
}

#[derive(Debug)]
pub enum Stmt {
    Let {
        name: String,
        mutable: bool,
        /// The type of each argument. The parser leaves this empty,
        /// and the checker fills it in.
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    If {
        cond: Expr,
        then: Vec<Stmt>,
        otherwise: Option<Vec<Stmt>>,
    },
    Expr(Expr),
}

#[derive(Debug)]
pub struct Program {
    pub imports: Vec<String>,
    pub body: Vec<Stmt>,
}
