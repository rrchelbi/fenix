use crate::ast::*;

// Division in C is undefined for a zero divisor, and for the lowest value
// divided by -1. These helpers give both a defined result.
const RUNTIME: &str = r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static void fx_panic(const char *message) {
    fflush(stdout);
    fprintf(stderr, "panic: %s\n", message);
    exit(2);
}

static int32_t fx_div(int32_t a, int32_t b) {
    if (b == 0) fx_panic("integer division by zero");
    if (b == -1) return (int32_t)(0u - (uint32_t)a);
    return a / b;
}

static int32_t fx_rem(int32_t a, int32_t b) {
    if (b == 0) fx_panic("integer division by zero");
    if (b == -1) return 0;
    return a % b;
}
"#;

pub fn generate(program: &Program) -> String {
    let mut out = String::from(RUNTIME);
    out.push_str("\nint main(void) {\n");
    for stmt in &program.body {
        out.push_str("    ");
        out.push_str(&stmt_code(stmt));
        out.push('\n');
    }
    out.push_str("    return 0;\n}\n");
    out
}

/// Fenix names get a prefix, so a variable called `int` or `printf` is safe.
fn var(name: &str) -> String {
    format!("fx_{name}")
}

fn stmt_code(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let { name, value, .. } => format!("int32_t {} = {};", var(name), expr_code(value)),
        Stmt::Assign { name, value, .. } => format!("{} = {};", var(name), expr_code(value)),
        Stmt::Expr(Expr::Call(_, args, _)) => {
            // `fmt.println` prints its values separated by spaces.
            let format = vec!["%d"; args.len()].join(" ");
            let mut code = format!("printf(\"{format}\\n\"");
            for arg in args {
                code.push_str(", ");
                code.push_str(&expr_code(arg));
            }
            code.push_str(");");
            code
        }
        Stmt::Expr(expr) => format!("(void)({});", expr_code(expr)),
    }
}

fn expr_code(expr: &Expr) -> String {
    match expr {
        Expr::Int(value, _) => format!("((int32_t){value})"),
        Expr::Var(name, _) => var(name),
        Expr::Neg(inner, _) => {
            // Written this way so `-2147483648` is valid C.
            if let Expr::Int(value, _) = inner.as_ref() {
                return format!("((int32_t)(0u - {value}u))");
            }
            format!("(-{})", expr_code(inner))
        }
        Expr::Binary(op, left, right, _) => {
            let (l, r) = (expr_code(left), expr_code(right));
            match op {
                BinOp::Add => format!("({l} + {r})"),
                BinOp::Sub => format!("({l} - {r})"),
                BinOp::Mul => format!("({l} * {r})"),
                BinOp::Div => format!("fx_div({l}, {r})"),
                BinOp::Rem => format!("fx_rem({l}, {r})"),
            }
        }
        Expr::Call(..) => unreachable!("the checker allows calls only as statements"),
    }
}
