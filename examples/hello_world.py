from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program, PrinterDevice


def main_run() -> None:
    # 1. Create a printer device with a custom callback that prints to the terminal
    printer = PrinterDevice(output_callback=lambda char: print(char, end=""))

    # 2. Initialize the machine with the printer device attached to port 0x02
    machine = Machine.create(
        address_lines=16,
        data_lines=8,
        devices=[(printer, [0x02])]
    )

    # 3. Create the 8085 assembly program
    # This program reads characters starting at memory address 0x0100 and outputs
    # them to the printer device (port 0x02) until it encounters a null terminator (0x00).
    program = Program([
        Instruction(Opcode.MVI_HL, Data.words(0x00, 0x01)), # HL points to start of string (0x0100)
        # loop:
        Instruction(Opcode.MOV_A_M),                 # Load character from memory to A (at address 0x0003)
        Instruction(Opcode.CPI, Data.byte(0x00)),# Compare character with null (0x00)
        Instruction(Opcode.JZ, Data.words(0x0F, 0x00)),    # If zero, jump to HLT (address 0x000F)
        Instruction(Opcode.OUT, Data.byte(0x02)),# Output character to printer port 0x02
        Instruction(Opcode.INX_HL),                   # Point HL to next memory location
        Instruction(Opcode.JMP, Data.words(0x03, 0x00)),    # Jump back to loop start (address 0x0003)
        # end:
        Instruction(Opcode.HLT)                      # Halt the CPU (at address 0x000F)
    ])

    # 4. Load program at entry address 0x0000
    machine.load(program, Mem(0x0000))

    # 5. Load "Hello, World!\x00" into memory starting at 0x0100
    message = b"Hello, World!\n"
    for i, byte_val in enumerate(message):
        machine.ram.write(Mem(0x0100 + i), Data.byte(byte_val))
    # Null terminator
    machine.ram.write(Mem(0x0100 + len(message)), Data.byte(0x00))

    # 6. Run the machine
    print("--- Executing Hello World Program ---")
    machine.run()
    print("-------------------------------------")

if __name__ == "__main__":
    main_run()
