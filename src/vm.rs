use super::{chunk, compiler, debug, errors::NotloxError, value::*};
use std::collections::HashMap;
use std::time::Instant;

const STACK_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
struct CallFrame {
    return_address: usize,
    locals_base: usize,
}

pub struct VM {
    chunk: chunk::Chunk,
    ip: usize,
    stack: Vec<Value>,
    stack_top: usize,
    return_stack: [CallFrame; STACK_SIZE],
    return_stack_top: usize,
    locals: Vec<Value>,
    locals_base: usize,
    locals_top: usize,
    heap: Vec<ReferenceType>,
}

#[derive(Debug)]
pub enum InterpreterError {
    CompileError(NotloxError),
    RuntimeError(String, usize),
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
            InterpreterError::RuntimeError(s, line) => {
                write!(f, "Runtime Error, line {}: {}", line, s)
            }
        }
    }
}

impl std::error::Error for InterpreterError {}

macro_rules! binary_op {
    ( $self:expr, $op:tt, $type: ident, $ret:ident, $line:expr ) => {
        {
            let mut b;
            if let Value::$type(bval) = $self.pop() {
                b = bval;
            } else {
                return Err(InterpreterError::RuntimeError("Bad argument to binary operator, not a number.".to_string(), $line));
            }
            let mut a;
            if let Value::$type(aval) = $self.pop() {
                a = aval;
            } else {
                return Err(InterpreterError::RuntimeError("Bad argument to binary operator, not a number.".to_string(), $line));
            }
            $self.push(Value::$ret(a $op b))
        }
    }
}

impl VM {
    pub fn new() -> VM {
        let mut array = Vec::new();
        array.resize(STACK_SIZE, Value::Nil);

        VM {
            chunk: chunk::Chunk::new(),
            ip: 0,
            stack: array.clone(),
            stack_top: 0,
            return_stack: [CallFrame {
                return_address: 0,
                locals_base: 0,
            }; STACK_SIZE],
            return_stack_top: 0,
            locals: array,
            locals_base: 0,
            locals_top: 0,
            heap: Vec::new(),
        }
    }

    pub fn interpret(&mut self, source: &str) -> Result<Value, InterpreterError> {
        let start = Instant::now();
        let chunk = compiler::compile(source)?;
        let compiled = Instant::now();
        self.chunk = chunk;
        self.ip = self.chunk.lookup_function("main");
        let result = self.run();
        let finished = Instant::now();
        println!(
            "VM Done. Compiled: {}s {}ms, Run: {}s {}ms.",
            compiled.duration_since(start).as_secs(),
            compiled.duration_since(start).subsec_millis(),
            finished.duration_since(compiled).as_secs(),
            finished.duration_since(compiled).subsec_millis()
        );
        result
    }

