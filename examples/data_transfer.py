from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    ram = machine.ram

    # Set initial register values
    cpu.reg_a.write(Data.byte(0xAA))
    cpu.reg_b.write(Data.byte(0x11))
    cpu.reg_c.write(Data.byte(0x22))
    cpu.reg_d.write(Data.byte(0x33))
    cpu.reg_e.write(Data.byte(0x44))
    cpu.reg_h.write(Data.byte(0x01))
    cpu.reg_l.write(Data.byte(0x00)) # HL points to 0x0100

    # Write a test byte to memory 0x0100
    ram.write(Mem(0x0100), Data.byte(0x99))

    # Assembly Program covering various data transfers
    program = Program([
        Instruction(Opcode.MOV_B_A),            # MOV B, A (B becomes 0xAA)
        Instruction(Opcode.MOV_A_M),            # MOV A, M (A reads from [HL] = 0x99)
        Instruction(Opcode.MVI_C, Data.byte(0x55)), # MVI C, 0x55
        Instruction(Opcode.MOV_M_C),            # MOV M, C (memory at [HL] becomes 0x55)
        Instruction(Opcode.LDA_DE),             # LDAX D (Load Accumulator from memory at DE [0x3344])
        Instruction(Opcode.STA_BC),             # STAX B (Store Accumulator to memory at BC [0xAA22])
        Instruction(Opcode.LHLD, Data.words(0x00, 0x02)), # LHLD 0x0200 (Load HL from memory 0x0200)
        Instruction(Opcode.SHLD, Data.words(0x00, 0x03)), # SHLD 0x0300 (Store HL to memory 0x0300)
        Instruction(Opcode.XCHG),               # XCHG (Exchange HL and DE)
        Instruction(Opcode.HLT)
    ])

    # Write source data to memory locations
    ram.write(Mem(0x3344), Data.byte(0x77)) # DE pointer source
    ram.write(Mem(0x0200), Data.byte(0xEF)) # L value
    ram.write(Mem(0x0201), Data.byte(0xBE)) # H value (HL = 0xBEEF)

    # Load and run program
    machine.load(program, Mem(0x0000))
    machine.run()

    print("Data Transfer Example State:")
    print(f"Register A: {hex(cpu.reg_a.value)}")
    print(f"Register B: {hex(cpu.reg_b.value)}")
    print(f"Register C: {hex(cpu.reg_c.value)}")
    print(f"Register D: {hex(cpu.reg_d.value)}")
    print(f"Register E: {hex(cpu.reg_e.value)}")
    print(f"Register H: {hex(cpu.reg_h.value)}")
    print(f"Register L: {hex(cpu.reg_l.value)}")
    print(f"Memory at 0x0100: {hex(ram.read(Mem(0x0100)).value)}")
    print(f"Memory at 0xAA22: {hex(ram.read(Mem(0xAA22)).value)}")
    print(f"Memory at 0x0300: {hex(ram.read(Mem(0x0300)).value)}")
    print(f"Memory at 0x0301: {hex(ram.read(Mem(0x0301)).value)}")

if __name__ == "__main__":
    main_run()
