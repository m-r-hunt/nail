use super::chunk::OpCode;
use super::{chunk, compiler, debug, errors::NotloxError, value::*};
use std::collections::HashMap;
use std::time::Instant;

const STACK_SIZE: usize = 256;

#[derive(Copy, Clone, Debug)]
struct CallFrame {
    return_address: usize,
    locals_base: usize,
}

struct ValueStack {
    stack: Vec<Value>,
    top: usize,
}

impl ValueStack {
    fn new() -> Self {
        let mut array = Vec::new();
        array.resize(STACK_SIZE, Value::Nil);
        ValueStack {
            stack: array,
            top: 0,
        }
    }

    fn push(&mut self, value: Value) {
        self.stack[self.top] = value;
        self.top += 1
    }

    fn pop(&mut self, line: usize) -> Result<Value, InterpreterError> {
        if self.top >= 1 {
            self.top -= 1;
            Ok(self.stack[self.top].clone())
        } else {
            runtime_error("Not enough values on the stack", line)
        }
    }

    fn pop_multi(&mut self, n: usize, line: usize) -> Result<Value, InterpreterError> {
        if self.top >= n {
            self.top -= n;
            Ok(Value::Nil)
        } else {
            runtime_error("Not enough values on the stack", line)
        }
    }

    fn peek(&mut self) -> Value {
        if self.top >= 1 {
            self.stack[self.top - 1].clone()
        } else {
            Value::Nil
        }
    }
}

pub struct VM {
    chunk: chunk::Chunk,
    ip: usize,
    stack: ValueStack,
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpreterError::CompileError(c) => c.fmt(f),
            InterpreterError::RuntimeError(s, line) => {
                write!(f, "Runtime Error, line {}: {}", line, s)
            }
        }
    }
}

impl std::error::Error for InterpreterError {}

fn runtime_error<T>(message: &str, line: usize) -> Result<T, InterpreterError> {
    Err(InterpreterError::RuntimeError(message.to_string(), line))
}

