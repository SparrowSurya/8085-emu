from main import *


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu

    program = Program([
        Instruction(Opcode.MVI_A, Data.byte(0x10)),
        Instruction(Opcode.ADI, Data.byte(0x20)),   # A = 0x30
        Instruction(Opcode.ACI, Data.byte(0x05)),   # A = 0x35
        Instruction(Opcode.STC),                         # Set CY = 1
        Instruction(Opcode.ACI, Data.byte(0x05)),   # A = 0x3B
        Instruction(Opcode.SUI, Data.byte(0x10)),   # A = 0x2B
        Instruction(Opcode.STC),                         # Set CY = 1
        Instruction(Opcode.SBI, Data.byte(0x0A)),   # A = 0x20
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("Arithmetic Immediate Example State:")
    print(f"Accumulator (A): {hex(cpu.reg_a.value)} (Expected: 0x20)")
    print(f"Carry Flag (CY): {cpu.flag_reg.carry}")
    print(f"Zero Flag (Z): {cpu.flag_reg.zero}")
    print(f"Sign Flag (S): {cpu.flag_reg.sign}")

if __name__ == "__main__":
    main_run()
