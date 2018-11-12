use super::errors::{NotloxError::*, Result};

// One hack here: The kw_map is a mapping from keyword string to
// TokenType. It's really just static/compile time data. We create it
// as a hash map on scanner construction for convenience. It could be
// a trie (as in Lox book) or a PHF style static map or something.
pub struct Scanner {
    source: Vec<char>,
    start: usize,
    current: usize,
    line: usize,
    kw_map: std::collections::HashMap<String, TokenType>,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum TokenType {
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Colon,
    Slash,
    Star,

    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    HashLeftBrace,

    Identifier,
    String,
    Number,

    And,
    Else,
    False,
    Fn,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    True,
    Let,
    While,

    EOF,
}

//impl Display for TokenType

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
}

impl Scanner {
    pub fn new(source: &str) -> Scanner {
        let mut kw_map = std::collections::HashMap::new();
        kw_map.insert("and".to_string(), TokenType::And);
        kw_map.insert("else".to_string(), TokenType::Else);
        kw_map.insert("false".to_string(), TokenType::False);
        kw_map.insert("for".to_string(), TokenType::For);
        kw_map.insert("fn".to_string(), TokenType::Fn);
        kw_map.insert("if".to_string(), TokenType::If);
        kw_map.insert("nil".to_string(), TokenType::Nil);
        kw_map.insert("or".to_string(), TokenType::Or);
        kw_map.insert("print".to_string(), TokenType::Print);
        kw_map.insert("return".to_string(), TokenType::Return);
        kw_map.insert("true".to_string(), TokenType::True);
        kw_map.insert("let".to_string(), TokenType::Let);
        kw_map.insert("while".to_string(), TokenType::While);

        Scanner {
            source: source.chars().collect(),
            start: 0,
            current: 0,
            line: 1,
            kw_map,
        }
    }

    pub fn scan_token(&mut self) -> Result<Token> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return Ok(self.make_token(TokenType::EOF));
        }

        let c = self.advance();
        match c {
            '(' => Ok(self.make_token(TokenType::LeftParen)),
            ')' => Ok(self.make_token(TokenType::RightParen)),
            '{' => Ok(self.make_token(TokenType::LeftBrace)),
            '}' => Ok(self.make_token(TokenType::RightBrace)),
            ';' => Ok(self.make_token(TokenType::Semicolon)),
            ',' => Ok(self.make_token(TokenType::Comma)),
            '.' => Ok(self.make_token(TokenType::Dot)),
            '-' => Ok(self.make_token(TokenType::Minus)),
            '+' => Ok(self.make_token(TokenType::Plus)),
            '/' => Ok(self.make_token(TokenType::Slash)),
            '*' => Ok(self.make_token(TokenType::Star)),

            '!' => {
                let token_type = if self.token_match('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                Ok(self.make_token(token_type))
            }
            '=' => {
                let token_type = if self.token_match('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                Ok(self.make_token(token_type))
            }
            '<' => {
                let token_type = if self.token_match('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                Ok(self.make_token(token_type))
            }
            '>' => {
                let token_type = if self.token_match('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                Ok(self.make_token(token_type))
            }
            '#' => {
                if self.token_match('{') {
                    Ok(self.make_token(TokenType::HashLeftBrace))
                } else {
                    Err(ScannerError(
                        "Unexpected character: # without {.".to_string(),
                    ))
                }
            }

            '"' => self.string(),

            n if is_digit(n) => self.number(),
            a if is_alpha(a) => self.identifier(),

            _ => Err(ScannerError("Unexpected character.".to_string())),
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn peek(&self) -> char {
        self.source[self.current]
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        return self.source[self.current + 1];
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source[self.current - 1]
    }

    fn token_match(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.source[self.current] != expected {
            return false;
        }
        self.current += 1;
        return true;
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let c = self.peek();
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            };
        }
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            token_type: token_type,
            start: self.start,
            length: self.current - self.start,
            line: self.line,
        }
    }

    fn string(&mut self) -> Result<Token> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(ScannerError("Unterminated string.".to_string()));
        }
        self.advance();

        return Ok(self.make_token(TokenType::String));
    }

    fn number(&mut self) -> Result<Token> {
        while is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == '.' && is_digit(self.peek_next()) {
            self.advance();
            while is_digit(self.peek()) {
                self.advance();
            }
        }

        return Ok(self.make_token(TokenType::Number));
    }

    fn identifier(&mut self) -> Result<Token> {
        while is_alpha(self.peek()) || is_digit(self.peek()) {
            self.advance();
        }
        return Ok(self.make_token(self.identifier_type()));
    }

    fn identifier_type(&self) -> TokenType {
        let name: String = self.source[self.start..self.current].into_iter().collect();
        *self.kw_map.get(&name).unwrap_or(&TokenType::Identifier)
    }

    pub fn get_lexeme(&self, token: &Token) -> String {
        self.source[token.start..token.start + token.length]
            .into_iter()
            .collect()
    }
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}
