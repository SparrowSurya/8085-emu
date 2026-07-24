from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu

    # Initialize register pairs:
    # BC = 0x1234
    # DE = 0x000F
    # HL = 0x1000
    cpu.reg_b.write(Data.byte(0x12))
    cpu.reg_c.write(Data.byte(0x34))
    cpu.reg_d.write(Data.byte(0x00))
    cpu.reg_e.write(Data.byte(0x0F))
    cpu.reg_h.write(Data.byte(0x10))
    cpu.reg_l.write(Data.byte(0x00))

    program = Program([
        Instruction(Opcode.INX_BC),  # BC = 0x1235
        Instruction(Opcode.DCX_DE),  # DE = 0x000E
        Instruction(Opcode.DAD_BC),  # HL = HL + BC = 0x1000 + 0x1235 = 0x2235
        Instruction(Opcode.DAD_DE),  # HL = HL + DE = 0x2235 + 0x000E = 0x2243
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    bc_val = (cpu.reg_b.read().value << 8) | cpu.reg_c.read().value
    de_val = (cpu.reg_d.read().value << 8) | cpu.reg_e.read().value
    hl_val = (cpu.reg_h.read().value << 8) | cpu.reg_l.read().value

    print("16-Bit Register Pair Arithmetic Example State:")
    print(f"Register Pair BC: {hex(bc_val)} (Expected: 0x1235)")
    print(f"Register Pair DE: {hex(de_val)} (Expected: 0x000e)")
    print(f"Register Pair HL: {hex(hl_val)} (Expected: 0x2243)")
    print(f"Carry Flag (CY): {cpu.flag_reg.carry}")

if __name__ == "__main__":
    main_run()
