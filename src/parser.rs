use super::errors::{NotloxError::*, Result};
use super::scanner;
use super::scanner::TokenType;

struct Parser {
    scanner: scanner::Scanner,
    previous: Option<scanner::Token>,
    next: scanner::Token,
}

#[derive(Debug)]
pub enum Literal {
    Number(f64),
    String(String),
    False,
    True,
    Nil,
}

#[derive(Debug)]
pub struct Unary {
    pub operator: scanner::Token,
    pub expression: Box<Expression>,
}

#[derive(Debug)]
pub struct Binary {
    pub left: Box<Expression>,
    pub operator: scanner::Token,
    pub right: Box<Expression>,
}

#[derive(Debug)]
pub struct Grouping {
    pub expression: Box<Expression>,
}

#[derive(Debug)]
pub struct Variable {
    pub name: String,
}

#[derive(Debug)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub expression: Option<Box<Expression>>,
}

#[derive(Debug)]
pub struct Call {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
}

#[derive(Debug)]
pub enum Expression {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
    Grouping(Grouping),
    Variable(Variable),
    Block(Block),
    Call(Call),
}

#[derive(Debug)]
pub struct ExpressionStatement {
    pub expression: Expression,
}

#[derive(Debug)]
pub struct PrintStatement {
    pub value: Expression,
}

#[derive(Debug)]
pub struct LetStatement {
    pub name: String,
    pub initializer: Option<Expression>,
}

#[derive(Debug)]
pub struct FnStatement {
    pub name: String,
    pub args: Vec<String>,
    pub block: Block,
}

#[derive(Debug)]
pub enum Statement {
    ExpressionStatement(ExpressionStatement),
    LetStatement(LetStatement),
    PrintStatement(PrintStatement),
    FnStatement(FnStatement),
}

