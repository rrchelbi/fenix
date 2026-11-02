use crate::ast::*;
use crate::lexer::{Error, Kind, Span, Token};

/// How a token looks in source, for error messages.
fn describe(k: &Kind) -> String {
    let text = match k {
        Kind::Ident(name) => return name.clone(),
        Kind::Int(value) => return value.to_string(),
        Kind::Str(text) => return format!("\"{text}\""),
        Kind::Import => "import",
        Kind::Fn => "fn",
        Kind::Var => "var",
        Kind::Const => "const",
        Kind::Plus => "+",
        Kind::Minus => "-",
        Kind::Star => "*",
        Kind::Slash => "/",
        Kind::Percent => "%",
        Kind::Assign => "=",
        Kind::LParen => "(",
        Kind::RParen => ")",
        Kind::LBrace => "{",
        Kind::RBrace => "}",
        Kind::Colon => ":",
        Kind::Comma => ",",
        Kind::Dot => ".",
        Kind::Semi => ";",
        Kind::Eof => "end of file",
    };
    text.to_string()
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

type PResult<T> = Result<T, Error>;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Kind {
        &self.tokens[self.pos].kind
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn bump(&mut self) {
        if *self.peek() != Kind::Eof {
            self.pos += 1;
        }
    }

    fn error<T>(&self, message: &str) -> PResult<T> {
        let found = match self.peek() {
            Kind::Semi => "end of line".to_string(),
            Kind::Eof => "end of file".to_string(),
            other => format!("`{}`", describe(other)),
        };
        Err(Error {
            message: format!("expected {message}, found {found}"),
            span: self.span(),
        })
    }

    fn expect(&mut self, kind: Kind, what: &str) -> PResult<Span> {
        if *self.peek() == kind {
            let span = self.span();
            self.bump();
            Ok(span)
        } else {
            self.error(what)
        }
    }

    fn ident(&mut self) -> PResult<String> {
        if let Kind::Ident(name) = self.peek() {
            let name = name.clone();
            self.bump();
            Ok(name)
        } else {
            self.error("a name")
        }
    }

    fn skip_semis(&mut self) {
        while *self.peek() == Kind::Semi {
            self.bump();
        }
    }

    /// The span from `start` to the end of the last consumed token.
    fn span_from(&self, start: Span) -> Span {
        let end = self.tokens[self.pos - 1].span.end;
        Span {
            start: start.start,
            end,
        }
    }

    // file = { "import" STRING END } "fn" "main" "(" ")" block
    //
    // A file has no header: it is a module named after the file.

    pub fn program(&mut self) -> PResult<Program> {
        self.skip_semis();

        let mut imports = Vec::new();
        while *self.peek() == Kind::Import {
            self.bump();
            match self.peek().clone() {
                Kind::Str(path) => imports.push(path),
                _ => return self.error("an import path"),
            }
            self.bump();
            self.expect(Kind::Semi, "end of line")?;
            self.skip_semis();
        }

        self.expect(Kind::Fn, "`fn`")?;
        if self.ident()? != "main" {
            return Err(Error {
                message: "this compiler only supports `fn main` so far".to_string(),
                span: self.tokens[self.pos - 1].span,
            });
        }
        self.expect(Kind::LParen, "`(`")?;
        self.expect(Kind::RParen, "`)`")?;
        let body = self.block()?;
        self.skip_semis();
        self.expect(Kind::Eof, "end of file")?;
        Ok(Program { imports, body })
    }

    // block = "{" { stmt END } "}"

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(Kind::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        loop {
            self.skip_semis();
            if *self.peek() == Kind::RBrace {
                self.bump();
                return Ok(stmts);
            }
            stmts.push(self.stmt()?);
            if *self.peek() != Kind::RBrace {
                self.expect(Kind::Semi, "end of statement")?;
            }
        }
    }

    // stmt = ( "var" | "const" ) IDENT [ ":" IDENT ] "=" expr
    //      | IDENT "=" expr
    //      | expr

    fn stmt(&mut self) -> PResult<Stmt> {
        let start = self.span();
        if matches!(self.peek(), Kind::Var | Kind::Const) {
            // `var` can change later, and `const` cannot.
            let mutable = *self.peek() == Kind::Var;
            self.bump();
            let name = self.ident()?;
            if *self.peek() == Kind::Colon {
                self.bump();
                let ty = self.ident()?;
                if ty != "i32" {
                    return Err(Error {
                        message: format!("only `i32` is supported so far, not `{ty}`"),
                        span: self.tokens[self.pos - 1].span,
                    });
                }
            }
            self.expect(Kind::Assign, "`=`")?;
            let value = self.expr()?;
            let span = self.span_from(start);
            return Ok(Stmt::Let {
                name,
                mutable,
                value,
                span,
            });
        }

        let expr = self.expr()?;
        if *self.peek() == Kind::Assign {
            let Expr::Var(name, _) = expr else {
                return Err(Error {
                    message: "only a variable can be assigned to".to_string(),
                    span: expr.span(),
                });
            };
            self.bump();
            let value = self.expr()?;
            let span = self.span_from(start);
            return Ok(Stmt::Assign { name, value, span });
        }
        Ok(Stmt::Expr(expr))
    }

    // Expressions use precedence climbing:
    //   expr  = term { ("+" | "-") term }
    //   term  = unary { ("*" | "/" | "%") unary }
    //   unary = "-" unary | call
    //   call  = primary [ "(" args ")" ]

    fn expr(&mut self) -> PResult<Expr> {
        self.binary(1)
    }

    fn binary(&mut self, min_level: u8) -> PResult<Expr> {
        let mut left = self.unary()?;
        loop {
            let (op, level) = match self.peek() {
                Kind::Plus => (BinOp::Add, 1),
                Kind::Minus => (BinOp::Sub, 1),
                Kind::Star => (BinOp::Mul, 2),
                Kind::Slash => (BinOp::Div, 2),
                Kind::Percent => (BinOp::Rem, 2),
                _ => return Ok(left),
            };
            if level < min_level {
                return Ok(left);
            }
            self.bump();
            // The right side only takes tighter operators, which keeps
            // `a - b - c` as `(a - b) - c`.
            let right = self.binary(level + 1)?;
            let span = Span {
                start: left.span().start,
                end: right.span().end,
            };
            left = Expr::Binary(op, Box::new(left), Box::new(right), span);
        }
    }

    fn unary(&mut self) -> PResult<Expr> {
        if *self.peek() == Kind::Minus {
            let start = self.span();
            self.bump();
            let inner = self.unary()?;
            let span = Span {
                start: start.start,
                end: inner.span().end,
            };
            return Ok(Expr::Neg(Box::new(inner), span));
        }
        self.call()
    }

    fn call(&mut self) -> PResult<Expr> {
        let start = self.span();
        let mut expr = self.primary()?;
        if *self.peek() == Kind::Dot {
            // A dotted name such as `fmt.println`, which must be called.
            let Expr::Var(first, _) = expr else {
                return self.error("a name before `.`");
            };
            let mut path = vec![first];
            while *self.peek() == Kind::Dot {
                self.bump();
                path.push(self.ident()?);
            }
            self.expect(Kind::LParen, "`(`")?;
            let mut args = Vec::new();
            while *self.peek() != Kind::RParen {
                args.push(self.expr()?);
                if *self.peek() != Kind::Comma {
                    break;
                }
                self.bump();
            }
            self.expect(Kind::RParen, "`,` or `)`")?;
            expr = Expr::Call(path, args, self.span_from(start));
        }
        Ok(expr)
    }

    fn primary(&mut self) -> PResult<Expr> {
        let span = self.span();
        match self.peek().clone() {
            Kind::Int(value) => {
                self.bump();
                Ok(Expr::Int(value, span))
            }
            Kind::Ident(name) => {
                self.bump();
                Ok(Expr::Var(name, span))
            }
            Kind::LParen => {
                self.bump();
                let inner = self.expr()?;
                self.expect(Kind::RParen, "`)`")?;
                Ok(inner)
            }
            _ => self.error("an expression"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    /// Parses `main`'s body and prints its first expression as nested text.
    fn shape(body: &str) -> String {
        let source = format!("fn main() {{\n{body}\n}}\n");
        let program = Parser::new(lex(&source).unwrap()).program().unwrap();
        fn show(e: &Expr) -> String {
            match e {
                Expr::Int(v, _) => v.to_string(),
                Expr::Var(name, _) => name.clone(),
                Expr::Neg(inner, _) => format!("(- {})", show(inner)),
                Expr::Binary(op, l, r, _) => format!("({op:?} {} {})", show(l), show(r)),
                Expr::Call(path, args, _) => {
                    let args: Vec<String> = args.iter().map(show).collect();
                    format!("{}({})", path.join("."), args.join(" "))
                }
            }
        }
        match &program.body[0] {
            Stmt::Expr(e) => show(e),
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } => show(value),
        }
    }

    #[test]
    fn precedence() {
        assert_eq!(shape("1 + 2 * 3"), "(Add 1 (Mul 2 3))");
        assert_eq!(shape("(1 + 2) * 3"), "(Mul (Add 1 2) 3)");
        assert_eq!(shape("a - b - c"), "(Sub (Sub a b) c)");
        assert_eq!(shape("-a * b"), "(Mul (- a) b)");
    }

    #[test]
    fn calls() {
        assert_eq!(shape("fmt.println(a, b + 1)"), "fmt.println(a (Add b 1))");
    }
}
