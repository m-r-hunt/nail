mod chunk;
mod debug;
mod value;
mod vm;

pub fn main() {
    let mut vm = vm::VM::new();
    let mut chunk = chunk::Chunk::new();
    let constant = chunk.add_constant(value::Value(1.2));
    chunk.write_chunk(chunk::OpCode::Constant as u8, 123);
    chunk.write_chunk(constant, 123);
    chunk.write_chunk(chunk::OpCode::Return as u8, 123);
    debug::disassemble_chunk(&chunk, "test chunk");
    vm.interpret(chunk);
}
