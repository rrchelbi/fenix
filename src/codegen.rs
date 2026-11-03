use std::collections::HashMap;

use crate::ast::*;

// Division in C is undefined for a zero divisor, and for the lowest value
// divided by -1. These helpers give both a defined result.
const RUNTIME: &str = r#"#include <stdbool.h>
#include <stdint.h>
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
    let mut generator = Generator {
        out: String::from(RUNTIME),
        indent: 1,
        scopes: Vec::new(),
        declared: HashMap::new(),
    };
    generator.out.push_str("\nint main(void) {\n");
    generator.block(&program.body);
    generator.out.push_str("    return 0;\n}\n");
    generator.out
}

struct Generator {
    out: String,
    indent: usize,
    /// One map per open block, from each Fenix name to its C name.
    scopes: Vec<HashMap<String, String>>,
    /// How many times each Fenix name has been declared so far.
    declared: HashMap<String, usize>,
}

fn c_type(ty: Type) -> &'static str {
    match ty {
        Type::I32 => "int32_t",
        Type::Bool => "bool",
    }
}

impl Generator {
    fn line(&mut self, text: &str) {
        self.out.push_str(&"    ".repeat(self.indent));
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn block(&mut self, stmts: &[Stmt]) {
        self.scopes.push(HashMap::new());
        for stmt in stmts {
            self.stmt(stmt);
        }
        self.scopes.pop();
    }

    /// A fresh C name for a new Fenix variable. Each declaration gets its
    /// own, because in C `int32_t x = x + 1;` reads the new `x`, not the
    /// outer one.
    fn declare(&mut self, name: &str) -> String {
        let count = self.declared.entry(name.to_string()).or_insert(0);
        *count += 1;
        // Fenix names get a prefix, so a variable called `int` or `printf` is safe.
        let c_name = match *count {
            1 => format!("fx_{name}"),
            n => format!("fx_{name}_{n}"),
        };
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), c_name.clone());
        c_name
    }

    fn var(&self, name: &str) -> String {
        let found = self.scopes.iter().rev().find_map(|scope| scope.get(name));
        found.expect("the checker found every name").clone()
    }

    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name, ty, value, ..
            } => {
                // Read the value before declaring, so it sees the outer names.
                let code = self.expr(value);
                let ty = c_type(ty.expect("the checker sets every type"));
                let c_name = self.declare(name);
                self.line(&format!("{ty} {c_name} = {code};"));
            }
            Stmt::Assign { name, value, .. } => {
                let code = format!("{} = {};", self.var(name), self.expr(value));
                self.line(&code);
            }
            Stmt::If {
                cond,
                then,
                otherwise,
            } => {
                let code = format!("if ({}) {{", self.expr(cond));
                self.line(&code);
                self.if_rest(then, otherwise.as_deref());
            }
            Stmt::Expr(Expr::Call {
                args, arg_types, ..
            }) => {
                // `fmt.println` prints its values separated by spaces.
                let mut format = Vec::new();
                let mut values = String::new();
                for (arg, ty) in args.iter().zip(arg_types) {
                    let code = self.expr(arg);
                    match ty {
                        Type::I32 => {
                            format.push("%d");
                            values.push_str(&format!(", {code}"));
                        }
                        Type::Bool => {
                            format.push("%s");
                            values.push_str(&format!(", ({code} ? \"true\" : \"false\")"));
                        }
                    }
                }
                let code = format!("printf(\"{}\\n\"{values});", format.join(" "));
                self.line(&code);
            }
            Stmt::Expr(expr) => {
                let code = format!("(void)({});", self.expr(expr));
                self.line(&code);
            }
        }
    }

    /// Writes an `if`'s body, then its `else`, after the `if (...) {` line.
    fn if_rest(&mut self, then: &[Stmt], otherwise: Option<&[Stmt]>) {
        self.indent += 1;
        self.block(then);
        self.indent -= 1;
        match otherwise {
            None => self.line("}"),
            // `else if` stays flat in C, rather than nesting deeper each time.
            Some(
                [
                    Stmt::If {
                        cond,
                        then,
                        otherwise,
                    },
                ],
            ) => {
                let code = format!("}} else if ({}) {{", self.expr(cond));
                self.line(&code);
                self.if_rest(then, otherwise.as_deref());
            }
            Some(otherwise) => {
                self.line("} else {");
                self.indent += 1;
                self.block(otherwise);
                self.indent -= 1;
                self.line("}");
            }
        }
    }

    fn expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Int(value, _) => format!("((int32_t){value})"),
            Expr::Bool(value, _) => value.to_string(),
            Expr::Var(name, _) => self.var(name),
            Expr::Neg(inner, _) => {
                // Written this way so `-2147483648` is valid C.
                if let Expr::Int(value, _) = inner.as_ref() {
                    return format!("((int32_t)(0u - {value}u))");
                }
                format!("(-{})", self.expr(inner))
            }
            Expr::Not(inner, _) => format!("(!{})", self.expr(inner)),
            Expr::Binary(op, left, right, _) => {
                let (l, r) = (self.expr(left), self.expr(right));
                // C's `&&` and `||` skip the right side just as Fenix's do.
                let symbol = match op {
                    BinOp::Div => return format!("fx_div({l}, {r})"),
                    BinOp::Rem => return format!("fx_rem({l}, {r})"),
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Eq => "==",
                    BinOp::NotEq => "!=",
                    BinOp::Lt => "<",
                    BinOp::LtEq => "<=",
                    BinOp::Gt => ">",
                    BinOp::GtEq => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                };
                format!("({l} {symbol} {r})")
            }
            Expr::Call { .. } => unreachable!("the checker allows calls only as statements"),
        }
    }
}
