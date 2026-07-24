from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program


def main_run() -> None:
    # Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    ram = machine.ram

    # Program to multiply B * C using successive addition loop
    # B = 7 (multiplicand)
    # C = 6 (multiplier)
    # Result accumulated in A, then stored in memory at address 0x0020
    program = Program([
        Instruction(Opcode.MVI_B, Data.byte(7)),
        Instruction(Opcode.MVI_C, Data.byte(6)),
        Instruction(Opcode.MVI_A, Data.byte(0)),   # Initialize accumulator A = 0

        # LOOP:
        Instruction(Opcode.ADD_B, label="LOOP"),   # A = A + B
        Instruction(Opcode.DCR_C),                 # Decrement C
        Instruction(Opcode.JNZ, "LOOP"),           # If C != 0, JMP back to LOOP

        # Store result and halt
        Instruction(Opcode.STA, Data.words(0x20, 0x00)),
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("Multiplication Example State:")
    print(f"Multiplicand B: {cpu.reg_b.value}")
    print(f"Multiplier   C: {cpu.reg_c.value}")
    print(f"Product      A: {cpu.reg_a.value} (Expected: 42)")
    print(f"Value at 0x0020: {ram.read(Mem(0x0020)).value} (Expected: 42)")

if __name__ == "__main__":
    main_run()
