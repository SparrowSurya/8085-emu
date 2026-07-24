from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    ram = machine.ram
    cpu.reg_sp.write(0x1000) # SP points to 0x1000

    # Subroutine at address 0x00A0:
    # 0x00A0: MVI B, 0xAA
    # 0x00A2: RET
    ram.write(Mem(0x00A0), Data.byte(Opcode.MVI_B))
    ram.write(Mem(0x00A1), Data.byte(0xAA))
    ram.write(Mem(0x00A2), Data.byte(Opcode.RET))

    # Conditional Jump Target at 0x00B0:
    # 0x00B0: MVI C, 0xBB
    # 0x00B2: HLT
    ram.write(Mem(0x00B0), Data.byte(Opcode.MVI_C))
    ram.write(Mem(0x00B1), Data.byte(0xBB))
    ram.write(Mem(0x00B2), Data.byte(Opcode.HLT))

    program = Program([
        Instruction(Opcode.CALL, Data.words(0xA0, 0x00)),     # Call subroutine at 0x00A0
        Instruction(Opcode.CPI, Data.byte(0x00)),              # CPI 0x00 (Sets Zero Flag since A=0)
        Instruction(Opcode.JZ, Data.words(0xB0, 0x00)),       # Jump if Zero to 0x00B0 (will jump)
        Instruction(Opcode.HLT)                                # Should not be executed
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("Branching & Control Example State:")
    print(f"Register B: {hex(cpu.reg_b.value)} (Expected: 0xaa from subroutine)")
    print(f"Register C: {hex(cpu.reg_c.value)} (Expected: 0xbb from conditional jump)")
    print(f"Program Counter (PC): {hex(cpu.reg_pc.value)}")
    print(f"Stack Pointer (SP): {hex(cpu.reg_sp.value)}")

if __name__ == "__main__":
    main_run()
