use super::{chunk, chunk::OpCode, debug, errors::Result, parser, scanner::TokenType, value};
use std::collections::HashMap;

pub fn compile(source: &str) -> Result<chunk::Chunk> {
    let ast = parser::parse(source)?;
    let mut compiler = Compiler::new();
    compiler.compile_program(ast);
    debug::disassemble_chunk(&compiler.chunk, "foo.nlx");
    Ok(compiler.chunk)
}

struct Environment {
    locals: HashMap<String, u8>,
    next_local: u8,
}

impl Environment {
    fn new(next_local: u8) -> Self {
        Self{locals: HashMap::new(), next_local}
    }
}

struct Compiler {
    chunk: chunk::Chunk,
    environments: Vec<Environment>,
    deferred: Vec<parser::FnStatement>,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            chunk: chunk::Chunk::new(),
            environments: vec![Environment::new(0)],
            deferred: Vec::new(),
        }
    }

    fn push_environment(&mut self) {
        let new_env = Environment::new(self.environments.last().unwrap().next_local);
        self.environments.push(new_env);
    }

    fn pop_environment(&mut self) {
        self.environments.pop();
    }

    fn find_local(&self, name: &str) -> Option<u8> {
        for e in self.environments.iter().rev() {
            if let Some(n) = e.locals.get(name) {
                return Some(*n);
            }
        }
        return None;
    }

    fn compile_program(&mut self, program: parser::Program) {
        for d in program.statements {
            self.compile_statement(d, true);
        }

        while self.deferred.len() > 0 {
            let fn_statement = self.deferred.pop().unwrap();
            self.compile_fn_statement(fn_statement, true);
        }
    }

    fn compile_statement(&mut self, statement: parser::Statement, top_level: bool) {
        match statement {
            parser::Statement::LetStatement(v) => self.compile_let_statement(v),
            parser::Statement::PrintStatement(p) => self.compile_print_statement(p),
            parser::Statement::ExpressionStatement(e) => self.compile_expression_statement(e),
            parser::Statement::FnStatement(f) => self.compile_fn_statement(f, top_level),
        }
    }

    fn bind_local(&mut self, name: String) -> u8 {
        let current_env = self.environments.last_mut().unwrap();
        current_env.locals.insert(name, current_env.next_local);
        current_env.next_local += 1;
        current_env.next_local - 1
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

    fn compile_fn_statement(&mut self, fn_statement: parser::FnStatement, top_level: bool) {
        self.chunk
            .register_function(fn_statement.name.clone(), fn_statement.args.len() as u8);
        if !top_level {
            self.deferred.push(fn_statement);
        } else {
            self.chunk
                .start_function(fn_statement.name, fn_statement.args.len() as u8, 1);
            for arg in fn_statement.args.into_iter().rev() {
                let local_number = self.bind_local(arg);
                self.chunk.write_chunk(OpCode::AssignLocal as u8, 1);
                self.chunk.write_chunk(local_number, 1);
            }
            self.compile_block(fn_statement.block);
            self.chunk.write_chunk(OpCode::Return as u8, 1);
        }
    }

    fn compile_expression(&mut self, expression: parser::Expression) {
        match expression {
            parser::Expression::Literal(l) => self.compile_literal(l),
            parser::Expression::Unary(u) => self.compile_unary(u),
            parser::Expression::Binary(b) => self.compile_binary(b),
            parser::Expression::Grouping(g) => self.compile_grouping(g),
            parser::Expression::Variable(v) => self.compile_variable(v),
            parser::Expression::Block(b) => self.compile_block(b),
            parser::Expression::Call(c) => self.compile_call(c),
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
        let number = self.find_local(&variable.name).unwrap();
        self.chunk.write_chunk(OpCode::LoadLocal as u8, 1);
        self.chunk.write_chunk(number, 1);
    }

    fn compile_block(&mut self, block: parser::Block) {
        self.push_environment();
        for s in block.statements {
            self.compile_statement(s, false);
        }
        match block.expression {
            Some(e) => self.compile_expression(*e),
            None => self.chunk.write_chunk(OpCode::PushNil as u8, 1),
        }
        self.pop_environment();
    }

    fn compile_call(&mut self, call: parser::Call) {
        for e in call.args {
            self.compile_expression(e);
        }
        self.chunk.write_chunk(OpCode::Call as u8, 1);
        if let parser::Expression::Variable(v) = *call.callee {
            let fn_number = *self.chunk.function_names.get(&v.name).unwrap();
            self.chunk.write_chunk(fn_number, 1);
        } else {
            panic!("Expected variable in call");
        }
    }
}
