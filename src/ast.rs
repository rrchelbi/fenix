use crate::lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

#[derive(Debug)]
pub enum Expr {
    Int(u64, Span),
    Var(String, Span),
    Neg(Box<Expr>, Span),
    Binary(BinOp, Box<Expr>, Box<Expr>, Span),
    Call(Vec<String>, Vec<Expr>, Span),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, span)
            | Expr::Var(_, span)
            | Expr::Neg(_, span)
            | Expr::Binary(_, _, _, span)
            | Expr::Call(_, _, span) => *span,
        }
    }
}

#[derive(Debug)]
pub enum Stmt {
    Let {
        name: String,
        mutable: bool,
        value: Expr,
        span: Span,
    },
    Assign {
        name: String,
        value: Expr,
        span: Span,
    },
    Expr(Expr),
}

#[derive(Debug)]
pub struct Program {
    pub imports: Vec<String>,
    pub body: Vec<Stmt>,
}
