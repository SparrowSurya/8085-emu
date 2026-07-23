from main import *


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    ram = machine.ram

    # Set SP = 0x1000
    cpu.reg_sp.write(0x1000)

    # Initialize registers
    cpu.reg_b.write(Data.byte(0x11))
    cpu.reg_c.write(Data.byte(0x22))
    cpu.reg_h.write(Data.byte(0xAA))
    cpu.reg_l.write(Data.byte(0xBB))
    cpu.reg_a.write(Data.byte(0x55))

    # Clear CY, set Z, S, P flags in PSW
    cpu.flag_reg.carry = 0
    cpu.flag_reg.zero = 1
    cpu.flag_reg.sign = 1
    cpu.flag_reg.parity = 1

    program = Program([
        Instruction(Opcode.PUSH_B),   # Stack [0x0FFE, 0x0FFF] = [0x22, 0x11], SP = 0x0FFE
        Instruction(Opcode.PUSH_PSW), # Stack [0x0FFC, 0x0FFD] = [Flags, 0x55], SP = 0x0FFC
        Instruction(Opcode.POP_D),    # Pop DE from PSW. D = 0x55, E = Flags, SP = 0x0FFE
        Instruction(Opcode.XTHL),     # Swap HL with top of stack (0x0FFE -> BC value [0x1122])
                                                # HL becomes 0x1122, Stack at 0x0FFE becomes 0xAABB
        Instruction(Opcode.SPHL),     # SP = HL = 0x1122
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("Stack Operations Example State:")
    print(f"Stack Pointer (SP): {hex(cpu.reg_sp.value)} (Expected: 0x1122)")
    print(f"Register D: {hex(cpu.reg_d.value)} (Expected: 0x55)")
    print(f"Register E: {hex(cpu.reg_e.value)}")
    print(f"Register H: {hex(cpu.reg_h.value)} (Expected: 0x11)")
    print(f"Register L: {hex(cpu.reg_l.value)} (Expected: 0x22)")
    print(f"Stack memory at 0x0FFE: {hex(ram.read(Mem(0x0FFE)).value)} (Expected: 0xBB)")
    print(f"Stack memory at 0x0FFF: {hex(ram.read(Mem(0x0FFF)).value)} (Expected: 0xAA)")

if __name__ == "__main__":
    main_run()
