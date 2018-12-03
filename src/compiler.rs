use super::{
    chunk, chunk::OpCode, debug, errors::Result, parser, scanner, scanner::TokenType, value,
};
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
        Self {
            locals: HashMap::new(),
            next_local,
        }
    }
}

struct LoopContext {
    continue_address: usize,
    pushed_this_loop: u8,
    breaks: Vec<usize>,
    break_pop: bool,
}

impl LoopContext {
    fn new(continue_address: usize, break_pop: bool) -> Self {
        Self {
            continue_address,
            pushed_this_loop: 0,
            breaks: Vec::new(),
            break_pop,
        }
    }
}

struct Compiler {
    chunk: chunk::Chunk,
    environments: Vec<Environment>,
    loop_contexts: Vec<LoopContext>,
    deferred: Vec<parser::FnStatement>,
    max_local: u8,
    pushed_this_fn: u8,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
            chunk: chunk::Chunk::new(),
            environments: vec![Environment::new(0)],
            loop_contexts: vec![LoopContext::new(0, false)],
            deferred: Vec::new(),
            max_local: 0,
            pushed_this_fn: 0,
        }
    }

    fn push_environment(&mut self) {
        let new_env = Environment::new(self.environments.last().unwrap().next_local);
        self.environments.push(new_env);
    }

    fn pop_environment(&mut self) {
        self.environments.pop();
    }

    fn push_loop_context(&mut self, continue_address: usize, break_pop: bool) {
        self.loop_contexts
            .push(LoopContext::new(continue_address, break_pop));
    }

    fn pop_loop_context(&mut self, break_address: usize) {
        let loop_context = self.loop_contexts.pop().unwrap();
        for b in loop_context.breaks {
            self.insert_jump_address(b, break_address);
        }
    }

    fn adjust_stack_usage(&mut self, usage: i8) {
        self.pushed_this_fn = (self.pushed_this_fn as i8 + usage) as u8;
        self.loop_contexts.last_mut().unwrap().pushed_this_loop =
            (self.loop_contexts.last().unwrap().pushed_this_loop as i8 + usage) as u8;
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
        self.max_local += 1;
        current_env.next_local - 1
    }

    fn compile_let_statement(&mut self, let_statement: parser::LetStatement) {
        let parser::LetStatement {
            name,
            line,
            initializer,
        } = let_statement;
        let opt = initializer.map(|expression| {
            self.compile_expression(expression);
        });
        let local_number = self.bind_local(name);
        opt.map(|_| {
            self.chunk.write_chunk(OpCode::AssignLocal as u8, line);
            self.chunk.write_chunk(local_number, line);
            self.adjust_stack_usage(-1);
        });
    }

    fn compile_print_statement(&mut self, statement: parser::PrintStatement) {
        self.compile_expression(statement.value);
        self.chunk.write_chunk(OpCode::Print as u8, statement.line);
        self.adjust_stack_usage(-1);
    }

    fn compile_expression_statement(&mut self, statement: parser::ExpressionStatement) {
        self.compile_expression(statement.expression);
        self.chunk.write_chunk(OpCode::Pop as u8, statement.line);
        self.adjust_stack_usage(-1);
    }

    fn compile_fn_statement(&mut self, fn_statement: parser::FnStatement, top_level: bool) {
        self.chunk
            .register_function(fn_statement.name.clone(), fn_statement.args.len() as u8);
        if !top_level {
            self.deferred.push(fn_statement);
        } else {
            self.max_local = 0;
            self.pushed_this_fn = 0;
            let locals_addr = self
                .chunk
                .start_function(fn_statement.name, fn_statement.line);
            for arg in fn_statement.args.into_iter().rev() {
                let local_number = self.bind_local(arg);
                self.chunk
                    .write_chunk(OpCode::AssignLocal as u8, fn_statement.line);
                self.chunk.write_chunk(local_number, fn_statement.line);
            }
            self.compile_block(fn_statement.block);
            self.chunk
                .write_chunk(OpCode::Return as u8, fn_statement.line);
            self.chunk.code[locals_addr] = self.max_local;
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
            parser::Expression::If(i) => self.compile_if(i),
            parser::Expression::While(w) => self.compile_while(w),
            parser::Expression::For(f) => self.compile_for(f),
            parser::Expression::Loop(l) => self.compile_loop(l),
            parser::Expression::Assignment(a) => self.compile_assignment(a),
            parser::Expression::CompoundAssignment(ca) => self.compile_compound_assignment(ca),
            parser::Expression::Index(i) => self.compile_index(i),
            parser::Expression::Array(a) => self.compile_array(a),
            parser::Expression::Map(m) => self.compile_map(m),
            parser::Expression::BuiltinCall(c) => self.compile_builtin_call(c),
            parser::Expression::Range(r) => self.compile_range(r),
            parser::Expression::Return(r) => self.compile_return(r),
            parser::Expression::Continue(line) => self.compile_continue(line),
            parser::Expression::Break(line) => self.compile_break(line),
        }
    }

    fn compile_literal(&mut self, literal: parser::Literal) {
        match literal {
            parser::Literal::Number(n, line) => {
                let c = self.chunk.add_constant(value::Value::Number(n));
                self.chunk.write_chunk(OpCode::Constant as u8, line);
                self.chunk.write_chunk(c, line);
                self.adjust_stack_usage(1);
            }
            parser::Literal::String(s, line) => {
                let c = self.chunk.add_constant(value::Value::String(s));
                self.chunk.write_chunk(OpCode::Constant as u8, line);
                self.chunk.write_chunk(c, line);
                self.adjust_stack_usage(1);
            }
            parser::Literal::Char(c, line) => {
                let c = self
                    .chunk
                    .add_constant(value::Value::Number(c as u64 as f64));
                self.chunk.write_chunk(OpCode::Constant as u8, line);
                self.chunk.write_chunk(c, line);
                self.adjust_stack_usage(1);
            }
            parser::Literal::False(line) => {
                self.chunk.write_chunk(OpCode::PushFalse as u8, line);
                self.adjust_stack_usage(1);
            }
            parser::Literal::True(line) => {
                self.chunk.write_chunk(OpCode::PushTrue as u8, line);
                self.adjust_stack_usage(1);
            }
            parser::Literal::Nil(line) => {
                self.chunk.write_chunk(OpCode::PushNil as u8, line);
                self.adjust_stack_usage(1);
            }
        }
    }

    fn compile_unary(&mut self, unary: parser::Unary) {
        self.compile_expression(*unary.expression);
        match unary.operator.token_type {
            TokenType::Minus => self.chunk.write_chunk(OpCode::Negate as u8, unary.line),
            TokenType::Bang => self.chunk.write_chunk(OpCode::Not as u8, unary.line),
            _ => panic!("Unimplemented unary operator"),
        }
    }

    fn compile_binary(&mut self, binary: parser::Binary) {
        self.compile_expression(*binary.left);
        self.compile_expression(*binary.right);
        match binary.operator.token_type {
            TokenType::Plus => self.chunk.write_chunk(OpCode::Add as u8, binary.line),
            TokenType::Minus => self.chunk.write_chunk(OpCode::Subtract as u8, binary.line),
            TokenType::Star => self.chunk.write_chunk(OpCode::Multiply as u8, binary.line),
            TokenType::Slash => self.chunk.write_chunk(OpCode::Divide as u8, binary.line),
            TokenType::Percent => self.chunk.write_chunk(OpCode::Remainder as u8, binary.line),
            TokenType::Less => self.chunk.write_chunk(OpCode::TestLess as u8, binary.line),
            TokenType::LessEqual => self
                .chunk
                .write_chunk(OpCode::TestLessOrEqual as u8, binary.line),
            TokenType::Greater => self
                .chunk
                .write_chunk(OpCode::TestGreater as u8, binary.line),
            TokenType::GreaterEqual => self
                .chunk
                .write_chunk(OpCode::TestGreaterOrEqual as u8, binary.line),
            TokenType::EqualEqual => self.chunk.write_chunk(OpCode::TestEqual as u8, binary.line),
            TokenType::BangEqual => self
                .chunk
                .write_chunk(OpCode::TestNotEqual as u8, binary.line),
            TokenType::AmpersandAmpersand => self.chunk.write_chunk(OpCode::And as u8, binary.line),
            _ => panic!("Unimplemented binary operator"),
        }
        self.adjust_stack_usage(-1);
    }

    fn compile_grouping(&mut self, grouping: parser::Grouping) {
        self.compile_expression(*grouping.expression);
    }

    fn compile_variable(&mut self, variable: parser::Variable) {
        let number = self.find_local(&variable.name).unwrap();
        self.chunk
            .write_chunk(OpCode::LoadLocal as u8, variable.line);
        self.chunk.write_chunk(number, variable.line);
        self.adjust_stack_usage(1);
    }

    fn compile_block(&mut self, block: parser::Block) {
        self.push_environment();
        for s in block.statements {
            self.compile_statement(s, false);
        }
        match block.expression {
            Some(e) => self.compile_expression(*e),
            None => {
                self.chunk.write_chunk(OpCode::PushNil as u8, block.line);
                self.adjust_stack_usage(1);
            }
        }
        self.pop_environment();
    }

    fn compile_call(&mut self, call: parser::Call) {
        let nargs = call.args.len() as u8;
        for e in call.args {
            self.compile_expression(e);
        }
        self.chunk.write_chunk(OpCode::Call as u8, call.line);
        if let parser::Expression::Variable(v) = *call.callee {
            let fn_number = *self.chunk.function_names.get(&v.name).unwrap();
            self.chunk.write_chunk(fn_number, call.line);
        } else {
            panic!("Expected variable in call");
        }
        self.adjust_stack_usage(-(nargs as i8));
        self.adjust_stack_usage(1);
    }

    fn insert_jump_address(&mut self, jump_target_address: usize, dest_address: usize) {
        let addr = (dest_address as isize - jump_target_address as isize - 2) as i16;
        self.chunk.code[jump_target_address] = (addr & 0xFF) as u8;
        self.chunk.code[jump_target_address + 1] = (addr >> 8) as u8;
    }

    fn compile_if(&mut self, if_expression: parser::If) {
        self.compile_expression(*if_expression.condition);
        self.chunk
            .write_chunk(OpCode::JumpIfFalse as u8, if_expression.line);
        self.chunk.write_chunk(0, if_expression.line);
        self.chunk.write_chunk(0, if_expression.line);
        self.adjust_stack_usage(-1);
        let jump_target_address = self.chunk.code.len() - 2;
        self.compile_block(if_expression.then_block);
        self.chunk
            .write_chunk(OpCode::Jump as u8, if_expression.line);
        self.chunk.write_chunk(0, if_expression.line);
        self.chunk.write_chunk(0, if_expression.line);
        let else_target_address = self.chunk.code.len() - 2;
        let addr = self.chunk.code.len();
        self.insert_jump_address(jump_target_address, addr);
        self.adjust_stack_usage(-1);
        match if_expression.else_expression {
            Some(e) => self.compile_expression(*e),
            None => {
                self.chunk
                    .write_chunk(OpCode::PushNil as u8, if_expression.line);
                self.adjust_stack_usage(1);
            }
        }
        let addr = self.chunk.code.len();
        self.insert_jump_address(else_target_address, addr);
    }

    fn compile_while(&mut self, while_expression: parser::While) {
        let while_start_address = self.chunk.code.len();
        self.compile_expression(*while_expression.condition);
        self.chunk
            .write_chunk(OpCode::JumpIfFalse as u8, while_expression.line);
        self.chunk.write_chunk(0, while_expression.line);
        self.chunk.write_chunk(0, while_expression.line);
        self.adjust_stack_usage(-1);
        self.push_loop_context(while_start_address, false);
        let jump_target_address = self.chunk.code.len() - 2;
        self.compile_block(while_expression.block);
        self.chunk
            .write_chunk(OpCode::Pop as u8, while_expression.line);
        self.adjust_stack_usage(-1);
        self.chunk
            .write_chunk(OpCode::Jump as u8, while_expression.line);
        self.chunk.write_chunk(0, while_expression.line);
        self.chunk.write_chunk(0, while_expression.line);
        let current_address = self.chunk.code.len();
        self.insert_jump_address(current_address - 2, while_start_address);
        self.insert_jump_address(jump_target_address, current_address);
        self.pop_loop_context(current_address);
        self.chunk
            .write_chunk(OpCode::PushNil as u8, while_expression.line);
        self.adjust_stack_usage(1);
    }

    fn compile_for(&mut self, for_expression: parser::For) {
        self.compile_expression(*for_expression.range);
        let for_local_n = self.bind_local("_for_loop_range".to_string());
        self.chunk
            .write_chunk(OpCode::AssignLocal as u8, for_expression.line);
        self.chunk.write_chunk(for_local_n, for_expression.line);
        self.chunk
            .write_chunk(OpCode::LoadLocal as u8, for_expression.line);
        self.chunk.write_chunk(for_local_n, for_expression.line);

        let for_start_address = self.chunk.code.len();
        self.chunk
            .write_chunk(OpCode::ForLoop as u8, for_expression.line);
        let local_n = self.bind_local(for_expression.variable);
        self.chunk.write_chunk(local_n, for_expression.line);
        self.chunk.write_chunk(0, for_expression.line);
        self.chunk.write_chunk(0, for_expression.line);
        let for_jump_target_address = self.chunk.code.len() - 2;
        self.push_loop_context(for_start_address, true);

        if let Some(variable2) = for_expression.variable2 {
            let local2_n = self.bind_local(variable2);
            self.chunk
                .write_chunk(OpCode::LoadLocal as u8, for_expression.line);
            self.chunk.write_chunk(for_local_n, for_expression.line);
            self.chunk
                .write_chunk(OpCode::LoadLocal as u8, for_expression.line);
            self.chunk.write_chunk(local_n, for_expression.line);
            self.chunk
                .write_chunk(OpCode::Index as u8, for_expression.line);
            self.chunk
                .write_chunk(OpCode::AssignLocal as u8, for_expression.line);
            self.chunk.write_chunk(local2_n as u8, for_expression.line);
        }

        self.compile_block(for_expression.block);
        self.chunk
            .write_chunk(OpCode::Pop as u8, for_expression.line);
        self.adjust_stack_usage(-1);
        self.chunk
            .write_chunk(OpCode::Jump as u8, for_expression.line);
        self.chunk.write_chunk(0, for_expression.line);
        self.chunk.write_chunk(0, for_expression.line);
        let current_address = self.chunk.code.len();
        self.insert_jump_address(current_address - 2, for_start_address);
        self.insert_jump_address(for_jump_target_address, current_address);
        self.chunk
            .write_chunk(OpCode::PushNil as u8, for_expression.line);
        self.pop_loop_context(current_address);
    }

    fn compile_loop(&mut self, loop_expression: parser::Loop) {
        let loop_start_address = self.chunk.code.len();
        self.push_loop_context(loop_start_address, false);
        self.compile_block(loop_expression.block);
        self.chunk
            .write_chunk(OpCode::Pop as u8, loop_expression.line);
        self.adjust_stack_usage(-1);
        self.chunk
            .write_chunk(OpCode::Jump as u8, loop_expression.line);
        self.chunk.write_chunk(0, loop_expression.line);
        self.chunk.write_chunk(0, loop_expression.line);
        let current_address = self.chunk.code.len();
        self.insert_jump_address(current_address - 2, loop_start_address);
        self.pop_loop_context(current_address);
        self.chunk
            .write_chunk(OpCode::PushNil as u8, loop_expression.line);
        self.adjust_stack_usage(1);
    }

    fn compile_assignment(&mut self, assignment: parser::Assignment) {
        match assignment.lvalue {
            parser::LValue::Variable(v) => {
                self.compile_expression(*assignment.value);
                self.chunk
                    .write_chunk(OpCode::AssignLocal as u8, assignment.line);
                let local_number = self.find_local(&v.name).unwrap();
                self.chunk.write_chunk(local_number, assignment.line);
                self.adjust_stack_usage(-1);
            }
            parser::LValue::Index(i) => {
                self.compile_expression(*i.indexer);
                self.compile_expression(*i.value);
                self.compile_expression(*assignment.value);
                self.chunk
                    .write_chunk(OpCode::IndexAssign as u8, assignment.line);
                self.adjust_stack_usage(-3);
            }
        }
        self.chunk
            .write_chunk(OpCode::PushNil as u8, assignment.line);
        self.adjust_stack_usage(1);
    }

    fn compile_compound_assignment(&mut self, compound_assignment: parser::CompoundAssignment) {
        let op = scanner::Token {
            token_type: match compound_assignment.operator {
                TokenType::MinusEqual => TokenType::Minus,
                TokenType::PlusEqual => TokenType::Plus,
                TokenType::StarEqual => TokenType::Star,
                TokenType::SlashEqual => TokenType::Slash,
                _ => panic!("Unsupported compound assignment"),
            },
            start: 0,
            length: 0,
            line: compound_assignment.line,
        };
        let lvalue = Box::new(match compound_assignment.lvalue.clone() {
            parser::LValue::Variable(v) => parser::Expression::Variable(v),
            parser::LValue::Index(i) => parser::Expression::Index(i),
        });
        self.compile_assignment(parser::Assignment {
            lvalue: compound_assignment.lvalue,
            value: Box::new(parser::Expression::Binary(parser::Binary {
                left: lvalue,
                operator: op,
                right: compound_assignment.value,
                line: compound_assignment.line,
            })),
            line: compound_assignment.line,
        });
    }

    fn compile_index(&mut self, index: parser::Index) {
        self.compile_expression(*index.indexer);
        self.compile_expression(*index.value);
        self.chunk.write_chunk(OpCode::Index as u8, index.line);
        self.adjust_stack_usage(-1);
    }

    fn compile_array(&mut self, array: parser::Array) {
        self.chunk.write_chunk(OpCode::NewArray as u8, array.line);
        self.adjust_stack_usage(1);
        for e in array.initializers {
            self.compile_expression(e);
            self.chunk.write_chunk(OpCode::PushArray as u8, array.line);
            self.adjust_stack_usage(-1);
        }
    }

    fn compile_map(&mut self, map: parser::Map) {
        self.chunk.write_chunk(OpCode::NewMap as u8, map.line);
        self.adjust_stack_usage(1);
        for i in map.initializers {
            match i.key {
                parser::MapLHS::Name(s) => {
                    let c = self.chunk.add_constant(value::Value::String(s));
                    self.chunk.write_chunk(OpCode::Constant as u8, map.line);
                    self.chunk.write_chunk(c, map.line);
                    self.adjust_stack_usage(1);
                }
                parser::MapLHS::Expression(e) => {
                    self.compile_expression(e);
                }
            }

            self.compile_expression(*i.value);

            self.chunk.write_chunk(OpCode::PushMap as u8, map.line);
            self.adjust_stack_usage(-2);
        }
    }

    fn compile_builtin_call(&mut self, builtin_call: parser::BuiltinCall) {
        let nargs = builtin_call.args.len() as u8;
        for e in builtin_call.args {
            self.compile_expression(e);
        }
        self.compile_expression(*builtin_call.callee);
        let c = self
            .chunk
            .add_constant(value::Value::String(builtin_call.name));
        self.chunk
            .write_chunk(OpCode::Constant as u8, builtin_call.line);
        self.chunk.write_chunk(c, builtin_call.line);
        self.adjust_stack_usage(1);
        self.chunk
            .write_chunk(OpCode::BuiltinCall as u8, builtin_call.line);
        self.adjust_stack_usage(-2 - (nargs as i8));
        self.adjust_stack_usage(1);
    }

    fn compile_range(&mut self, range: parser::Range) {
        self.compile_expression(*range.left);
        self.compile_expression(*range.right);
        self.chunk.write_chunk(OpCode::MakeRange as u8, range.line);
        self.adjust_stack_usage(-1);
    }

    fn compile_return(&mut self, return_expression: parser::Return) {
        if self.pushed_this_fn > 0 {
            self.chunk
                .write_chunk(OpCode::PopMulti as u8, return_expression.line);
            self.chunk
                .write_chunk(self.pushed_this_fn, return_expression.line);
        }
        match return_expression.value {
            Some(e) => self.compile_expression(*e),
            None => {
                self.chunk
                    .write_chunk(OpCode::PushNil as u8, return_expression.line);
                self.adjust_stack_usage(1);
            }
        }
        self.chunk
            .write_chunk(OpCode::Return as u8, return_expression.line);
        self.adjust_stack_usage(1); // Logically this should be an expression returning a value, but it doesn't return.
    }

    fn compile_continue(&mut self, line: usize) {
        if self.loop_contexts.last().unwrap().pushed_this_loop > 0 {
            self.chunk.write_chunk(OpCode::PopMulti as u8, line);
            self.chunk
                .write_chunk(self.loop_contexts.last().unwrap().pushed_this_loop, line);
        }
        self.chunk.write_chunk(OpCode::Jump as u8, line);
        self.chunk.write_chunk(0, line);
        self.chunk.write_chunk(0, line);
        let jump_target_address = self.chunk.code.len() - 2;
        let continue_address = self.loop_contexts.last().unwrap().continue_address;
        self.insert_jump_address(jump_target_address, continue_address);
        self.adjust_stack_usage(1); // Logically this should be an expression returning a value, but it doesn't return.
    }

    fn compile_break(&mut self, line: usize) {
        if self.loop_contexts.last().unwrap().pushed_this_loop > 0 {
            self.chunk.write_chunk(OpCode::PopMulti as u8, line);
            self.chunk
                .write_chunk(self.loop_contexts.last().unwrap().pushed_this_loop, line);
            self.loop_contexts.last_mut().unwrap().pushed_this_loop = 0;
        }
        if self.loop_contexts.last().unwrap().break_pop {
            self.chunk.write_chunk(OpCode::Pop as u8, line);
        }
        self.chunk.write_chunk(OpCode::Jump as u8, line);
        self.chunk.write_chunk(0, line);
        self.chunk.write_chunk(0, line);
        self.loop_contexts
            .last_mut()
            .unwrap()
            .breaks
            .push(self.chunk.code.len() - 2);
        self.adjust_stack_usage(1); // Logically this should be an expression returning a value, but it doesn't return.
    }
}
