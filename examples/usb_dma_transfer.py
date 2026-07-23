from main import *


def main_run() -> None:
    # 1. Create a USB device (supporting DMA transfers)
    usb = USBDevice()

    # 2. Initialize the machine with the USB device attached to port 0x10
    machine = Machine.create(
        address_lines=16,
        data_lines=8,
        devices=[(usb, [0x10])]
    )
    ram = machine.ram

    # Set up some program in memory
    program = Program([
        Instruction(Opcode.NOP),
        Instruction(Opcode.HLT)
    ])
    machine.load(program, Mem(0x00A0))

    # 3. Simulate high-speed USB writing data directly to RAM via DMA protocol
    # The DMA protocol asserts HOLD -> CPU grants HLDA -> USB writes directly -> Release HOLD
    write_data = b"USB_DMA_PACKET"
    print(f"USB writing to memory at 0x0200 via DMA: {write_data}")
    usb.dma_write(machine, start_addr=0x0200, data=write_data)

    # 4. Simulate high-speed USB reading data back from RAM via DMA protocol
    read_data = usb.dma_read(machine, start_addr=0x0200, length=len(write_data))
    print(f"USB read back from memory at 0x0200 via DMA: {read_data}")

    # Verify that CPU yields control and memory is updated
    print("\nUSB DMA Example State:")
    print(f"Read Match: {read_data == write_data}")
    print(f"Memory at 0x0200: {bytes(ram.read(Mem(0x0200 + i)).value for i in range(len(write_data)))}")

if __name__ == "__main__":
    main_run()
