import unittest

import emu_8085


# NOTE:
# 1) Never use default test memory address or default test data value '0x0'. Due to default
# initial value.
# 2) Inside instruction provide address as Data instead Mem.
class TestDevice(unittest.TestCase):

    def test_keyboard_device(self):
        kbd = emu_8085.KeyboardDevice()
        self.assertEqual(kbd.name, "KeyboardDevice")
        self.assertFalse(kbd.has_key())

        kbd.trigger_key_press('A')
        kbd.trigger_key_press(66)
        self.assertTrue(kbd.has_key())

        self.assertEqual(kbd.port_read(0x01), 65)
        self.assertEqual(kbd.port_read(0x01), 66)
        self.assertEqual(kbd.port_read(0x01), 0x00)
        self.assertFalse(kbd.has_key())

        with self.assertRaises(ValueError):
            kbd.trigger_key_press(128)

        machine = emu_8085.Machine.create(address_lines=16, data_lines=8)
        machine.device_manager.attach_device(kbd, ports=[0x01])

        kbd.trigger_key_press('X')

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.IN, emu_8085.Data.byte(0x01)),
            emu_8085.Instruction(emu_8085.Opcode.MOV_B_A),
            emu_8085.Instruction(emu_8085.Opcode.IN, emu_8085.Data.byte(0x01)),
            emu_8085.Instruction(emu_8085.Opcode.MOV_C_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x00A0))
        machine.run()

        self.assertEqual(machine.cpu.reg_b.value, ord('X'))
        self.assertEqual(machine.cpu.reg_c.value, 0x00)

    def test_keyboard_device_interrupt(self):
        kbd = emu_8085.KeyboardDevice(interrupt_vector=1)
        self.assertEqual(kbd.on_inta(), 0xCF)  # RST 1 opcode = 0xCF

        kbd_vector3 = emu_8085.KeyboardDevice(interrupt_vector=3)
        self.assertEqual(kbd_vector3.on_inta(), 0xDF)  # RST 3 opcode = 0xDF

    def test_hardware_interrupt_trap(self):
        machine = emu_8085.Machine.create(16, 8)
        cpu = machine.cpu
        ram = machine.ram

        cpu.reg_sp.write(0x1000)

        # Set up TRAP ISR at vector 0x0024: MVI A, 0x88; RET
        ram.write(emu_8085.Mem(0x0024), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(0x0025), emu_8085.Data.byte(0x88))
        ram.write(emu_8085.Mem(0x0026), emu_8085.Data.byte(emu_8085.Opcode.RET))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.NOP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x00A0))
        cpu.trap = True

        machine.tick()
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x88)

    def test_dma_hold_hlda(self):
        machine = emu_8085.Machine.create(16, 8)
        bus = machine.bus

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.NOP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, emu_8085.Mem(0x00A0))

        bus.hold = emu_8085.Data.on()
        machine.tick()

        self.assertEqual(bus.hlda.value, 1)

        bus.hold = emu_8085.Data.off()
        machine.tick()

        self.assertEqual(bus.hlda.value, 0)

    def test_hardware_reset(self):
        machine = emu_8085.Machine.create(16, 8)
        bus = machine.bus
        cpu = machine.cpu

        cpu.reg_pc.write(0x1234)
        cpu.inte = True

        bus.reset_in = emu_8085.Data.on()
        machine.tick()

        self.assertEqual(cpu.reg_pc.value, 0x0000)
        self.assertFalse(cpu.inte)
        self.assertEqual(bus.reset_out.value, 1)

        bus.reset_in = emu_8085.Data.off()
        machine.tick()
        self.assertEqual(bus.reset_out.value, 0)


class TestUSBDevice(unittest.TestCase):

    def test_usb_device_dma(self):
        machine = emu_8085.Machine.create(16, 8)
        usb = emu_8085.USBDevice()
        machine.device_manager.attach_device(usb, ports=[0x10])

        self.assertEqual(usb.name, "USBDevice")

        write_data = b"HELLO_DMA"
        usb.dma_write(machine, start_addr=0x0200, data=write_data)

        read_data = usb.dma_read(machine, start_addr=0x0200, length=len(write_data))
        self.assertEqual(read_data, write_data)

    def test_machine_create_with_devices_tuple(self):
        kbd = emu_8085.KeyboardDevice()
        usb = emu_8085.USBDevice()
        machine = emu_8085.Machine.create(
            16, 8,
            devices=[(kbd, [0x01]), (usb, [0x10])]
        )

        self.assertEqual(len(machine.device_manager.devices), 2)
        self.assertIs(machine.device_manager.port_map[0x01], kbd)
        self.assertIs(machine.device_manager.port_map[0x10], usb)


class TestPrinterDevice(unittest.TestCase):

    def test_printer_device_keyboard_pipeline(self):
        printed_output = []

        def custom_printer_callback(char: str):
            printed_output.append(char)

        kbd = emu_8085.KeyboardDevice()
        printer = emu_8085.PrinterDevice(output_callback=custom_printer_callback)

        machine = emu_8085.Machine.create(
            16, 8,
            devices=[(kbd, [0x01]), (printer, [0x02])]
        )

        self.assertEqual(printer.name, "PrinterDevice")

        kbd.trigger_key_press('H')
        kbd.trigger_key_press('I')

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.IN, emu_8085.Data.byte(0x01)),
            emu_8085.Instruction(emu_8085.Opcode.OUT, emu_8085.Data.byte(0x02)),
            emu_8085.Instruction(emu_8085.Opcode.IN, emu_8085.Data.byte(0x01)),
            emu_8085.Instruction(emu_8085.Opcode.OUT, emu_8085.Data.byte(0x02)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x00A0))
        machine.run()

        self.assertEqual(printed_output, ['H', 'I'])
        self.assertEqual(printer.history, ['H', 'I'])