macro_rules! binary_op {
    ( $self:expr, $op:tt, $type: ident, $ret:ident, $line:expr ) => {
        {
            let a;
            if let Value::$type(aval) = $self.stack.pop($line)? {
                a = aval;
            } else {
                return runtime_error("Bad argument to binary operator, not a number.", $line);
            }
            let b;
            if let Value::$type(bval) = $self.stack.pop($line)? {
                b = bval;
            } else {
                return runtime_error("Bad argument to binary operator, not a number.", $line);
            }
            $self.stack.push(Value::$ret(a $op b))
        }
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> VM {
        let mut array = Vec::new();
        array.resize(STACK_SIZE, Value::Nil);

        VM {
            chunk: chunk::Chunk::new(),
            ip: 0,
            stack: ValueStack::new(),
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
                for slot in 0..self.stack.top {
                    print!("[ {:?} ]", self.stack.stack[slot])
                }
                println!();
                debug::disassemble_instruction(&self.chunk, self.ip);
                //let mut buf = [0; 10];
                //use std::io::Read;
                //std::io::stdin().read(&mut buf).unwrap();
            }
            let line = self.chunk.lines[self.ip];
            let instruction = self.read_byte();
            match OpCode::try_from(instruction) {
                Some(OpCode::Return) => {
                    if self.return_stack_top > 0 {
                        let call_frame = self.return_stack[self.return_stack_top - 1];
                        self.return_stack_top -= 1;
                        self.locals_top = self.locals_base;
                        self.locals_base = call_frame.locals_base;
                        self.ip = call_frame.return_address;
                    } else {
                        return Ok(self.stack.pop(line)?);
                    }
                }

                Some(OpCode::Constant) => self.op_constant(line)?,

                Some(OpCode::Negate) => self.op_negate(line)?,

                Some(OpCode::Add) => self.op_add(line)?,
                Some(OpCode::Subtract) => binary_op!(self, -, Number, Number, line),
                Some(OpCode::Multiply) => binary_op!(self, *, Number, Number, line),
                Some(OpCode::Divide) => binary_op!(self, /, Number, Number, line),
                Some(OpCode::Remainder) => binary_op!(self, %, Number, Number, line),

                Some(OpCode::Print) => println!("{}", self.stack.pop(line)?),

                Some(OpCode::AssignLocal) => self.op_assign_local(line)?,
                Some(OpCode::LoadLocal) => self.op_load_local(line)?,

                Some(OpCode::PushNil) => self.stack.push(Value::Nil),

                Some(OpCode::Pop) => {
                    self.stack.pop(line)?;
                }

                Some(OpCode::FunctionEntry) => self.op_function_entry(line)?,
                Some(OpCode::Call) => self.op_call(line)?,

                Some(OpCode::JumpIfFalse) => self.op_jump_if_false(line)?,
                Some(OpCode::Jump) => self.op_jump(line)?,

                Some(OpCode::TestLess) => binary_op!(self, <, Number, Boolean, line),
                Some(OpCode::TestLessOrEqual) => binary_op!(self, <=, Number, Boolean, line),

                Some(OpCode::TestGreater) => binary_op!(self, >, Number, Boolean, line),
                Some(OpCode::TestGreaterOrEqual) => binary_op!(self, >=, Number, Boolean, line),

                Some(OpCode::TestEqual) => {
                    let a = self.stack.pop(line)?;
                    let b = self.stack.pop(line)?;
                    self.stack.push(Value::Boolean(a == b));
                }
                Some(OpCode::TestNotEqual) => {
                    let a = self.stack.pop(line)?;
                    let b = self.stack.pop(line)?;
                    self.stack.push(Value::Boolean(a != b));
                }

                Some(OpCode::Index) => self.op_index(line)?,

                Some(OpCode::NewArray) => {
                    let id = self.new_reference_type(ReferenceType::Array(Vec::new()));
                    self.stack.push(Value::ReferenceId(id));
                }

                Some(OpCode::PushArray) => self.op_push_array(line)?,

                Some(OpCode::IndexAssign) => self.op_index_assign(line)?,

                Some(OpCode::BuiltinCall) => self.op_builtin_call(line)?,

                Some(OpCode::MakeRange) => self.op_make_range(line)?,

                Some(OpCode::ForLoop) => self.op_for_loop(line)?,

                Some(OpCode::PopMulti) => {
                    let n = self.read_byte();
                    self.stack.pop_multi(n as usize, line)?;
                }

                Some(OpCode::PushTrue) => self.stack.push(Value::Boolean(true)),

                Some(OpCode::PushFalse) => self.stack.push(Value::Boolean(false)),

                Some(OpCode::NewMap) => {
                    let id = self.new_reference_type(ReferenceType::Map(HashMap::new()));
                    self.stack.push(Value::ReferenceId(id));
                }

                Some(OpCode::PushMap) => self.op_push_map(line)?,

                Some(OpCode::Not) => {
                    let value = self.stack.pop(line)?;
                    self.stack.push(Value::Boolean(value.is_falsey()));
                }

                Some(OpCode::Dup) => {
                    let val = self.stack.peek();
                    self.stack.push(val);
                }

                Some(OpCode::JumpIfTrue) => self.op_jump_if_true(line)?,

                Some(OpCode::AssignGlobal) => self.op_assign_global(line)?,
                Some(OpCode::LoadGlobal) => self.op_load_global(line)?,

                None => return runtime_error("Bad instruction", line),
            }
        }
    }

    fn op_constant(&mut self, _current_line: usize) -> Result<(), InterpreterError> {
        let constant = self.read_constant();
        self.stack.push(constant);
        Ok(())
    }

    fn op_negate(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        if let Value::Number(value) = self.stack.pop(current_line)? {
            self.stack.push(Value::Number(-value));
            Ok(())
        } else {
            runtime_error("Bad argument to negate, not a number.", current_line)
        }
    }

    fn op_add(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let top = self.stack.peek();
        if let Value::Number(_) = top {
            binary_op!(self, +, Number, Number, current_line)
        } else if let Value::String(_) = top {
            let aa = self.stack.pop(current_line)?;
            let b = self.stack.pop(current_line)?;
            if let Value::String(b) = b {
                let a;
                if let Value::String(aval) = aa {
                    a = aval;
                } else {
                    panic!("This should be unreachable as we know aa is a string.");
                }
                self.stack.push(Value::String(a + &b))
            } else if let Value::Number(n) = b {
                let a;
                if let Value::String(aval) = aa {
                    a = aval;
                } else {
                    panic!("This should be unreachable as we know aa is a string.");
                }
                let mut s = a.clone();
                s.push(n as u8 as char);
                self.stack.push(Value::String(s));
            } else {
                return runtime_error(
                    "Bad argument to binary operator, string must have string or char on RHS.",
                    current_line,
                );
            }
        } else {
            return runtime_error("Bad or mismatched arguments to +", current_line);
        }
        Ok(())
    }
    fn op_assign_local(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let number = self.read_byte() as usize + self.locals_base;
        if number >= self.locals_top {
            return runtime_error("Local store out of range", current_line);
        }
        self.locals[number as usize] = self.stack.pop(current_line)?;
        Ok(())
    }
    fn op_load_local(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let number = self.read_byte() as usize + self.locals_base;
        if number >= self.locals_top {
            return runtime_error("Local load out of range", current_line);
        }
        let value = self.locals[number as usize].clone();
        self.stack.push(value);
        Ok(())
    }

    fn op_function_entry(&mut self, _current_line: usize) -> Result<(), InterpreterError> {
        let localsn = self.read_byte() as usize;
        self.locals_top = self.locals_base + localsn;
        Ok(())
    }

    fn op_call(&mut self, _current_line: usize) -> Result<(), InterpreterError> {
        let fn_number = self.read_byte();
        self.return_stack[self.return_stack_top] = CallFrame {
            return_address: self.ip,
            locals_base: self.locals_base,
        };
        self.return_stack_top += 1;
        self.ip = self.chunk.function_locations[fn_number as usize];
        self.locals_base = self.locals_top;
        Ok(())
    }

    fn op_jump_if_false(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let target = self.read_signed_16();
        let value = self.stack.pop(current_line)?;
        if value.is_falsey() {
            self.ip = (self.ip as isize + target as isize) as usize;
        }
        Ok(())
    }

    fn op_jump(&mut self, _current_line: usize) -> Result<(), InterpreterError> {
        let target = self.read_signed_16();
        self.ip = (self.ip as isize + target as isize) as usize;
        Ok(())
    }

    fn op_index(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let the_value = self.stack.pop(current_line)?;
        let indexer = self.stack.pop(current_line)?;
        match indexer {
            Value::String(s) => {
                let v;
                if let Value::Number(n) = the_value {
                    v = n as usize;
                } else {
                    return runtime_error("Index must be number.", current_line);
                }
                // Todo: Make this better and maybe utf8 safe.
                let c = s.into_bytes()[v];
                self.stack.push(Value::Number(f64::from(c)));
            }

            Value::ReferenceId(id) => {
                let ref_type = &mut self.heap[id];
                match ref_type {
                    ReferenceType::Array(ref mut a) => {
                        let v = if let Value::Number(n) = the_value {
                            n as usize
                        } else {
                            return runtime_error("Index must be number.", current_line);
                        };
                        if v >= a.len() {
                            a.resize(v + 1, Value::Nil);
                        }
                        self.stack.push(a[v].clone());
                    }
                    ReferenceType::Map(m) => {
                        let hashable_value =
                            HashableValue::try_from(&the_value, current_line).unwrap();
                        self.stack
                            .push(m.get(&hashable_value).unwrap_or(&Value::Nil).clone());
                    }
                    _ => {
                        return runtime_error("Don't know how to index that.", current_line);
                    }
                }
            }

            _ => {
                return runtime_error("Don't know how to index that.", current_line);
            }
        }
        Ok(())
    }

    fn op_push_array(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let value = self.stack.pop(current_line)?;
        let array = self.stack.pop(current_line)?;
        match array {
            Value::ReferenceId(id) => {
                let ref_type = &mut self.heap[id];
                match ref_type {
                    ReferenceType::Array(ref mut a) => {
                        a.push(value);
                    }
                    _ => {
                        return runtime_error("Array push on non-array", current_line);
                    }
                }
                self.stack.push(Value::ReferenceId(id));
            }
            _ => {
                return runtime_error("Array push on non-array", current_line);
            }
        }
        Ok(())
    }

    fn op_index_assign(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let new_value = self.stack.pop(current_line)?;
        let index_value = self.stack.pop(current_line)?;
        let indexer = self.stack.pop(current_line)?;
        match indexer {
            Value::ReferenceId(id) => {
                let ref_type = &mut self.heap[id];
                match ref_type {
                    ReferenceType::Array(ref mut a) => {
                        let n;
                        if let Value::Number(value) = index_value {
                            n = value as usize;
                        } else {
                            return runtime_error("Index must be number.", current_line);
                        }
                        if n >= a.len() {
                            a.resize(n + 1, Value::Nil);
                        }
                        a[n] = new_value;
                    }
                    ReferenceType::Map(ref mut m) => {
                        m.insert(
                            HashableValue::try_from(&index_value, current_line)?,
                            new_value,
                        );
                    }

                    _ => return runtime_error("Don't know how to index assign that", current_line),
                }
            }

            _ => return runtime_error("Don't know how to index assign that", current_line),
        }
        Ok(())
    }

    fn op_builtin_call(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let builtin = self.stack.pop(current_line)?;
        let callee = self.stack.pop(current_line)?;
        let builtin = if let Value::String(s) = builtin {
            s
        } else {
            return runtime_error("Expected builtin name", current_line);
        };

        if builtin == "to_string" {
            self.stack.push(Value::String(format!("{}", callee)));
        } else {
            // TODO: Some kind of data driven solution rather than hardcoded ifs.
            match callee {
                Value::ReferenceId(id) => match &mut self.heap[id] {
                    ReferenceType::Array(ref mut a) => {
                        if builtin == "len" {
                            self.stack.push(Value::Number(a.len() as f64));
                        } else if builtin == "push" {
                            let value = self.stack.pop(current_line)?;
                            a.push(value);
                            self.stack.push(Value::Nil);
                        } else if builtin == "pop" {
                            self.stack.push(a.pop().unwrap());
                        } else if builtin == "remove" {
                            let to_remove = self.stack.pop(current_line)?;
                            if let Value::Number(n) = to_remove {
                                self.stack.push(a.remove(n as usize));
                            } else {
                                return runtime_error(
                                    "Attempt to remove non-integer index from array",
                                    current_line,
                                );
                            }
                        } else if builtin == "insert" {
                            let to_insert_val = self.stack.pop(current_line)?;
                            let to_insert = self.stack.pop(current_line)?;
                            if let Value::Number(n) = to_insert {
                                a.insert(n as usize, to_insert_val);
                                self.stack.push(Value::Nil);
                            } else {
                                return runtime_error(
                                    "Attempt to insert non-integer index from array",
                                    current_line,
                                );
                            }
                        } else if builtin == "sort" {
                            a.sort_by(|a, b| {
                                HashableValue::try_from(a, current_line)
                                    .unwrap()
                                    .cmp(&HashableValue::try_from(b, current_line).unwrap())
                            });
                            self.stack.push(Value::ReferenceId(id));
                        } else if builtin == "resize" {
                            let v = self.stack.pop(current_line)?;
                            let v = if let Value::Number(n) = v {
                                n
                            } else {
                                panic!("Bad arg to array resize.");
                            };
                            a.resize(v as usize, Value::Nil);
                            self.stack.push(Value::Nil);
                        } else {
                            return runtime_error("Unknown array builtin", current_line);
                        }
                    }
                    ReferenceType::External(ref mut e) => {
                        let arity = e.get_arity(&builtin);
                        let mut args = Vec::new();
                        for _ in 0..arity {
                            args.push(self.stack.pop(current_line)?)
                        }
                        let result = e.call(&builtin, args);
                        match result {
                            ValueOrRef::Value(v) => {
                                self.stack.push(v);
                            }
                            ValueOrRef::Ref(rt) => {
                                let id = self.new_reference_type(rt);
                                self.stack.push(Value::ReferenceId(id));
                            }
                        }
                    }
                    _ => return runtime_error("Unknown builtin", current_line),
                },

                Value::String(s) => {
                    if builtin == "len" {
                        self.stack.push(Value::Number(s.len() as f64));
                    } else if builtin == "readFile" {
                        self.stack
                            .push(Value::String(std::fs::read_to_string(s).unwrap()));
                    } else if builtin == "split" {
                        let sep = self.stack.pop(current_line)?;
                        if let Value::String(sep) = sep {
                            let parts: Vec<_> = s
                                .split(&sep)
                                .map(|p| Value::String(p.to_string()))
                                .collect();
                            let id = self.new_reference_type(ReferenceType::Array(parts));
                            self.stack.push(Value::ReferenceId(id));
                        } else {
                            return runtime_error("Expected string argument to split", current_line);
                        }
                    } else if builtin == "parseNumber" {
                        self.stack.push(Value::Number(s.parse().unwrap()));
                    } else if builtin == "regex" {
                        let id = self.new_reference_type(ReferenceType::External(Box::new(
                            regex::Regex::new(&s).unwrap(),
                        )));
                        self.stack.push(Value::ReferenceId(id));
                    } else {
                        return runtime_error("Unknown string builtin", current_line);
                    }
                }

                Value::Number(n) => {
                    if builtin == "floor" {
                        self.stack.push(Value::Number(n.floor()));
                    } else if builtin == "abs" {
                        self.stack.push(Value::Number(n.abs()));
                    } else {
                        return runtime_error("Unknown number builtin", current_line);
                    }
                }

                _ => return runtime_error("Unknown builtin", current_line),
            }
        }
        Ok(())
    }

    fn op_make_range(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let right = if let Value::Number(n) = self.stack.pop(current_line)? {
            n
        } else {
            return runtime_error("Expected number in range bounds", current_line);
        };
        let left = if let Value::Number(n) = self.stack.pop(current_line)? {
            n
        } else {
            return runtime_error("Expected number in range bounds", current_line);
        };
        self.stack.push(Value::Range(left, right));
        Ok(())
    }

    fn op_for_loop(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let local_n = self.read_byte();
        let jump_target = self.read_signed_16();
        let target_ip = (self.ip as isize + jump_target as isize) as usize;
        let range = self.stack.pop(current_line)?;
        match range {
            Value::Range(l, r) => {
                if l < r {
                    self.locals[local_n as usize + self.locals_base] = Value::Number(l);
                    self.stack.push(Value::Range(l + 1.0, r));
                } else {
                    self.ip = target_ip;
                }
            }
            Value::ReferenceId(id) => match &mut self.heap[id] {
                ReferenceType::Array(a) => {
                    if !a.is_empty() {
                        self.locals[local_n as usize + self.locals_base] = Value::Number(0.0);
                        self.stack.push(Value::Range(1.0, a.len() as f64));
                    } else {
                        self.ip = target_ip;
                    }
                }
                ReferenceType::Map(m) => {
                    let keys: Vec<_> = m.keys().cloned().collect();
                    let len = keys.len();
                    if len > 0 {
                        self.locals[local_n as usize + self.locals_base] = Value::from(&keys[0]);
                        self.stack.push(Value::MapForContext(keys, 1.0, len as f64));
                    } else {
                        self.ip = target_ip;
                    }
                }
                _ => return runtime_error("Don't know how to for over that", current_line),
            },
            Value::MapForContext(keys, l, r) => {
                if l < r {
                    self.locals[local_n as usize + self.locals_base] =
                        Value::from(&keys[l as usize]);
                    self.stack.push(Value::MapForContext(keys, l + 1.0, r));
                } else {
                    self.ip = (self.ip as isize + jump_target as isize) as usize;
                }
            }
            _ => return runtime_error("Don't know how to for over that", current_line),
        }
        Ok(())
    }

    fn op_push_map(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let value = self.stack.pop(current_line)?;
        let key = self.stack.pop(current_line)?;
        let map = self.stack.pop(current_line)?;
        if let Value::ReferenceId(id) = map {
            let map = &mut self.heap[id];
            if let ReferenceType::Map(ref mut m) = map {
                m.insert(HashableValue::try_from(&key, current_line)?, value);
            } else {
                return runtime_error("Map push on non-map", current_line);
            }
            self.stack.push(Value::ReferenceId(id));
        } else {
            return runtime_error("Map push on non-map", current_line);
        }
        Ok(())
    }

    fn op_jump_if_true(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let target = self.read_signed_16();
        let value = self.stack.pop(current_line)?;
        if value.is_truey() {
            self.ip = (self.ip as isize + target as isize) as usize;
        }
        Ok(())
    }

    fn op_assign_global(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let global = self.stack.pop(current_line)?;
        let value = self.stack.pop(current_line)?;
        if let Value::String(global_name) = global {
            self.chunk.globals.insert(global_name, value);
        } else {
            return runtime_error("Expected name string for Assign Global.", current_line);
        }
        Ok(())
    }

    fn op_load_global(&mut self, current_line: usize) -> Result<(), InterpreterError> {
        let global = self.stack.pop(current_line)?;
        if let Value::String(global_name) = global {
            self.stack.push(
                self.chunk
                    .globals
                    .get(&global_name)
                    .unwrap_or(&Value::Nil)
                    .clone(),
            );
        } else {
            return runtime_error("Expected name string for Load Global.", current_line);
        }
        Ok(())
    }

    fn read_byte(&mut self) -> u8 {
        self.ip += 1;
        self.chunk.code[self.ip - 1]
    }

    fn _read_signed_byte(&mut self) -> i8 {
        self.read_byte() as i8
    }

    fn read_signed_16(&mut self) -> i16 {
        let number = self.read_byte() as usize;
        let number2 = self.read_byte() as usize;
        (number | number2 << 8) as i16
    }

    fn read_constant(&mut self) -> Value {
        let constant_number = self.read_byte();
        self.chunk.constants[constant_number as usize].clone()
    }

    fn new_reference_type(&mut self, value: ReferenceType) -> usize {
        self.heap.push(value);
        self.heap.len() - 1
    }
}
