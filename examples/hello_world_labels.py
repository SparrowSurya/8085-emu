from main import *


def main_run() -> None:
    # 1. Create a printer device with a custom callback that prints to the terminal
    printer = PrinterDevice(output_callback=lambda char: print(char, end=""))

    # 2. Initialize the machine with the printer device attached to port 0x02
    machine = Machine.create(
        address_lines=16,
        data_lines=8,
        devices=[(printer, [0x02])]
    )

    # 3. Create the 8085 assembly program using label referencing
    program = Program([
        Instruction(Opcode.LXI, "STR_DATA"),          # HL points to start of string (using LXI label)

        # LOOP: Load char and check for null
        Instruction(Opcode.MOV_A_M, label="LOOP"),
        Instruction(Opcode.CPI, Data.byte(0x00)),
        Instruction(Opcode.JZ, "EXIT"),               # Jump to EXIT label if character is null

        # Output character, advance pointer, and repeat
        Instruction(Opcode.OUT, Data.byte(0x02)),
        Instruction(Opcode.INX_HL),                   # Point HL to next memory location
        Instruction(Opcode.JMP, "LOOP"),              # Loop back to LOOP label

        # EXIT: Halt execution
        Instruction(Opcode.HLT, label="EXIT"),

        # STR_DATA: Placeholder instruction representing the start of string data
        Instruction(Opcode.NOP, label="STR_DATA")
    ])

    # 4. Load program at entry address 0x0000
    machine.load(program, Mem(0x0000))

    # 5. Load the string message into RAM at the resolved address of "STR_DATA"
    # Instruction sizes: LXI (3), MOV_A_M (1), CPI (2), JZ (3), OUT (2), INX_HL (1), JMP (3), HLT (1).
    # Total size of executable code before STR_DATA is 16 bytes.
    # So STR_DATA is located at 0x0010.
    str_addr = Mem(0x0010)
    message = b"Hello, World using Labels!\n"
    for i, byte_val in enumerate(message):
        machine.ram.write(Mem(str_addr + i), Data.byte(byte_val))
    # Null terminator
    machine.ram.write(Mem(str_addr + len(message)), Data.byte(0x00))

    # 6. Run the machine
    print("--- Executing Hello World with Labels ---")
    machine.run()
    print("-----------------------------------------")

if __name__ == "__main__":
    main_run()
