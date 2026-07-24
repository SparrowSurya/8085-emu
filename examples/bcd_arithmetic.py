from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu

    # Test BCD addition:
    # 0x38 + 0x45 = 0x7D (binary sum)
    # Applying DAA (Decimal Adjust Accumulator) corrects 0x7D to BCD 0x83
    # (with Auxiliary Carry and Carry updated properly)
    program = Program([
        Instruction(Opcode.MVI_A, Data.byte(0x38)), # Load BCD 38 into A
        Instruction(Opcode.ADI, Data.byte(0x45)),   # Add BCD 45 -> binary sum 0x7D
        Instruction(Opcode.DAA),                         # Adjust to BCD -> 0x83
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("BCD Arithmetic & DAA Example State:")
    print(f"Accumulator (A): {hex(cpu.reg_a.value)} (Expected BCD result: 0x83)")
    print(f"Flags: CY={cpu.flag_reg.carry}, Z={cpu.flag_reg.zero}, S={cpu.flag_reg.sign}")

if __name__ == "__main__":
    main_run()
