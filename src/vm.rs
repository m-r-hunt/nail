use super::{chunk, compiler, debug, errors::NotloxError, value};

const STACK_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
struct CallFrame {
    return_address: usize,
    locals_base: usize,
}

pub struct VM {
    chunk: chunk::Chunk,
    ip: usize,
    stack: [value::Value; STACK_SIZE],
    stack_top: usize,
    return_stack: [CallFrame; 256],
    return_stack_top: usize,
    locals: [value::Value; 256],
    locals_base: usize,
    locals_top: usize,
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
    ( $self:expr, $op:tt, $ret:ident ) => {
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
            $self.push(value::Value::$ret(a $op b))
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
            return_stack: [CallFrame {
                return_address: 0,
                locals_base: 0,
            }; 256],
            return_stack_top: 0,
            locals: [value::Value::Nil; 256],
            locals_base: 0,
            locals_top: 0,
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
                    if self.return_stack_top > 0 {
                        let call_frame = self.return_stack[self.return_stack_top - 1];
                        self.return_stack_top -= 1;
                        self.locals_top = self.locals_base;
                        self.locals_base = call_frame.locals_base;
                        self.ip = call_frame.return_address;
                    } else {
                        return Ok(());
                    }
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

                Some(chunk::OpCode::Add) => binary_op!(self, +, Number),
                Some(chunk::OpCode::Subtract) => binary_op!(self, -, Number),
                Some(chunk::OpCode::Multiply) => binary_op!(self, *, Number),
                Some(chunk::OpCode::Divide) => binary_op!(self, /, Number),

                Some(chunk::OpCode::Print) => println!("{}", self.pop()),

                Some(chunk::OpCode::AssignLocal) => {
                    let number = self.read_byte() as usize + self.locals_base;
                    if number >= self.locals_top {
                        return Err(InterpreterError::RuntimeError(
                            "Local store out of range".to_string(),
                        ));
                    }
                    self.locals[number as usize] = self.pop();
                }
                Some(chunk::OpCode::LoadLocal) => {
                    let number = self.read_byte() as usize + self.locals_base;
                    if number >= self.locals_top {
                        return Err(InterpreterError::RuntimeError(
                            "Local load out of range".to_string(),
                        ));
                    }
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
                    let localsn = self.read_byte() as usize;
                    self.locals_top = self.locals_base + localsn;
                }
                Some(chunk::OpCode::Call) => {
                    let fn_number = self.read_byte();
                    self.return_stack[self.return_stack_top] = CallFrame {
                        return_address: self.ip,
                        locals_base: self.locals_base,
                    };
                    self.return_stack_top += 1;
                    self.ip = self.chunk.function_locations[fn_number as usize];
                    self.locals_base = self.locals_top;
                }

                Some(chunk::OpCode::JumpIfFalse) => {
                    let target = self.read_byte();
                    let value = self.pop();
                    if value.is_falsey() {
                        self.ip += target as usize;
                    }
                }
                Some(chunk::OpCode::Jump) => {
                    let target = self.read_byte();
                    self.ip += target as usize;
                }

                Some(chunk::OpCode::TestLess) => binary_op!(self, <, Boolean),
                Some(chunk::OpCode::TestLessOrEqual) => binary_op!(self, <=, Boolean),
                Some(chunk::OpCode::TestGreater) => binary_op!(self, >, Boolean),
                Some(chunk::OpCode::TestGreaterOrEqual) => binary_op!(self, >=, Boolean),

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
