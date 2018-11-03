use super::{chunk, debug, value};

pub struct VM {
    chunk: chunk::Chunk,
    ip: usize,
}

pub enum InterpreterError {
    CompileError,
    RuntimeError,
}

impl VM {
    pub fn new() -> VM {
        VM{chunk: chunk::Chunk::new(), ip: 0}
    }

    pub fn interpret(&mut self, chunk: chunk::Chunk) -> Result<(), InterpreterError> {
        self.chunk = chunk;
        self.ip = 0;
        self.run()
    }

    pub fn run(&mut self) -> Result<(), InterpreterError> {
        loop {
            if cfg!(feature="debugTraceExecution") {
                debug::disassemble_instruction(&self.chunk, self.ip);
            }
            let instruction = self.read_byte();
            match chunk::OpCode::from(instruction) {
                Some(chunk::OpCode::Return) => return Ok(()),
                Some(chunk::OpCode::Constant) => {
                    let constant = self.read_constant();
                    println!("{}", constant);
                },
                None => return Err(InterpreterError::CompileError),
            }
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        self.ip += 1;
        self.chunk.code[self.ip-1]
    }

    pub fn read_constant(&mut self) -> value::Value {
        let constant_number = self.read_byte();
        self.chunk.constants[constant_number as usize]
    }
}
