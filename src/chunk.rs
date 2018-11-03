use super::value::Value;

#[derive(Copy, Clone)]
pub enum OpCode {
    Return = 1,
    Constant = 2,
}

impl OpCode {
    pub fn from(val: u8) -> Option<Self> {
        match val {
            1 => Some(OpCode::Return),
            2 => Some(OpCode::Constant),
            _ => None
        }
    }
}

pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk{code: Vec::new(), lines: Vec::new(), constants: Vec::new()}
    }
    
    pub fn write_chunk(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        return (self.constants.len() - 1) as u8
    }
}
