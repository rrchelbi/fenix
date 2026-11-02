// The checker finds mistakes the parser cannot see

use std::collections::HashMap;

use crate::ast::*;
use crate::lexer::{Error, Span};

pub fn check(program: &Program) -> Result<(), Error> {
    // Each variable's name, and whether it was declared with `var`.
    let mut variables: HashMap<String, bool> = HashMap::new();

    for stmt in &program.body {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                span,
            } => {
                check_expr(value, &variables, program)?;
                if variables.contains_key(name) {
                    return Err(Error {
                        message: format!("`{name}` is already declared"),
                        span: *span,
                    });
                }
                variables.insert(name.clone(), *mutable);
            }
            Stmt::Assign { name, value, span } => {
                check_expr(value, &variables, program)?;
                match variables.get(name) {
                    None => {
                        return Err(Error {
                            message: format!("unknown name `{name}`"),
                            span: *span,
                        });
                    }
                    Some(false) => {
                        return Err(Error {
                            message: format!(
                                "`{name}` is a constant; declare it with `var` to change it"
                            ),
                            span: *span,
                        });
                    }
                    Some(true) => {}
                }
            }
            Stmt::Expr(Expr::Call(path, args, span)) => {
                check_call(path, *span, program)?;
                for arg in args {
                    check_expr(arg, &variables, program)?;
                }
            }
            Stmt::Expr(expr) => check_expr(expr, &variables, program)?,
        }
    }
    Ok(())
}

fn check_expr(
    expr: &Expr,
    variables: &HashMap<String, bool>,
    program: &Program,
) -> Result<(), Error> {
    match expr {
        Expr::Int(value, span) => {
            if *value > i32::MAX as u64 {
                return Err(Error {
                    message: "number does not fit in `i32`".to_string(),
                    span: *span,
                });
            }
        }
        Expr::Var(name, span) => {
            if !variables.contains_key(name) {
                return Err(Error {
                    message: format!("unknown name `{name}`"),
                    span: *span,
                });
            }
        }
        Expr::Neg(inner, _) => {
            // `-2147483648` fits in `i32`, even though `2147483648` alone does not.
            if let Expr::Int(value, span) = inner.as_ref() {
                if *value > 2147483648 {
                    return Err(Error {
                        message: "number does not fit in `i32`".to_string(),
                        span: *span,
                    });
                }
                return Ok(());
            }
            check_expr(inner, variables, program)?;
        }
        Expr::Binary(_, left, right, _) => {
            check_expr(left, variables, program)?;
            check_expr(right, variables, program)?;
        }
        Expr::Call(path, _, span) => {
            // `fmt.println` gives no value, so it can only be a statement.
            check_call(path, *span, program)?;
            return Err(Error {
                message: format!("`{}` has no value to use", path.join(".")),
                span: *span,
            });
        }
    }
    Ok(())
}

fn check_call(path: &[String], span: Span, program: &Program) -> Result<(), Error> {
    if path.join(".") != "fmt.println" {
        return Err(Error {
            message: format!("unknown function `{}`", path.join(".")),
            span,
        });
    }
    if !program.imports.iter().any(|i| i == "fmt") {
        return Err(Error {
            message: "`fmt` is used but not imported; add `import \"fmt\"`".to_string(),
            span,
        });
    }
    Ok(())
}
