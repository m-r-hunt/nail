use super::chunk::*;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut i = 0;
    while i < chunk.code.len() {
        i = disassemble_instruction(chunk, i);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    let instr = chunk.code[offset];
    print!("{:04x} ", offset);
    if offset > 0 && chunk.lines[offset] == chunk.lines[offset-1] {
        print!("   | ");
    } else {
        print!("{:4} ", chunk.lines[offset]);
    }

    match OpCode::from(instr) {
        Some(OpCode::Return) => simple_instruction("OP_RETURN", offset),
        Some(OpCode::Constant) => constant_instruction("OP_CONSTANT", &chunk, offset),
        None => {
            println!("Unknown opcode {}", instr);
            offset+1
        }
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    return offset+1
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let constant = chunk.code[offset+1];
    println!("{} {} '{}'", name, constant, chunk.constants[constant as usize]);
    return offset+2
}