    pub fn run(&mut self) -> Result<Value, InterpreterError> {
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
            let current_line = self.chunk.lines[self.ip];
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
                    if let Value::Number(value) = self.pop() {
                        self.push(Value::Number(-value));
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Bad argument to unary operator, not a number.".to_string(),
                            current_line,
                        ));
                    }
                }

                Some(chunk::OpCode::Add) => {
                    let top = self.peek();
                    if let Value::Number(_) = top {
                        binary_op!(self, +, Number, Number, current_line)
                    } else if let Value::String(_) = top {
                        let mut b;
                        if let Value::String(bval) = self.pop() {
                            b = bval;
                        } else {
                            return Err(InterpreterError::RuntimeError(
                                "Bad argument to binary operator, not a string.".to_string(),
                                current_line,
                            ));
                        }
                        let mut a;
                        if let Value::String(aval) = self.pop() {
                            a = aval;
                        } else {
                            return Err(InterpreterError::RuntimeError(
                                "Bad argument to binary operator, not a number.".to_string(),
                                current_line,
                            ));
                        }
                        self.push(Value::String(a + &b))
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Bad or mismatched arguments to +".to_string(),
                            current_line,
                        ));
                    }
                }
                Some(chunk::OpCode::Subtract) => binary_op!(self, -, Number, Number, current_line),
                Some(chunk::OpCode::Multiply) => binary_op!(self, *, Number, Number, current_line),
                Some(chunk::OpCode::Divide) => binary_op!(self, /, Number, Number, current_line),
                Some(chunk::OpCode::Remainder) => binary_op!(self, %, Number, Number, current_line),

                Some(chunk::OpCode::Print) => println!("{}", self.pop()),

                Some(chunk::OpCode::AssignLocal) => {
                    let number = self.read_byte() as usize + self.locals_base;
                    if number >= self.locals_top {
                        return Err(InterpreterError::RuntimeError(
                            "Local store out of range".to_string(),
                            current_line,
                        ));
                    }
                    self.locals[number as usize] = self.pop();
                }
                Some(chunk::OpCode::LoadLocal) => {
                    let number = self.read_byte() as usize + self.locals_base;
                    if number >= self.locals_top {
                        return Err(InterpreterError::RuntimeError(
                            "Local load out of range".to_string(),
                            current_line,
                        ));
                    }
                    let value = self.locals[number as usize].clone();
                    self.push(value);
                }

                Some(chunk::OpCode::PushNil) => {
                    self.push(Value::Nil);
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
                    let target = self.read_signed_16();
                    let value = self.pop();
                    if value.is_falsey() {
                        self.ip = (self.ip as isize + target as isize) as usize;
                    }
                }
                Some(chunk::OpCode::Jump) => {
                    let target = self.read_signed_16();
                    self.ip = (self.ip as isize + target as isize) as usize;
                }

                Some(chunk::OpCode::TestLess) => binary_op!(self, <, Number, Boolean, current_line),
                Some(chunk::OpCode::TestLessOrEqual) => {
                    binary_op!(self, <=, Number, Boolean, current_line)
                }
                Some(chunk::OpCode::TestGreater) => {
                    binary_op!(self, >, Number, Boolean, current_line)
                }
                Some(chunk::OpCode::TestGreaterOrEqual) => {
                    binary_op!(self, >=, Number, Boolean, current_line)
                }

                Some(chunk::OpCode::TestEqual) => {
                    let a = self.pop();
                    let b = self.pop();
                    self.push(Value::Boolean(a == b));
                }
                Some(chunk::OpCode::TestNotEqual) => {
                    let a = self.pop();
                    let b = self.pop();
                    self.push(Value::Boolean(a != b));
                }

                Some(chunk::OpCode::Index) => {
                    let the_value = self.pop();
                    let indexer = self.pop();
                    match indexer {
                        Value::String(s) => {
                            let v;
                            if let Value::Number(n) = the_value {
                                v = n as usize;
                            } else {
                                return Err(InterpreterError::RuntimeError(
                                    "Index must be number.".to_string(),
                                    current_line,
                                ));
                            }
                            // Todo: Make this better and maybe utf8 safe.
                            let c = s.into_bytes()[v];
                            self.push(Value::Number(c as f64));
                        }

                        Value::ReferenceId(id) => {
                            let to_push;
                            {
                                let ref_type = &mut self.heap[id];
                                match ref_type {
                                    ReferenceType::Array(ref mut a) => {
                                        let v;
                                        if let Value::Number(n) = the_value {
                                            v = n as usize;
                                        } else {
                                            return Err(InterpreterError::RuntimeError(
                                                "Index must be number.".to_string(),
                                                current_line,
                                            ));
                                        }
                                        if v >= a.len() {
                                            a.resize(v + 1, Value::Nil);
                                        }
                                        to_push = a[v].clone();
                                    }
                                    ReferenceType::Map(m) => {
                                        let hashable_value =
                                            HashableValue::try_from(the_value, current_line)
                                                .unwrap();
                                        to_push =
                                            m.get(&hashable_value).unwrap_or(&Value::Nil).clone();
                                    }
                                    _ => {
                                        return Err(InterpreterError::RuntimeError(
                                            "Don't know how to index that.".to_string(),
                                            current_line,
                                        ));
                                    }
                                }
                            }
                            self.push(to_push);
                        }

                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Don't know how to index that.".to_string(),
                                current_line,
                            ));
                        }
                    }
                }

                Some(chunk::OpCode::NewArray) => {
                    let id = self.new_reference_type(ReferenceType::Array(Vec::new()));
                    self.push(Value::ReferenceId(id));
                }

                Some(chunk::OpCode::PushArray) => {
                    let value = self.pop();
                    let array = self.pop();
                    match array {
                        Value::ReferenceId(id) => {
                            {
                                let ref_type = &mut self.heap[id];
                                match ref_type {
                                    ReferenceType::Array(ref mut a) => {
                                        a.push(value);
                                    }
                                    _ => {
                                        return Err(InterpreterError::RuntimeError(
                                            "Array push on non-array".to_string(),
                                            current_line,
                                        ));
                                    }
                                }
                            }
                            self.push(Value::ReferenceId(id));
                        }
                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Array push on non-array".to_string(),
                                current_line,
                            ));
                        }
                    }
                }

                Some(chunk::OpCode::IndexAssign) => {
                    let new_value = self.pop();
                    let index_value = self.pop();
                    let indexer = self.pop();
                    match indexer {
                        Value::ReferenceId(id) => {
                            let ref_type = &mut self.heap[id];
                            match ref_type {
                                ReferenceType::Array(ref mut a) => {
                                    let n;
                                    if let Value::Number(value) = index_value {
                                        n = value as usize;
                                    } else {
                                        return Err(InterpreterError::RuntimeError(
                                            "Index must be number.".to_string(),
                                            current_line,
                                        ));
                                    }
                                    if n >= a.len() {
                                        a.resize(n + 1, Value::Nil);
                                    }
                                    a[n] = new_value;
                                }
                                ReferenceType::Map(ref mut m) => {
                                    m.insert(
                                        HashableValue::try_from(index_value, current_line)?,
                                        new_value,
                                    );
                                }

                                _ => {
                                    return Err(InterpreterError::RuntimeError(
                                        "Don't know how to index assign that".to_string(),
                                        current_line,
                                    ));
                                }
                            }
                        }

                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Don't know how to index assign that".to_string(),
                                current_line,
                            ));
                        }
                    }
                }

                Some(chunk::OpCode::BuiltinCall) => {
                    let builtin = self.pop();
                    let callee = self.pop();
                    let builtin = if let Value::String(s) = builtin {
                        s
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Expected builtin name".to_string(),
                            current_line,
                        ));
                    };

                    // TODO: Some kind of data driven solution rather than hardcoded ifs.
                    match callee {
                        Value::ReferenceId(id) => {
                            let to_push;
                            {
                                let ref_type = &mut self.heap[id];
                                match ref_type {
                                    ReferenceType::Array(ref mut a) => {
                                        if builtin == "len" {
                                            to_push = Value::Number(a.len() as f64);
                                        } else if builtin == "push" {
                                            let value = {
                                                // Hack, copied pop
                                                self.stack_top -= 1;
                                                self.stack[self.stack_top].clone()
                                            };
                                            a.push(value);
                                            to_push = Value::Nil;
                                        } else {
                                            return Err(InterpreterError::RuntimeError(
                                                "Unknown array builtin".to_string(),
                                                current_line,
                                            ));
                                        }
                                    }
                                    _ => {
                                        return Err(InterpreterError::RuntimeError(
                                            "Unknown builtin".to_string(),
                                            current_line,
                                        ))
                                    }
                                }
                            }
                            self.push(to_push);
                        }

                        Value::String(s) => {
                            if builtin == "len" {
                                self.push(Value::Number(s.len() as f64));
                            } else if builtin == "readFile" {
                                self.push(Value::String(std::fs::read_to_string(s).unwrap()));
                            } else if builtin == "split" {
                                let sep = self.pop();
                                if let Value::String(sep) = sep {
                                    let parts: Vec<_> = s
                                        .split(&sep)
                                        .map(|p| Value::String(p.to_string()))
                                        .collect();
                                    let id = self.new_reference_type(ReferenceType::Array(parts));
                                    self.push(Value::ReferenceId(id));
                                } else {
                                    return Err(InterpreterError::RuntimeError(
                                        "Expected string argument to split".to_string(),
                                        current_line,
                                    ));
                                }
                            } else if builtin == "parseNumber" {
                                self.push(Value::Number(s.parse().unwrap()));
                            } else {
                                return Err(InterpreterError::RuntimeError(
                                    "Unknown string builtin".to_string(),
                                    current_line,
                                ));
                            }
                        }

                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Unknown builtin".to_string(),
                                current_line,
                            ))
                        }
                    }
                }

                Some(chunk::OpCode::MakeRange) => {
                    let right = if let Value::Number(n) = self.pop() {
                        n
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Expected number in range bounds".to_string(),
                            current_line,
                        ));
                    };
                    let left = if let Value::Number(n) = self.pop() {
                        n
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Expected number in range bounds".to_string(),
                            current_line,
                        ));
                    };
                    self.push(Value::Range(left, right))
                }

                Some(chunk::OpCode::ForLoop) => {
                    let local_n = self.read_byte();
                    let jump_target = self.read_signed_16();
                    let target_ip = (self.ip as isize + jump_target as isize) as usize;
                    let range = self.pop();
                    match range {
                        Value::Range(l, r) => {
                            if l < r {
                                self.locals[local_n as usize + self.locals_base] = Value::Number(l);
                                self.push(Value::Range(l + 1.0, r));
                            } else {
                                self.ip = target_ip;
                            }
                        }
                        Value::ReferenceId(id) => {
                            let mut to_push = None;
                            {
                                let ref_type = &mut self.heap[id];
                                match ref_type {
                                    ReferenceType::Array(a) => {
                                        if a.len() > 0 {
                                            self.locals[local_n as usize + self.locals_base] =
                                                Value::Number(0.0);
                                            to_push = Some(Value::Range(1.0, a.len() as f64));
                                        } else {
                                            self.ip = target_ip;
                                        }
                                    }
                                    ReferenceType::Map(m) => {
                                        let keys: Vec<_> = m.keys().map(|e| e.clone()).collect();
                                        let len = keys.len();
                                        if len > 0 {
                                            self.locals[local_n as usize + self.locals_base] =
                                                Value::from(&keys[0]);
                                            to_push =
                                                Some(Value::MapForContext(keys, 1.0, len as f64));
                                        } else {
                                            self.ip = target_ip;
                                        }
                                    }
                                    _ => {
                                        return Err(InterpreterError::RuntimeError(
                                            "Don't know how to for over that".to_string(),
                                            current_line,
                                        ))
                                    }
                                }
                            }
                            if let Some(p) = to_push {
                                self.push(p);
                            }
                        }
                        Value::MapForContext(keys, l, r) => {
                            let mut to_push = None;
                            if l < r {
                                self.locals[local_n as usize + self.locals_base] =
                                    Value::from(&keys[l as usize]);
                                to_push = Some(Value::MapForContext(keys, l + 1.0, r));
                            } else {
                                self.ip = (self.ip as isize + jump_target as isize) as usize;
                            }
                            if let Some(p) = to_push {
                                self.push(p);
                            }
                        }
                        _ => {
                            return Err(InterpreterError::RuntimeError(
                                "Don't know how to for over that".to_string(),
                                current_line,
                            ))
                        }
                    }
                }

                Some(chunk::OpCode::PopMulti) => {
                    let n = self.read_byte();
                    self.stack_top -= n as usize;
                }

                Some(chunk::OpCode::PushTrue) => {
                    self.push(Value::Boolean(true));
                }

                Some(chunk::OpCode::PushFalse) => {
                    self.push(Value::Boolean(false));
                }

                Some(chunk::OpCode::NewMap) => {
                    let id = self.new_reference_type(ReferenceType::Map(HashMap::new()));
                    self.push(Value::ReferenceId(id));
                }

                Some(chunk::OpCode::PushMap) => {
                    let value = self.pop();
                    let key = self.pop();
                    let map = self.pop();
                    if let Value::ReferenceId(id) = map {
                        {
                            let map = &mut self.heap[id];
                            if let ReferenceType::Map(ref mut m) = map {
                                m.insert(HashableValue::try_from(key, current_line)?, value);
                            } else {
                                return Err(InterpreterError::RuntimeError(
                                    "Map push on non-map".to_string(),
                                    current_line,
                                ));
                            }
                        }
                        self.push(Value::ReferenceId(id));
                    } else {
                        return Err(InterpreterError::RuntimeError(
                            "Map push on non-map".to_string(),
                            current_line,
                        ));
                    }
                }

                Some(chunk::OpCode::Not) => {
                    let value = self.pop();
                    self.push(Value::Boolean(value.is_falsey()));
                }

                None => {
                    return Err(InterpreterError::RuntimeError(
                        "Bad instruction".to_string(),
                        current_line,
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

    pub fn read_signed_16(&mut self) -> i16 {
        let number = self.read_byte() as usize;
        let number2 = self.read_byte() as usize;
        (number | number2 << 8) as i16
    }

    pub fn read_constant(&mut self) -> Value {
        let constant_number = self.read_byte();
        self.chunk.constants[constant_number as usize].clone()
    }

    pub fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1
    }

    pub fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top].clone()
    }

    pub fn peek(&mut self) -> Value {
        self.stack[self.stack_top - 1].clone()
    }

    pub fn new_reference_type(&mut self, value: ReferenceType) -> usize {
        self.heap.push(value);
        return self.heap.len() - 1;
    }
}
