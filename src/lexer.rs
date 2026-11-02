#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Ident(String),
    Int(u64),
    Str(String),
    // Keywords
    Import,
    Fn,
    Var,
    Const,
    // Operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    Comma,
    Dot,
    // `;`, or a line break that ends a statement
    Semi,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

/// Whether a line break after this token ends the statement, as in Go.
fn ends_statement(tok: &Kind) -> bool {
    matches!(
        tok,
        Kind::Ident(_) | Kind::Int(_) | Kind::Str(_) | Kind::RParen | Kind::RBrace
    )
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let bytes = src.as_bytes();
    let mut tokens: Vec<Token> = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        let c = bytes[pos];
        let start = pos;

        // A line break may end a statement.
        if c == b'\n' {
            if tokens.last().is_some_and(|t| ends_statement(&t.kind)) {
                let span = Span { start, end: start };
                tokens.push(Token {
                    kind: Kind::Semi,
                    span,
                });
            }
            pos += 1;
            continue;
        }
        if c == b' ' || c == b'\t' || c == b'\r' {
            pos += 1;
            continue;
        }
        // A comment runs to the end of the line, but leaves the line break.
        if c == b'/' && bytes.get(pos + 1) == Some(&b'/') {
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        let tok = if c.is_ascii_alphabetic() || c == b'_' {
            while pos < bytes.len() && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                pos += 1;
            }
            match &src[start..pos] {
                "import" => Kind::Import,
                "fn" => Kind::Fn,
                "var" => Kind::Var,
                "const" => Kind::Const,
                name => Kind::Ident(name.to_string()),
            }
        } else if c.is_ascii_digit() {
            while pos < bytes.len() && (bytes[pos].is_ascii_digit() || bytes[pos] == b'_') {
                pos += 1;
            }
            let digits = src[start..pos].replace('_', "");
            match digits.parse::<u64>() {
                Ok(value) => Kind::Int(value),
                Err(_) => {
                    let span = Span { start, end: pos };
                    let message = "number is too large".to_string();
                    return Err(Error { message, span });
                }
            }
        } else if c == b'"' {
            pos += 1;
            while pos < bytes.len() && bytes[pos] != b'"' && bytes[pos] != b'\n' {
                pos += 1;
            }
            if pos >= bytes.len() || bytes[pos] != b'"' {
                let span = Span { start, end: pos };
                let message = "unterminated string".to_string();
                return Err(Error { message, span });
            }
            pos += 1;
            Kind::Str(src[start + 1..pos - 1].to_string())
        } else {
            pos += 1;
            match c {
                b'+' => Kind::Plus,
                b'-' => Kind::Minus,
                b'*' => Kind::Star,
                b'/' => Kind::Slash,
                b'%' => Kind::Percent,
                b'=' => Kind::Assign,
                b'(' => Kind::LParen,
                b')' => Kind::RParen,
                b'{' => Kind::LBrace,
                b'}' => Kind::RBrace,
                b':' => Kind::Colon,
                b',' => Kind::Comma,
                b'.' => Kind::Dot,
                b';' => Kind::Semi,
                _ => {
                    // Step past the whole character, which may be several bytes.
                    let width = src[start..].chars().next().map_or(1, char::len_utf8);
                    let span = Span {
                        start,
                        end: start + width,
                    };
                    let character = &src[start..start + width];
                    let message = format!("unexpected character `{character}`");
                    return Err(Error { message, span });
                }
            }
        };
        tokens.push(Token {
            kind: tok,
            span: Span { start, end: pos },
        });
    }

    // The last line may have no line break.
    if tokens.last().is_some_and(|t| ends_statement(&t.kind)) {
        let span = Span {
            start: pos,
            end: pos,
        };
        tokens.push(Token {
            kind: Kind::Semi,
            span,
        });
    }
    tokens.push(Token {
        kind: Kind::Eof,
        span: Span {
            start: pos,
            end: pos,
        },
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Kind> {
        lex(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn line_breaks_end_statements() {
        assert_eq!(
            kinds("const a = 1\nconst b = a +\n2"),
            [
                Kind::Const,
                Kind::Ident("a".into()),
                Kind::Assign,
                Kind::Int(1),
                Kind::Semi,
                Kind::Const,
                Kind::Ident("b".into()),
                Kind::Assign,
                Kind::Ident("a".into()),
                Kind::Plus,
                Kind::Int(2),
                Kind::Semi,
                Kind::Eof,
            ]
        );
    }

    #[test]
    fn comments_and_separators() {
        assert_eq!(
            kinds("1_000 // a comment\n"),
            [Kind::Int(1000), Kind::Semi, Kind::Eof]
        );
    }
}
