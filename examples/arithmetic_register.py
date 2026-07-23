from main import *


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    ram = machine.ram

    # Set up HL to point to memory location 0x0100
    cpu.reg_h.write(Data.byte(0x01))
    cpu.reg_l.write(Data.byte(0x00))
    ram.write(Mem(0x0100), Data.byte(0x05)) # Memory operand M = 0x05

    program = Program([
        Instruction(Opcode.MVI_A, Data.byte(0x12)), # A = 0x12
        Instruction(Opcode.MVI_B, Data.byte(0x0E)), # B = 0x0E
        Instruction(Opcode.ADD_B),                      # A = A + B = 0x12 + 0x0E = 0x20
        Instruction(Opcode.ADD_M),                      # A = A + M = 0x20 + 0x05 = 0x25
        Instruction(Opcode.STC),                         # CY = 1
        Instruction(Opcode.ADC_B),                      # A = A + B + CY = 0x25 + 0x0E + 1 = 0x34
        Instruction(Opcode.SUB_B),                      # A = A - B = 0x34 - 0x0E = 0x26
        Instruction(Opcode.STC),                         # CY = 1
        Instruction(Opcode.SBB_M),                      # A = A - M - CY = 0x26 - 0x05 - 1 = 0x20
        Instruction(Opcode.INR_B),                      # B = B + 1 = 0x0F
        Instruction(Opcode.INR_M),                      # Memory at HL [0x0100] = 0x06
        Instruction(Opcode.DCR_B),                      # B = B - 1 = 0x0E
        Instruction(Opcode.DCR_M),                      # Memory at HL [0x0100] = 0x05
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("Arithmetic Register & Memory Example State:")
    print(f"Accumulator (A): {hex(cpu.reg_a.value)} (Expected: 0x20)")
    print(f"Register B: {hex(cpu.reg_b.value)} (Expected: 0x0E)")
    print(f"Memory at 0x0100: {hex(ram.read(Mem(0x0100)).value)} (Expected: 0x05)")
    print(f"Flags: Z={cpu.flag_reg.zero}, CY={cpu.flag_reg.carry}, S={cpu.flag_reg.sign}")

if __name__ == "__main__":
    main_run()
