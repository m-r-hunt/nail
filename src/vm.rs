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
    RuntimeError,
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
            InterpreterError::RuntimeError => write!(f, "{}", "Runtime Error",),
        }
    }
}

impl std::error::Error for InterpreterError {}

macro_rules! binary_op {
    ( $self:expr, $op:tt ) => {
        {
            let b = $self.pop().0;
            let a = $self.pop().0;
            $self.push(value::Value(a $op b))
        }
    }
}

impl VM {
    pub fn new() -> VM {
        VM {
            chunk: chunk::Chunk::new(),
            ip: 0,
            stack: [value::Value(0.0); STACK_SIZE],
            stack_top: 0,
            locals: [value::Value(0.0); 256],
        }
    }

    pub fn interpret(&mut self, source: &str) -> Result<(), InterpreterError> {
        let chunk = compiler::compile(source)?;
        self.chunk = chunk;
        self.ip = 0;
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
            }
            let instruction = self.read_byte();
            match chunk::OpCode::try_from(instruction) {
                Some(chunk::OpCode::Return) => {
                    return Ok(());
                }

                Some(chunk::OpCode::Constant) => {
                    let constant = self.read_constant();
                    self.push(constant);
                }

                Some(chunk::OpCode::Negate) => {
                    let value = self.pop().0;
                    self.push(value::Value(-value));
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

                None => return Err(InterpreterError::RuntimeError),
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
