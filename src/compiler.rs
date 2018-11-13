use super::{chunk, chunk::OpCode, debug, errors::Result, parser, scanner::TokenType, value};
use std::collections::HashMap;

pub fn compile(source: &str) -> Result<chunk::Chunk> {
    let ast = parser::parse(source)?;
    let mut compiler = Compiler::new();
    compiler.compile_program(ast);
    debug::disassemble_chunk(&compiler.chunk, "foo.nlx");
    Ok(compiler.chunk)
}

struct Compiler {
    chunk: chunk::Chunk,
    locals: HashMap<String, u8>,
    next_local: u8,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            chunk: chunk::Chunk::new(),
            locals: HashMap::new(),
            next_local: 0,
        }
    }

    fn compile_program(&mut self, program: parser::Program) {
        for d in program.statements {
            self.compile_statement(d);
        }
    }

    fn compile_statement(&mut self, statement: parser::Statement) {
        match statement {
            parser::Statement::LetStatement(v) => self.compile_let_statement(v),
            parser::Statement::PrintStatement(p) => self.compile_print_statement(p),
            parser::Statement::ExpressionStatement(e) => self.compile_expression_statement(e),
            parser::Statement::FnStatement(f) => self.compile_fn_statement(f),
        }
    }

    fn bind_local(&mut self, name: String) -> u8 {
        self.locals.insert(name, self.next_local);
        self.next_local += 1;
        self.next_local - 1
    }

    fn compile_let_statement(&mut self, let_statement: parser::LetStatement) {
        let local_number = self.bind_local(let_statement.name);
        let_statement.initializer.map(|expression| {
            self.compile_expression(expression);
            self.chunk.write_chunk(OpCode::AssignLocal as u8, 1);
            self.chunk.write_chunk(local_number, 1);
        });
    }

    fn compile_print_statement(&mut self, statement: parser::PrintStatement) {
        self.compile_expression(statement.value);
        self.chunk.write_chunk(OpCode::Print as u8, 1);
    }

    fn compile_expression_statement(&mut self, statement: parser::ExpressionStatement) {
        self.compile_expression(statement.expression);
        self.chunk.write_chunk(OpCode::Pop as u8, 1);
    }

    fn compile_fn_statement(&mut self, fn_statement: parser::FnStatement) {
        self.chunk
            .add_function(fn_statement.name, fn_statement.args.len() as u8, 1);
        for arg in fn_statement.args.into_iter().rev() {
            let local_number = self.bind_local(arg);
            self.chunk.write_chunk(OpCode::AssignLocal as u8, 1);
            self.chunk.write_chunk(local_number, 1);
        }
        self.compile_block(fn_statement.block);
        self.chunk.write_chunk(OpCode::Return as u8, 1);
    }

    fn compile_expression(&mut self, expression: parser::Expression) {
        match expression {
            parser::Expression::Literal(l) => self.compile_literal(l),
            parser::Expression::Unary(u) => self.compile_unary(u),
            parser::Expression::Binary(b) => self.compile_binary(b),
            parser::Expression::Grouping(g) => self.compile_grouping(g),
            parser::Expression::Variable(v) => self.compile_variable(v),
            parser::Expression::Block(b) => self.compile_block(b),
        }
    }

    fn compile_literal(&mut self, literal: parser::Literal) {
        match literal {
            parser::Literal::Number(n) => {
                let c = self.chunk.add_constant(value::Value::Number(n));
                self.chunk.write_chunk(OpCode::Constant as u8, 1);
                self.chunk.write_chunk(c, 1);
            }
            _ => panic!("Unimplemented literal"),
        }
    }

    fn compile_unary(&mut self, unary: parser::Unary) {
        self.compile_expression(*unary.expression);
        match unary.operator.token_type {
            TokenType::Minus => self.chunk.write_chunk(OpCode::Negate as u8, 1),
            _ => panic!("Unimplemented unary operator"),
        }
    }

    fn compile_binary(&mut self, binary: parser::Binary) {
        self.compile_expression(*binary.left);
        self.compile_expression(*binary.right);
        match binary.operator.token_type {
            TokenType::Plus => self.chunk.write_chunk(OpCode::Add as u8, 1),
            TokenType::Minus => self.chunk.write_chunk(OpCode::Subtract as u8, 1),
            TokenType::Star => self.chunk.write_chunk(OpCode::Multiply as u8, 1),
            TokenType::Slash => self.chunk.write_chunk(OpCode::Divide as u8, 1),
            _ => panic!("Unimplemented binary operator"),
        }
    }

    fn compile_grouping(&mut self, grouping: parser::Grouping) {
        self.compile_expression(*grouping.expression);
    }

    fn compile_variable(&mut self, variable: parser::Variable) {
        let number = self.locals.get(&variable.name).unwrap();
        self.chunk.write_chunk(OpCode::LoadLocal as u8, 1);
        self.chunk.write_chunk(*number, 1);
    }

    fn compile_block(&mut self, block: parser::Block) {
        for s in block.statements {
            self.compile_statement(s);
        }
        match block.expression {
            Some(e) => self.compile_expression(*e),
            None => self.chunk.write_chunk(OpCode::PushNil as u8, 1),
        }
    }
}
