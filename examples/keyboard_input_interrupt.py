from main import *


def main_run() -> None:
    # 1. Create a keyboard device that triggers interrupt vector 1 (RST 1 -> vector 0x0008)
    kbd = KeyboardDevice(interrupt_vector=1)

    # 2. Attach keyboard device to machine on Port 0x01
    machine = Machine.create(
        address_lines=16,
        data_lines=8,
        devices=[(kbd, [0x01])]
    )
    cpu = machine.cpu
    ram = machine.ram
    cpu.reg_sp.write(0x1000)

    # 3. Write ISR for RST 1 at 0x0008:
    # 0x0008: IN 0x01   - Read ASCII key value from keyboard port into A
    # 0x000A: MOV B, A  - Save input key in Register B
    # 0x000B: RET       - Return from interrupt handler
    ram.write(Mem(0x0008), Data.byte(Opcode.IN))
    ram.write(Mem(0x0009), Data.byte(0x01))
    ram.write(Mem(0x000A), Data.byte(Opcode.MOV_B_A))
    ram.write(Mem(0x000B), Data.byte(Opcode.RET))

    # 4. Main Program: Enable interrupts and NOP in loop
    program = Program([
        Instruction(Opcode.EI),  # Enable Interrupts (inte = True)
        Instruction(Opcode.NOP),
        Instruction(Opcode.HLT)
    ])

    machine.load(program, Mem(0x00A0))

    # Trigger a keyboard keypress event
    kbd.trigger_key_press('K')

    # Emulate the hardware interrupt request asserting INTR pin
    cpu.intr = True

    # 5. Run the machine
    machine.run()

    print("Keyboard Input & Interrupt Example State:")
    print(f"Register B: {hex(cpu.reg_b.value)} (Expected: 0x4b - ASCII 'K')")
    print(f"Interrupts Enabled (inte): {cpu.inte}")

if __name__ == "__main__":
    main_run()
