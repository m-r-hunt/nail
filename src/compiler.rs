use super::{chunk, chunk::OpCode, debug, errors::Result, parser, scanner::TokenType, value};
use std::collections::HashMap;

pub fn compile(source: &str) -> Result<chunk::Chunk> {
    let ast = parser::parse(source)?;
    let mut compiler = Compiler::new();
    compiler.compile_program(ast);
    compiler.chunk.write_chunk(OpCode::Return as u8, 1);
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
        for d in program.declarations {
            self.compile_declaration(d);
        }
    }

    fn compile_declaration(&mut self, declaration: parser::Declaration) {
        match declaration {
            parser::Declaration::VariableDeclaration(v) => self.compile_variable_declaration(v),
            parser::Declaration::Statement(s) => self.compile_statement(s),
        }
    }

    fn compile_variable_declaration(&mut self, variable_declaration: parser::VariableDeclaration) {
        let local_number = self.next_local;
        self.locals
            .insert(variable_declaration.name, self.next_local);
        self.next_local += 1;
        variable_declaration.initializer.map(|expression| {
            self.compile_expression(expression);
            self.chunk.write_chunk(OpCode::AssignLocal as u8, 1);
            self.chunk.write_chunk(local_number, 1);
        });
    }

    fn compile_statement(&mut self, statement: parser::Statement) {
        match statement {
            parser::Statement::PrintStatement(p) => self.compile_print_statement(p),
            parser::Statement::ExpressionStatement(e) => self.compile_expression_statement(e),
        }
    }

    fn compile_print_statement(&mut self, statement: parser::PrintStatement) {
        self.compile_expression(statement.value);
        self.chunk.write_chunk(OpCode::Print as u8, 1);
    }

    fn compile_expression_statement(&mut self, statement: parser::ExpressionStatement) {
        self.compile_expression(statement.expression);
    }

    fn compile_expression(&mut self, expression: parser::Expression) {
        match expression {
            parser::Expression::Literal(l) => self.compile_literal(l),
            parser::Expression::Unary(u) => self.compile_unary(u),
            parser::Expression::Binary(b) => self.compile_binary(b),
            parser::Expression::Grouping(g) => self.compile_grouping(g),
            parser::Expression::Variable(v) => self.compile_variable(v),
        }
    }

    fn compile_literal(&mut self, literal: parser::Literal) {
        match literal {
            parser::Literal::Number(n) => {
                let c = self.chunk.add_constant(value::Value(n));
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
}
