// The checker finds mistakes the parser cannot see

use std::collections::HashMap;

use crate::ast::*;
use crate::lexer::{Error, Span};

struct Variable {
    ty: Type,
    mutable: bool,
}

struct Checker {
    /// One map per open block, innermost last.
    scopes: Vec<HashMap<String, Variable>>,
    imports_fmt: bool,
}

pub fn check(program: &mut Program) -> Result<(), Error> {
    let mut checker = Checker {
        scopes: Vec::new(),
        imports_fmt: program.imports.iter().any(|i| i == "fmt"),
    };
    checker.block(&mut program.body)
}

/// The error for a value of the wrong type.
fn mismatch(want: Type, found: Type, span: Span) -> Error {
    Error {
        message: format!("expected `{want}`, found `{found}`"),
        span,
    }
}

impl Checker {
    /// A block's names disappear at its closing `}`.
    fn block(&mut self, stmts: &mut [Stmt]) -> Result<(), Error> {
        self.scopes.push(HashMap::new());
        for stmt in stmts {
            self.stmt(stmt)?;
        }
        self.scopes.pop();
        Ok(())
    }

    /// Finds a name, starting from the innermost block.
    fn lookup(&self, name: &str) -> Option<&Variable> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn stmt(&mut self, stmt: &mut Stmt) -> Result<(), Error> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
                span,
            } => {
                let found = self.expr(value)?;
                if let Some(want) = *ty {
                    if found != want {
                        return Err(mismatch(want, found, value.span()));
                    }
                }
                *ty = Some(found);
                // Only the innermost block counts, so an inner block may
                // reuse an outer name.
                let scope = self.scopes.last_mut().unwrap();
                if scope.contains_key(name) {
                    return Err(Error {
                        message: format!("`{name}` is already declared in this block"),
                        span: *span,
                    });
                }
                scope.insert(
                    name.clone(),
                    Variable {
                        ty: found,
                        mutable: *mutable,
                    },
                );
            }
            Stmt::Assign { name, value, span } => {
                let found = self.expr(value)?;
                let Some(variable) = self.lookup(name) else {
                    return Err(Error {
                        message: format!("unknown name `{name}`"),
                        span: *span,
                    });
                };
                if !variable.mutable {
                    return Err(Error {
                        message: format!(
                            "`{name}` is a constant; declare it with `var` to change it"
                        ),
                        span: *span,
                    });
                }
                if found != variable.ty {
                    return Err(mismatch(variable.ty, found, value.span()));
                }
            }
            Stmt::If {
                cond,
                then,
                otherwise,
            } => {
                // There is no truthiness: `if (n)` must be `if (n != 0)`.
                let found = self.expr(cond)?;
                if found != Type::Bool {
                    return Err(mismatch(Type::Bool, found, cond.span()));
                }
                self.block(then)?;
                if let Some(otherwise) = otherwise {
                    self.block(otherwise)?;
                }
            }
            Stmt::Expr(Expr::Call {
                path,
                args,
                arg_types,
                span,
            }) => {
                self.call(path, *span)?;
                for arg in args.iter() {
                    arg_types.push(self.expr(arg)?);
                }
            }
            Stmt::Expr(expr) => {
                self.expr(expr)?;
            }
        }
        Ok(())
    }

    /// Checks an expression and returns its type.
    fn expr(&self, expr: &Expr) -> Result<Type, Error> {
        match expr {
            Expr::Int(value, span) => {
                if *value > i32::MAX as u64 {
                    return Err(Error {
                        message: "number does not fit in `i32`".to_string(),
                        span: *span,
                    });
                }
                Ok(Type::I32)
            }
            Expr::Bool(..) => Ok(Type::Bool),
            Expr::Var(name, span) => match self.lookup(name) {
                Some(variable) => Ok(variable.ty),
                None => Err(Error {
                    message: format!("unknown name `{name}`"),
                    span: *span,
                }),
            },
            Expr::Neg(inner, _) => {
                // `-2147483648` fits in `i32`, even though `2147483648` alone does not.
                if let Expr::Int(value, span) = inner.as_ref() {
                    if *value > 2147483648 {
                        return Err(Error {
                            message: "number does not fit in `i32`".to_string(),
                            span: *span,
                        });
                    }
                    return Ok(Type::I32);
                }
                self.expect(inner, Type::I32)?;
                Ok(Type::I32)
            }
            Expr::Not(inner, _) => {
                self.expect(inner, Type::Bool)?;
                Ok(Type::Bool)
            }
            Expr::Binary(op, left, right, _) => {
                let (operand, result) = match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                        (Type::I32, Type::I32)
                    }
                    BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => (Type::I32, Type::Bool),
                    BinOp::And | BinOp::Or => (Type::Bool, Type::Bool),
                    // `==` and `!=` work on either type, if both sides match.
                    BinOp::Eq | BinOp::NotEq => (self.expr(left)?, Type::Bool),
                };
                self.expect(left, operand)?;
                self.expect(right, operand)?;
                Ok(result)
            }
            Expr::Call { path, span, .. } => {
                // `fmt.println` gives no value, so it can only be a statement.
                self.call(path, *span)?;
                Err(Error {
                    message: format!("`{}` has no value to use", path.join(".")),
                    span: *span,
                })
            }
        }
    }

    /// Checks an expression that must have the type `want`.
    fn expect(&self, expr: &Expr, want: Type) -> Result<(), Error> {
        let found = self.expr(expr)?;
        if found != want {
            return Err(mismatch(want, found, expr.span()));
        }
        Ok(())
    }

    fn call(&self, path: &[String], span: Span) -> Result<(), Error> {
        if path.join(".") != "fmt.println" {
            return Err(Error {
                message: format!("unknown function `{}`", path.join(".")),
                span,
            });
        }
        if !self.imports_fmt {
            return Err(Error {
                message: "`fmt` is used but not imported; add `import \"fmt\"`".to_string(),
                span,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::Parser;

    /// Checks `main`'s body, and returns the error message, if any.
    fn error(body: &str) -> String {
        let source = format!("import \"fmt\"\nfn main() {{\n{body}\n}}\n");
        let mut program = Parser::new(lex(&source).unwrap()).program().unwrap();
        match check(&mut program) {
            Ok(()) => "no error".to_string(),
            Err(error) => error.message,
        }
    }

    #[test]
    fn types_must_match() {
        assert_eq!(error("const a = 1 + true"), "expected `i32`, found `bool`");
        assert_eq!(error("const a = !1"), "expected `bool`, found `i32`");
        assert_eq!(error("const a = 1 < 2 < 3"), "expected `i32`, found `bool`");
        assert_eq!(error("const a = 1 == true"), "expected `i32`, found `bool`");
        assert_eq!(error("const a: bool = 1"), "expected `bool`, found `i32`");
        assert_eq!(
            error("var a = 1\na = false"),
            "expected `i32`, found `bool`"
        );
        assert_eq!(error("if (1) {\n}"), "expected `bool`, found `i32`");
        assert_eq!(error("const a = true == false && 1 != 2"), "no error");
    }

    #[test]
    fn blocks_have_scopes() {
        assert_eq!(
            error("if (true) {\nconst a = 1\n}\nfmt.println(a)"),
            "unknown name `a`"
        );
        assert_eq!(
            error("const a = 1\nif (true) {\nconst a = true\n}"),
            "no error"
        );
        assert_eq!(
            error("const a = 1\nconst a = 2"),
            "`a` is already declared in this block"
        );
        assert_eq!(
            error("const a = 1\nif (true) {\na = 2\n}"),
            "`a` is a constant; declare it with `var` to change it"
        );
    }
}