#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Parser {
    fn new(source: &str) -> Result<Self> {
        let mut scanner = scanner::Scanner::new(source);
        let first = scanner.scan_token()?;
        Ok(Self {
            scanner,
            previous: None,
            next: first,
        })
    }

    fn statement(&mut self) -> Result<Statement> {
        if self.matches(&[TokenType::Print])? {
            return self.print_statement();
        }
        if self.matches(&[TokenType::Let])? {
            return self.let_statement();
        }
        if self.matches(&[TokenType::Fn])? {
            return self.fn_statement();
        }

        return self.expression_statement();
    }

    fn let_statement(&mut self) -> Result<Statement> {
        let name = self.consume(TokenType::Identifier, "Expect variable name.")?;

        let mut initializer = None;
        if self.matches(&[TokenType::Equal])? {
            initializer = Some(self.expression()?);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;
        let name = self.scanner.get_lexeme(&name);
        return Ok(Statement::LetStatement(LetStatement { name, initializer }));
    }

    fn print_statement(&mut self) -> Result<Statement> {
        let value = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        return Ok(Statement::PrintStatement(PrintStatement { value }));
    }

    fn fn_statement(&mut self) -> Result<Statement> {
        let name = self.consume(TokenType::Identifier, "Expected function name.")?;
        let name = self.scanner.get_lexeme(&name);

        self.consume(TokenType::LeftParen, "Expected '(' for fn arg list")?;
        let mut args = Vec::new();
        if self.matches(&[TokenType::Identifier])? {
            let arg_name = self.previous();
            args.push(self.scanner.get_lexeme(&arg_name));
            // Todo: Technically this loop will accept extra commas (as well as trailing, which is intended). Fine for now, maybe worth fixing at some point.
            while self.matches(&[TokenType::Comma])? {
                if self.matches(&[TokenType::Identifier])? {
                    let arg_name = self.previous();
                    args.push(self.scanner.get_lexeme(&arg_name));
                }
            }
        }
        self.consume(TokenType::RightParen, "Expected ')' for fn arg list")?;

        let block = self.block()?;

        return Ok(Statement::FnStatement(FnStatement { name, args, block }));
    }

    fn expression_statement(&mut self) -> Result<Statement> {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        return Ok(Statement::ExpressionStatement(ExpressionStatement {
            expression,
        }));
    }

    fn expression(&mut self) -> Result<Expression> {
        if self.peek().token_type == TokenType::LeftBrace {
            return Ok(Expression::Block(self.block()?));
        }
        return self.equality();
    }

    fn block(&mut self) -> Result<Block> {
        self.consume(TokenType::LeftBrace, "Expected '{' to start block.")?;
        let mut statements = Vec::new();
        let mut expression = None;
        while self.peek().token_type != TokenType::RightBrace {
            match self.peek().token_type {
                TokenType::Let => {
                    self.consume(TokenType::Let, "This should never happen.")?;
                    statements.push(self.let_statement()?);
                }
                TokenType::Print => {
                    self.consume(TokenType::Print, "This should never happen.")?;
                    statements.push(self.print_statement()?);
                }
                TokenType::Fn => {
                    self.consume(TokenType::Fn, "This should never happen.")?;
                    statements.push(self.fn_statement()?);
                }
                _ => {
                    let found_expression = self.expression()?;
                    if self.matches(&[TokenType::Semicolon])? {
                        statements.push(Statement::ExpressionStatement(ExpressionStatement {
                            expression: found_expression,
                        }))
                    } else {
                        expression = Some(Box::new(found_expression));
                        break;
                    }
                }
            }
        }
        self.consume(TokenType::RightBrace, "Expected '}' to end block.")?;
        return Ok(Block {
            statements,
            expression,
        });
    }

    fn equality(&mut self) -> Result<Expression> {
        let mut expr = self.comparison()?;
        while self.matches(&[TokenType::BangEqual, TokenType::EqualEqual])? {
            let operator = self.previous();
            let right = self.comparison()?;
            expr = Expression::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }
        return Ok(expr);
    }

    fn comparison(&mut self) -> Result<Expression> {
        let mut expr = self.addition()?;
        while self.matches(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ])? {
            let operator = self.previous();
            let right = self.addition()?;
            expr = Expression::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }
        return Ok(expr);
    }

    fn addition(&mut self) -> Result<Expression> {
        let mut expr = self.multiplication()?;
        while self.matches(&[TokenType::Plus, TokenType::Minus])? {
            let operator = self.previous();
            let right = self.multiplication()?;
            expr = Expression::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }
        return Ok(expr);
    }

    fn multiplication(&mut self) -> Result<Expression> {
        let mut expr = self.unary()?;
        while self.matches(&[TokenType::Slash, TokenType::Star])? {
            let operator = self.previous();
            let right = self.unary()?;
            expr = Expression::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }
        return Ok(expr);
    }

    fn unary(&mut self) -> Result<Expression> {
        if self.matches(&[TokenType::Bang, TokenType::Minus])? {
            let operator = self.previous();
            let expression = self.unary()?;
            return Ok(Expression::Unary(Unary {
                operator,
                expression: Box::new(expression),
            }));
        }
        return self.call();
    }

    fn call(&mut self) -> Result<Expression> {
        let mut expression = self.primary();

        loop {
            if self.matches(&[TokenType::LeftParen])? {
                expression = self.finish_call(expression?);
            } else {
                break;
            }
        }

        return expression;
    }

    fn finish_call(&mut self, callee: Expression) -> Result<Expression> {
        let mut args = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                args.push(self.expression()?);
                if !self.matches(&[TokenType::Comma])? {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after arguments.")?;

        return Ok(Expression::Call(Call {
            callee: Box::new(callee),
            args,
        }));
    }

    fn primary(&mut self) -> Result<Expression> {
        if self.matches(&[TokenType::False])? {
            return Ok(Expression::Literal(Literal::False));
        }
        if self.matches(&[TokenType::True])? {
            return Ok(Expression::Literal(Literal::True));
        }
        if self.matches(&[TokenType::Nil])? {
            return Ok(Expression::Literal(Literal::Nil));
        }
        if self.matches(&[TokenType::Number])? {
            let t = self.previous();
            let s = self.scanner.get_lexeme(&t);
            return match s.parse::<f64>() {
                Ok(f) => Ok(Expression::Literal(Literal::Number(f))),
                Err(_) => Err(ParserError("Invalid number literal".to_string())),
            };
        }
        if self.matches(&[TokenType::String])? {
            let t = self.previous();
            let s = self.scanner.get_lexeme(&t);
            let s = &s[1..s.len() - 1];
            return Ok(Expression::Literal(Literal::String(s.to_string())));
        }
        if self.matches(&[TokenType::Identifier])? {
            let t = self.previous();
            let name = self.scanner.get_lexeme(&t);
            return Ok(Expression::Variable(Variable { name }));
        }
        if self.matches(&[TokenType::LeftParen])? {
            let expression = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
            return Ok(Expression::Grouping(Grouping {
                expression: Box::new(expression),
            }));
        }
        return Err(ParserError("Expect expression".to_string()));
    }

    fn matches(&mut self, types: &[TokenType]) -> Result<bool> {
        for t in types {
            if self.check(*t) {
                self.advance()?;
                return Ok(true);
            }
        }
        return Ok(false);
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        return self.peek().token_type == token_type;
    }

    fn advance(&mut self) -> Result<scanner::Token> {
        if !self.is_at_end() {
            self.previous = Some(self.next);
            self.next = self.scanner.scan_token()?;
        }
        return Ok(self.previous());
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Result<scanner::Token> {
        if self.check(token_type) {
            return self.advance();
        }
        Err(ParserError(message.to_string()))
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::EOF
    }

    fn peek(&self) -> &scanner::Token {
        &self.next
    }

    fn previous(&self) -> scanner::Token {
        self.previous.unwrap()
    }
}

pub fn parse(source: &str) -> Result<Program> {
    /*
    let mut scanner = scanner::Scanner::new(source);
    let mut line = std::usize::MAX;
    loop {
        let token = scanner.scan_token()?;
        if token.line != line {
            print!("{:4} ", token.line);
            line = token.line;
        } else {
            print!("   | ");
        }
        println!(
            "{:?} '{}'",
            token.token_type,
            &source[token.start..token.start + token.length]
        );

        if token.token_type == scanner::TokenType::EOF {
            return Ok(());
        }
    }
     */
    let mut parser = Parser::new(source)?;
    let mut statements = Vec::new();
    while !parser.is_at_end() {
        statements.push(parser.statement()?);
    }
    let out = Program { statements };
    println!("{:?}", out);
    return Ok(out);
}
