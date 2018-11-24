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
    ( $self:expr, $op:tt, $type: ident, $ret:ident ) => {
        {
            let mut b;
            if let value::Value::$type(bval) = $self.pop() {
                b = bval;
            } else {
                return Err(InterpreterError::RuntimeError("Bad argument to binary operator, not a number.".to_string()));
            }
            let mut a;
            if let value::Value::$type(aval) = $self.pop() {
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
        let array = unsafe {
            let mut array: [value::Value; STACK_SIZE] = std::mem::uninitialized();
            for i in 0..STACK_SIZE {
                array[i] = value::Value::Nil;
            }
            array
        };

        VM {
            chunk: chunk::Chunk::new(),
            ip: 0,
            stack: array.clone(),
            stack_top: 0,
            return_stack: [CallFrame {
                return_address: 0,
                locals_base: 0,
            }; 256],
            return_stack_top: 0,
            locals: array,
            locals_base: 0,
            locals_top: 0,
        }
    }

    pub fn interpret(&mut self, source: &str) -> Result<value::Value, InterpreterError> {
        let chunk = compiler::compile(source)?;
        self.chunk = chunk;
        self.ip = self.chunk.lookup_function("main");
        self.run()
    }

    pub fn run(&mut self) -> Result<value::Value, InterpreterError> {
        loop {
            if cfg!(feature = "debugTraceExecution") {
                print!("          ");
                for slot in 0..self.stack_top {
                    print!("[ {:?} ]", self.stack[slot])
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
                        return Ok(self.pop());
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

                Some(chunk::OpCode::Add) => {
                    let top = self.peek();
                    if let value::Value::Number(_) = top {
                        binary_op!(self, +, Number, Number)
                    } else if let value::Value::String(_) = top {
                        let mut b;
                        if let value::Value::String(bval) = self.pop() {
                            b = bval;
                        } else {
                            return Err(InterpreterError::RuntimeError(
                                "Bad argument to binary operator, not a string.".to_string(),
                            ));
                        }
                        let mut a;
                        if let value::Value::String(aval) = self.pop() {
                            a = aval;
                        } else {
                            return Err(InterpreterError::RuntimeError(
                                "Bad argument to binary operator, not a number.".to_string(),
                            ));
                        }
                        self.push(value::Value::String(a + &b))
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Bad or mismatched arguments to +".to_string(),
                        ));
                    }
                }
                Some(chunk::OpCode::Subtract) => binary_op!(self, -, Number, Number),
                Some(chunk::OpCode::Multiply) => binary_op!(self, *, Number, Number),
                Some(chunk::OpCode::Divide) => binary_op!(self, /, Number, Number),

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
                    let value = self.locals[number as usize].clone();
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
                    let target = self.read_signed_byte();
                    let value = self.pop();
                    if value.is_falsey() {
                        self.ip += target as usize;
                    }
                }
                Some(chunk::OpCode::Jump) => {
                    let target = self.read_signed_byte() as isize;
                    self.ip = (self.ip as isize + target) as usize;
                }

                Some(chunk::OpCode::TestLess) => binary_op!(self, <, Number, Boolean),
                Some(chunk::OpCode::TestLessOrEqual) => binary_op!(self, <=, Number, Boolean),
                Some(chunk::OpCode::TestGreater) => binary_op!(self, >, Number, Boolean),
                Some(chunk::OpCode::TestGreaterOrEqual) => binary_op!(self, >=, Number, Boolean),

                Some(chunk::OpCode::Index) => {
                    let the_value = self.pop();

                    let v;
                    if let value::Value::Number(n) = the_value {
                        v = n as usize;
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Index must be number.".to_string(),
                        ));
                    }

                    let indexer = self.pop();
                    if let value::Value::String(s) = indexer {
                        // Todo: Make this better and maybe utf8 safe.
                        let c = s.into_bytes()[v];
                        self.push(value::Value::Number(c as f64));
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Don't know how to index that.".to_string(),
                        ));
                    }
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

    pub fn read_signed_byte(&mut self) -> i8 {
        self.read_byte() as i8
    }

    pub fn read_constant(&mut self) -> value::Value {
        let constant_number = self.read_byte();
        self.chunk.constants[constant_number as usize].clone()
    }

    pub fn push(&mut self, value: value::Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1
    }

    pub fn pop(&mut self) -> value::Value {
        self.stack_top -= 1;
        self.stack[self.stack_top].clone()
    }

    pub fn peek(&mut self) -> value::Value {
        self.stack[self.stack_top - 1].clone()
    }
}
