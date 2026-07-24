from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    ram = machine.ram

    # Set up HL pointing to memory location 0x0100
    cpu.reg_h.write(Data.byte(0x01))
    cpu.reg_l.write(Data.byte(0x00))
    ram.write(Mem(0x0100), Data.byte(0x55)) # M = 0x55

    program = Program([
        Instruction(Opcode.MVI_A, Data.byte(0xFF)), # A = 0xFF
        Instruction(Opcode.MVI_B, Data.byte(0x0A)), # B = 0x0A
        Instruction(Opcode.ANA_B),                      # A = A & B = 0x0A
        Instruction(Opcode.ANI, Data.byte(0x0F)),   # A = A & 0x0F = 0x0A
        Instruction(Opcode.ORA_M),                      # A = A | M = 0x0A | 0x55 = 0x5F
        Instruction(Opcode.ORI, Data.byte(0x80)),   # A = A | 0x80 = 0xDF
        Instruction(Opcode.XRA_B),                      # A = A ^ B = 0xDF ^ 0x0A = 0xD5
        Instruction(Opcode.XRI, Data.byte(0xD5)),   # A = A ^ 0xD5 = 0x00 (Zero flag set to 1)
        Instruction(Opcode.MVI_A, Data.byte(0x10)), # A = 0x10
        Instruction(Opcode.CMP_B),                      # Compare A with B (0x10 > 0x0A -> Carry clear, Zero clear)
        Instruction(Opcode.CPI, Data.byte(0x10)),   # Compare A with 0x10 (0x10 == 0x10 -> Zero set, Carry clear)
        Instruction(Opcode.CMA),                         # Complement A (A = ~0x10 = 0xEF)
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("Logical Operations Example State:")
    print(f"Accumulator (A): {hex(cpu.reg_a.value)} (Expected: 0xef)")
    print(f"Zero Flag (Z): {cpu.flag_reg.zero}")
    print(f"Carry Flag (CY): {cpu.flag_reg.carry}")
    print(f"Sign Flag (S): {cpu.flag_reg.sign}")
    print(f"Parity Flag (P): {cpu.flag_reg.parity}")

if __name__ == "__main__":
    main_run()
