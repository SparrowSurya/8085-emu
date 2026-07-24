from emu_8085 import Data, Instruction, Machine, Mem, Opcode, Program, PrinterDevice


def main_run() -> None:
    # 1. Create a printer device with a custom logging callback
    printed_chars = []
    def log_printed_char(char: str):
        printed_chars.append(char)
        print(f"[Printer Output]: '{char}'")

    printer = PrinterDevice(output_callback=log_printed_char)

    # 2. Attach printer to port 0x05
    machine = Machine.create(
        address_lines=16,
        data_lines=8,
        devices=[(printer, [0x05])]
    )

    # 3. Create a program that outputs the letters 'A', 'B', 'C' to the printer
    program = Program([
        Instruction(Opcode.MVI_A, Data.byte(ord('A'))),
        Instruction(Opcode.OUT, Data.byte(0x05)),
        Instruction(Opcode.MVI_A, Data.byte(ord('B'))),
        Instruction(Opcode.OUT, Data.byte(0x05)),
        Instruction(Opcode.MVI_A, Data.byte(ord('C'))),
        Instruction(Opcode.OUT, Data.byte(0x05)),
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x0000))
    machine.run()

    print("\nPrinter Output Example State:")
    print(f"Printed character buffer: {printed_chars}")
    print(f"Printer device history: {printer.history}")

if __name__ == "__main__":
    main_run()
