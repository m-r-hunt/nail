use super::errors::{NotloxError::*, Result};
use super::scanner;
use super::scanner::TokenType;

struct Parser {
    scanner: scanner::Scanner,
    previous: Option<scanner::Token>,
    next: scanner::Token,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(f64),
    String(String),
    Char(char),
    False,
    True,
    Nil,
}

#[derive(Debug, Clone)]
pub struct Unary {
    pub operator: scanner::Token,
    pub expression: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Binary {
    pub left: Box<Expression>,
    pub operator: scanner::Token,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Grouping {
    pub expression: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub expression: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct BuiltinCall {
    pub callee: Box<Expression>,
    pub name: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub struct If {
    pub condition: Box<Expression>,
    pub then_block: Block,
    pub else_block: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct While {
    pub condition: Box<Expression>,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct For {
    pub variable: String,
    pub range: Box<Expression>,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub struct Loop {
    pub block: Block,
}

#[derive(Debug, Clone)]
pub enum LValue {
    Variable(Variable),
    Index(Index),
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub lvalue: LValue,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct CompoundAssignment {
    pub lvalue: LValue,
    pub operator: TokenType,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Index {
    pub indexer: Box<Expression>,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Array {
    pub initializers: Vec<Expression>,
}

#[derive(Debug, Clone)]
pub enum MapLHS {
    Name(String),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct MapInitializer {
    pub key: MapLHS,
    pub value: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Map {
    pub initializers: Vec<MapInitializer>,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Return {
    pub value: Option<Box<Expression>>,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Unary(Unary),
    Binary(Binary),
    Grouping(Grouping),
    Variable(Variable),
    Block(Block),
    Call(Call),
    If(If),
    While(While),
    For(For),
    Loop(Loop),
    Assignment(Assignment),
    CompoundAssignment(CompoundAssignment),
    Index(Index),
    Array(Array),
    Map(Map),
    BuiltinCall(BuiltinCall),
    Range(Range),
    Return(Return),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct ExpressionStatement {
    pub expression: Expression,
}

#[derive(Debug, Clone)]
pub struct PrintStatement {
    pub value: Expression,
}

#[derive(Debug, Clone)]
pub struct LetStatement {
    pub name: String,
    pub initializer: Option<Expression>,
}

#[derive(Debug, Clone)]
pub struct FnStatement {
    pub name: String,
    pub args: Vec<String>,
    pub block: Block,
}

#[derive(Debug, Clone)]
pub enum Statement {
    ExpressionStatement(ExpressionStatement),
    LetStatement(LetStatement),
    PrintStatement(PrintStatement),
    FnStatement(FnStatement),
}

#[derive(Debug, Clone)]
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
        return self.compound_assignment();
    }

    fn can_be_statement_without_semicolon(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Block(_) => true,
            Expression::For(_) => true,
            Expression::While(_) => true,
            Expression::If(_) => true,
            _ => false,
        }
    }

    fn block(&mut self) -> Result<Block> {
        // TODO: Block parsing is inconsistent with rust when a block has no
        // semicolon followed by a newline in a statement context.
        // Ex: https://doc.rust-lang.org/reference/statements.html
        // We always parse as a full statement.
        // Correct solution may be to insert semicolons in lexer?
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
                    if self.matches(&[TokenType::Semicolon])?
                        || (self.can_be_statement_without_semicolon(&found_expression)
                            && !(self.peek().token_type == TokenType::RightBrace))
                    {
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

    fn compound_assignment(&mut self) -> Result<Expression> {
        let mut expr = self.assignment()?;
        while self.matches(&[
            TokenType::MinusEqual,
            TokenType::PlusEqual,
            TokenType::StarEqual,
            TokenType::SlashEqual,
        ])? {
            let operator = self.previous().token_type;
            let value = self.expression()?;
            match expr {
                Expression::Variable(v) => {
                    expr = Expression::CompoundAssignment(CompoundAssignment {
                        lvalue: LValue::Variable(v),
                        operator,
                        value: Box::new(value),
                    })
                }
                Expression::Index(i) => {
                    expr = Expression::CompoundAssignment(CompoundAssignment {
                        lvalue: LValue::Index(i),
                        operator,
                        value: Box::new(value),
                    })
                }
                _ => {
                    return Err(ParserError(
                        "Not a valid LValue in assignment".to_string(),
                        self.previous().line,
                    ))
                }
            }
        }
        return Ok(expr);
    }

    fn assignment(&mut self) -> Result<Expression> {
        let mut expr = self.equality()?;
        while self.matches(&[TokenType::Equal])? {
            let value = self.expression()?;
            match expr {
                Expression::Variable(v) => {
                    expr = Expression::Assignment(Assignment {
                        lvalue: LValue::Variable(v),
                        value: Box::new(value),
                    })
                }
                Expression::Index(i) => {
                    expr = Expression::Assignment(Assignment {
                        lvalue: LValue::Index(i),
                        value: Box::new(value),
                    })
                }
                _ => {
                    return Err(ParserError(
                        "Not a valid LValue in assignment".to_string(),
                        self.previous().line,
                    ))
                }
            }
        }
        return Ok(expr);
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
        let mut expr = self.range()?;
        while self.matches(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ])? {
            let operator = self.previous();
            let right = self.range()?;
            expr = Expression::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }
        return Ok(expr);
    }

    fn range(&mut self) -> Result<Expression> {
        let mut expr = self.addition()?;
        if self.matches(&[TokenType::DotDot])? {
            let right = self.addition()?;
            expr = Expression::Range(Range {
                left: Box::new(expr),
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
        while self.matches(&[TokenType::Slash, TokenType::Star, TokenType::Percent])? {
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
        return self.unary_postfix();
    }

    fn unary_postfix(&mut self) -> Result<Expression> {
        let mut expression = self.primary();

        loop {
            if self.matches(&[TokenType::LeftBracket])? {
                expression = self.finish_index(expression?);
            } else if self.matches(&[TokenType::Dot])? {
                expression = self.finish_dot(expression?);
            } else if self.matches(&[TokenType::LeftParen])? {
                expression = self.finish_call(expression?);
            } else if self.matches(&[TokenType::Colon])? {
                expression = self.finish_builtin_call(expression?);
            } else {
                break;
            }
        }

        return expression;
    }

    fn finish_index(&mut self, indexer: Expression) -> Result<Expression> {
        let value = self.expression()?;
        self.consume(TokenType::RightBracket, "Expected ']' after arguments.")?;

        return Ok(Expression::Index(Index {
            indexer: Box::new(indexer),
            value: Box::new(value),
        }));
    }

    fn finish_dot(&mut self, indexer: Expression) -> Result<Expression> {
        let name = self.consume(
            TokenType::Identifier,
            "Expected identifier in '.' expression.",
        )?;
        let name = self.scanner.get_lexeme(&name);

        return Ok(Expression::Index(Index {
            indexer: Box::new(indexer),
            value: Box::new(Expression::Literal(Literal::String(name))),
        }));
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

    fn finish_builtin_call(&mut self, callee: Expression) -> Result<Expression> {
        let name = self.consume(TokenType::Identifier, "Expected builtin name.")?;

        self.consume(TokenType::LeftParen, "Expected '(' to start arguments.")?;
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
        let name = self.scanner.get_lexeme(&name);
        return Ok(Expression::BuiltinCall(BuiltinCall {
            callee: Box::new(callee),
            name,
            args,
        }));
    }

    fn if_expression(&mut self) -> Result<Expression> {
        let condition = Box::new(self.expression()?);
        let then_block = self.block()?;
        let mut else_block = None;
        if self.matches(&[TokenType::Else])? {
            else_block = Some(self.block()?);
        }
        return Ok(Expression::If(If {
            condition,
            then_block,
            else_block,
        }));
    }

    fn while_expression(&mut self) -> Result<Expression> {
        let condition = Box::new(self.expression()?);
        let block = self.block()?;
        return Ok(Expression::While(While { condition, block }));
    }

    fn for_expression(&mut self) -> Result<Expression> {
        let variable = self.consume(TokenType::Identifier, "Expected identifier in for loop")?;
        let variable = self.scanner.get_lexeme(&variable);
        self.consume(TokenType::In, "Expected 'in' in for loop.")?;
        let range = self.expression()?;
        let block = self.block()?;
        return Ok(Expression::For(For {
            variable,
            range: Box::new(range),
            block,
        }));
    }

    fn loop_expression(&mut self) -> Result<Expression> {
        let block = self.block()?;
        return Ok(Expression::Loop(Loop { block }));
    }

    fn return_expression(&mut self) -> Result<Expression> {
        let mut value = None;
        // TODO: Check how Rust works out wether a return has an expression.
        if !(self.check(TokenType::Semicolon) || self.check(TokenType::RightBrace)) {
            value = Some(Box::new(self.expression()?));
        }
        return Ok(Expression::Return(Return { value }));
    }

    fn array(&mut self) -> Result<Expression> {
        let mut out = Array {
            initializers: Vec::new(),
        };
        loop {
            if self.check(TokenType::RightBracket) {
                break;
            }
            out.initializers.push(self.expression()?);
            if !self.matches(&[TokenType::Comma])? {
                break;
            }
        }
        self.consume(TokenType::RightBracket, "Expected ']' to close array.")?;
        return Ok(Expression::Array(out));
    }

    fn map(&mut self) -> Result<Expression> {
        let mut out = Map {
            initializers: Vec::new(),
        };
        loop {
            if self.check(TokenType::RightBrace) {
                break;
            }
            if self.matches(&[TokenType::LeftBracket])? {
                let lhs = self.expression()?;
                self.consume(
                    TokenType::RightBracket,
                    "Expected ']' after map key expression",
                )?;
                self.consume(TokenType::Colon, "Expected ':' in map initializer")?;
                let value = self.expression()?;
                out.initializers.push(MapInitializer {
                    key: MapLHS::Expression(lhs),
                    value: Box::new(value),
                });
            } else {
                let name_t = self.consume(
                    TokenType::Identifier,
                    "Expected identifier in map initializer",
                )?;
                let name = self.scanner.get_lexeme(&name_t);
                if self.matches(&[TokenType::Colon])? {
                    let value = self.expression()?;
                    out.initializers.push(MapInitializer {
                        key: MapLHS::Name(name),
                        value: Box::new(value),
                    });
                } else {
                    out.initializers.push(MapInitializer {
                        key: MapLHS::Name(name.clone()),
                        value: Box::new(Expression::Variable(Variable { name })),
                    });
                }
            }
            if !self.matches(&[TokenType::Comma])? {
                break;
            }
        }
        self.consume(TokenType::RightBrace, "Expected '}' to end map expression")?;
        return Ok(Expression::Map(out));
    }

    fn primary(&mut self) -> Result<Expression> {
        if self.peek().token_type == TokenType::LeftBrace {
            return Ok(Expression::Block(self.block()?));
        }
        if self.matches(&[TokenType::LeftBracket])? {
            return self.array();
        }
        if self.matches(&[TokenType::HashLeftBrace])? {
            return self.map();
        }
        if self.matches(&[TokenType::If])? {
            return self.if_expression();
        }
        if self.matches(&[TokenType::While])? {
            return self.while_expression();
        }
        if self.matches(&[TokenType::For])? {
            return self.for_expression();
        }
        if self.matches(&[TokenType::Loop])? {
            return self.loop_expression();
        }
        if self.matches(&[TokenType::Return])? {
            return self.return_expression();
        }
        if self.matches(&[TokenType::Break])? {
            return Ok(Expression::Break);
        }
        if self.matches(&[TokenType::Continue])? {
            return Ok(Expression::Continue);
        }
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
                Err(_) => Err(ParserError(
                    "Invalid number literal".to_string(),
                    self.previous().line,
                )),
            };
        }
        if self.matches(&[TokenType::String])? {
            let t = self.previous();
            let s = self.scanner.get_lexeme(&t);
            let s = &s[1..s.len() - 1];
            return Ok(Expression::Literal(Literal::String(s.to_string())));
        }
        if self.matches(&[TokenType::CharLiteral])? {
            let t = self.previous();
            let s = self.scanner.get_lexeme(&t);
            let chars = s.chars().collect::<Vec<_>>();
            let mut c = chars[1];
            if c == '\\' {
                match chars[2] {
                    'n' => c = '\n',
                    'r' => c = '\r',
                    't' => c = '\t',
                    _ => return Err(ParserError("Unknown char literal escape".to_string(), self.previous().line)),
                }
            }
            return Ok(Expression::Literal(Literal::Char(c)));
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
        return Err(ParserError(
            "Expect expression".to_string(),
            self.peek().line,
        ));
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
        Err(ParserError(message.to_string(), self.peek().line))
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
