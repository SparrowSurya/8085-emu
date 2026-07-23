from main import *


def main_run() -> None:
    # 1. Initialize machine
    machine = Machine.create(address_lines=16, data_lines=8)
    cpu = machine.cpu
    bus = machine.bus

    # Load a NOP and HLT program
    program = Program([
        Instruction(Opcode.NOP),
        Instruction(Opcode.HLT)
    ])
    machine.load(program, Mem(0x00A0))

    # Set some initial non-zero PC value to check hardware reset
    cpu.reg_pc.write(0x1234)
    cpu.inte = True

    # 2. Assert RESET_IN hardware reset pin on system bus
    print("Asserting RESET_IN = 1")
    bus.reset_in = Data.on()
    machine.tick() # Tick once to trigger hardware reset latching

    print("Checking hardware reset state:")
    print(f"Program Counter (PC): {hex(cpu.reg_pc.value)} (Expected: 0x0000)")
    print(f"Interrupts Enabled (inte): {cpu.inte} (Expected: False)")
    print(f"RESET_OUT Pin on bus: {bus.reset_out.value} (Expected: 1)")

    # De-assert reset
    bus.reset_in = Data.off()
    machine.tick()
    print(f"RESET_OUT Pin after releasing RESET_IN: {bus.reset_out.value} (Expected: 0)")

    # Restore PC
    cpu.reg_pc.write(0x00A0)

    # 3. Demonstrate READY signal wait states insertion
    print("\nDemonstrating READY pin wait states:")
    bus.ready = Data.off() # Peripherals not ready, drive READY = 0

    current_pc = cpu.reg_pc.value
    machine.tick() # Tick the machine cycle

    print(f"Did PC advance when READY=0? {cpu.reg_pc.value == current_pc} (Expected: True - PC does not advance)")
    print(f"CPU Cycle State: {cpu._cycle.name}, T-state: {cpu.t_state}")

    # Set READY back to high
    bus.ready = Data.on()
    machine.tick()
    print(f"Did PC advance after asserting READY=1? {cpu.reg_pc.value != current_pc} (Expected: True)")

if __name__ == "__main__":
    main_run()
