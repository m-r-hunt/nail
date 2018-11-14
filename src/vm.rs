use super::{chunk, compiler, debug, errors::NotloxError, value};

const STACK_SIZE: usize = 256;

pub struct VM {
    chunk: chunk::Chunk,
    ip: usize,
    stack: [value::Value; STACK_SIZE],
    stack_top: usize,
    locals: [value::Value; 256],
}

#[derive(Debug)]
pub enum InterpreterError {
    CompileError(NotloxError),
    RuntimeError(String),
}

impl From<NotloxError> for InterpreterError {
    fn from(c: NotloxError) -> Self {
        InterpreterError::CompileError(c)
    }
}

impl std::fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            InterpreterError::CompileError(c) => c.fmt(f),
            InterpreterError::RuntimeError(s) => write!(f, "Runtime Error: {}", s),
        }
    }
}

impl std::error::Error for InterpreterError {}

macro_rules! binary_op {
    ( $self:expr, $op:tt ) => {
        {
            let mut b: f64;
            if let value::Value::Number(bval) = $self.pop() {
                b = bval;
            } else {
                return Err(InterpreterError::RuntimeError("Bad argument to binary operator, not a number.".to_string()));
            }
            let mut a: f64;
            if let value::Value::Number(aval) = $self.pop() {
                a = aval;
            } else {
                return Err(InterpreterError::RuntimeError("Bad argument to binary operator, not a number.".to_string()));
            }
            $self.push(value::Value::Number(a $op b))
        }
    }
}

impl VM {
    pub fn new() -> VM {
        VM {
            chunk: chunk::Chunk::new(),
            ip: 0,
            stack: [value::Value::Nil; STACK_SIZE],
            stack_top: 0,
            locals: [value::Value::Nil; 256],
        }
    }

    pub fn interpret(&mut self, source: &str) -> Result<(), InterpreterError> {
        let chunk = compiler::compile(source)?;
        self.chunk = chunk;
        self.ip = self.chunk.lookup_function("main");
        self.run()
    }

    pub fn run(&mut self) -> Result<(), InterpreterError> {
        loop {
            if cfg!(feature = "debugTraceExecution") {
                print!("          ");
                for slot in 0..self.stack_top {
                    print!("[ {} ]", self.stack[slot])
                }
                println!();
                debug::disassemble_instruction(&self.chunk, self.ip);
                let mut buf = [0; 10];
                use std::io::Read;
                std::io::stdin().read(&mut buf).unwrap();
            }
            let instruction = self.read_byte();
            match chunk::OpCode::try_from(instruction) {
                Some(chunk::OpCode::Return) => {
                    return Ok(()); // Todo: Fix!
                }

                Some(chunk::OpCode::Constant) => {
                    let constant = self.read_constant();
                    self.push(constant);
                }

                Some(chunk::OpCode::Negate) => {
                    if let value::Value::Number(value) = self.pop() {
                        self.push(value::Value::Number(-value));
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Bad argument to unary operator, not a number.".to_string(),
                        ));
                    }
                }

                Some(chunk::OpCode::Add) => binary_op!(self, +),
                Some(chunk::OpCode::Subtract) => binary_op!(self, -),
                Some(chunk::OpCode::Multiply) => binary_op!(self, *),
                Some(chunk::OpCode::Divide) => binary_op!(self, /),

                Some(chunk::OpCode::Print) => println!("{}", self.pop()),

                Some(chunk::OpCode::AssignLocal) => {
                    let number = self.read_byte();
                    self.locals[number as usize] = self.pop();
                }
                Some(chunk::OpCode::LoadLocal) => {
                    let number = self.read_byte();
                    let value = self.locals[number as usize];
                    self.push(value);
                }

                Some(chunk::OpCode::PushNil) => {
                    self.push(value::Value::Nil);
                }
                Some(chunk::OpCode::Pop) => {
                    self.pop();
                }

                Some(chunk::OpCode::FunctionEntry) => {
                    let _argn = self.read_byte();
                    // Todo: Reserve space for locals when this works properly
                }
                Some(chunk::OpCode::Call) => {
                    let fn_number = self.read_byte();
                    self.ip = self.chunk.function_locations[fn_number as usize];
                }

                None => {
                    return Err(InterpreterError::RuntimeError(
                        "Bad instruction".to_string(),
                    ))
                }
            }
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.ip += 1;
        self.chunk.code[self.ip - 1]
    }

    pub fn read_constant(&mut self) -> value::Value {
        let constant_number = self.read_byte();
        self.chunk.constants[constant_number as usize]
    }

    pub fn push(&mut self, value: value::Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1
    }

    pub fn pop(&mut self) -> value::Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
    }
}
