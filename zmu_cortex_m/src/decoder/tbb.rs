use core::instruction::Instruction;
use bit_field::BitField;
use core::register::Reg;

#[allow(non_snake_case)]
pub fn decode_TBB_t1(opcode: u32) -> Instruction {
    let rn = opcode.get_bits(16..20);
    let rm = opcode.get_bits(0..4);

    Instruction::TBB {
        rn: Reg::from(rn),
        rm: Reg::from(rm)
    }
}
