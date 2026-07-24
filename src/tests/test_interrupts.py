import unittest

import emu_8085


# NOTE:
# 1) Never use default test memory address or default test data value '0x0'. Due to default
# initial value.
# 2) Inside instruction provide address as Data instead Mem.
class TestSoftwareInterrupt(unittest.TestCase):

    def test_software_interrupt_vectors(self):
        rst_vectors = [
            (emu_8085.Opcode.RST_0, emu_8085.VEC_RST_0),
            (emu_8085.Opcode.RST_1, emu_8085.VEC_RST_1),
            (emu_8085.Opcode.RST_2, emu_8085.VEC_RST_2),
            (emu_8085.Opcode.RST_3, emu_8085.VEC_RST_3),
            (emu_8085.Opcode.RST_4, emu_8085.VEC_RST_4),
            (emu_8085.Opcode.RST_5, emu_8085.VEC_RST_5),
            (emu_8085.Opcode.RST_6, emu_8085.VEC_RST_6),
            (emu_8085.Opcode.RST_7, emu_8085.VEC_RST_7),
        ]

        for rst_op, expected_vector in rst_vectors:
            with self.subTest(rst=rst_op.name):
                machine = emu_8085.Machine.create(16, 8)
                cpu, ram = machine.cpu, machine.ram
                cpu.reg_sp.write(0x1000)

                isr_addr = expected_vector
                ram.write(isr_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
                ram.write(emu_8085.Mem(isr_addr + 1), emu_8085.Data.byte(0x55))
                ram.write(emu_8085.Mem(isr_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

                program = emu_8085.Program([
                    emu_8085.Instruction(rst_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, emu_8085.Mem(0x00A0))
                machine.run()

                self.assertEqual(cpu.reg_a.value, 0x55)
                self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_nested_software_interrupts(self):
        machine = emu_8085.Machine.create(16, 8)
        cpu, ram = machine.cpu, machine.ram
        cpu.reg_sp.write(0x1000)

        ram.write(emu_8085.VEC_RST_1, emu_8085.Data.byte(emu_8085.Opcode.MVI_B))
        ram.write(emu_8085.Mem(emu_8085.VEC_RST_1 + 1), emu_8085.Data.byte(0x11))
        ram.write(emu_8085.Mem(emu_8085.VEC_RST_1 + 2), emu_8085.Data.byte(emu_8085.Opcode.RST_3))
        ram.write(emu_8085.Mem(emu_8085.VEC_RST_1 + 3), emu_8085.Data.byte(emu_8085.Opcode.RET))

        ram.write(emu_8085.VEC_RST_3, emu_8085.Data.byte(emu_8085.Opcode.MVI_C))
        ram.write(emu_8085.Mem(emu_8085.VEC_RST_3 + 1), emu_8085.Data.byte(0x33))
        ram.write(emu_8085.Mem(emu_8085.VEC_RST_3 + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.RST_1),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x00A0))
        machine.run()

        self.assertEqual(cpu.reg_b.value, 0x11)
        self.assertEqual(cpu.reg_c.value, 0x33)
        self.assertEqual(cpu.reg_sp.value, 0x1000)


class TestHardwareInterrupt(unittest.TestCase):

    def test_trap_interrupt(self):
        machine = emu_8085.Machine.create(16, 8)
        cpu, ram = machine.cpu, machine.ram
        cpu.reg_sp.write(0x1000)

        ram.write(emu_8085.VEC_TRAP, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(emu_8085.VEC_TRAP + 1), emu_8085.Data.byte(0x88))
        ram.write(emu_8085.Mem(emu_8085.VEC_TRAP + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.NOP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x00A0))
        cpu.trap = True

        machine.tick()
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x88)

    def test_rst_7_5_6_5_5_5_interrupts(self):
        tests_data = [
            ("rst_7_5", emu_8085.VEC_RST_7_5, 0x75),
            ("rst_6_5", emu_8085.VEC_RST_6_5, 0x65),
            ("rst_5_5", emu_8085.VEC_RST_5_5, 0x55),
        ]

        for pin_name, vector_addr, reg_val in tests_data:
            with self.subTest(pin=pin_name):
                machine = emu_8085.Machine.create(16, 8)
                cpu, ram = machine.cpu, machine.ram
                cpu.reg_sp.write(0x1000)
                cpu.inte = True

                ram.write(emu_8085.Mem(vector_addr), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
                ram.write(emu_8085.Mem(vector_addr + 1), emu_8085.Data.byte(reg_val))
                ram.write(emu_8085.Mem(vector_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.NOP),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, emu_8085.Mem(0x00A0))
                setattr(cpu, pin_name, True)

                machine.tick()
                machine.run()

                self.assertEqual(cpu.reg_a.value, reg_val)

    def test_intr_inta_cycle(self):
        kbd = emu_8085.KeyboardDevice(interrupt_vector=2)
        machine = emu_8085.Machine.create(16, 8, devices=[(kbd, [0x01])])
        cpu, ram = machine.cpu, machine.ram
        cpu.reg_sp.write(0x1000)

        ram.write(emu_8085.Mem(0x0010), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(0x0011), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(0x0012), emu_8085.Data.byte(emu_8085.Opcode.RET))

        kbd.trigger_key_press('K')

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.EI),
            emu_8085.Instruction(emu_8085.Opcode.NOP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x00A0))
        cpu.intr = True

        machine.run()

        self.assertEqual(kbd.on_inta(), 0xD7)


