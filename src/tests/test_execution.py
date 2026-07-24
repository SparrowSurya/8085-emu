import unittest

import emu_8085


# NOTE:
# 1) Never use default test memory address or default test data value '0x0'. Due to default
# initial value.
# 2) Inside instruction provide address as Data instead Mem.
class TestExecution(unittest.TestCase):

    address_lines: int = 16
    data_lines: int = 8
    mem_exec_entry: emu_8085.Mem = emu_8085.Mem(0x00A0)

    _machine: emu_8085.Machine | None


    def setUp(self):
        super().setUp()
        self._machine = emu_8085.Machine.create(
            address_lines=self.address_lines,
            data_lines=self.data_lines,
        )

    def tearDown(self) -> None:
        super().tearDown()
        self._machine = None

    @property
    def machine(self) -> emu_8085.Machine:
        assert(self._machine is not None)
        return self._machine


    def test_move_immediate_to_register(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x01)),
            (cpu.reg_b, emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x02)),
            (cpu.reg_c, emu_8085.Opcode.MVI_C, emu_8085.Data.byte(0x03)),
            (cpu.reg_d, emu_8085.Opcode.MVI_D, emu_8085.Data.byte(0x04)),
            (cpu.reg_e, emu_8085.Opcode.MVI_E, emu_8085.Data.byte(0x05)),
            (cpu.reg_h, emu_8085.Opcode.MVI_H, emu_8085.Data.byte(0x06)),
            (cpu.reg_l, emu_8085.Opcode.MVI_L, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(emu_8085.Instruction(opcode, arg) for _, opcode, arg in data),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_immediate_to_register_pair(self):
        cpu = self.machine.cpu
        data = [
            ((cpu.reg_b, cpu.reg_c), emu_8085.Opcode.MVI_BC, emu_8085.Data.words(0x01, 0x02)),
            ((cpu.reg_d, cpu.reg_e), emu_8085.Opcode.MVI_DE, emu_8085.Data.words(0x03, 0x04)),
            ((cpu.reg_h, cpu.reg_l), emu_8085.Opcode.MVI_HL, emu_8085.Data.words(0x05, 0x06)),
        ]
        program = emu_8085.Program([
            *(emu_8085.Instruction(opcode, arg) for _, opcode, arg in data),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        m1 = emu_8085.Mask.bits(8, 0)
        m2 = emu_8085.Mask.bits(8, 8)

        for (reg1, reg2), opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg1.value, m1.apply(val).value)
                self.assertEqual(reg2.value, m2.apply(val).value)


    def test_move_to_memory_from_register(self):
        ram = self.machine.ram
        data = [
            (emu_8085.Opcode.MVI_A, emu_8085.Opcode.MOV_M_A, emu_8085.Data.byte(0x01), emu_8085.Data.words(0x01, 0x00)),
            (emu_8085.Opcode.MVI_B, emu_8085.Opcode.MOV_M_B, emu_8085.Data.byte(0x02), emu_8085.Data.words(0x02, 0x00)),
            (emu_8085.Opcode.MVI_C, emu_8085.Opcode.MOV_M_C, emu_8085.Data.byte(0x03), emu_8085.Data.words(0x03, 0x00)),
            (emu_8085.Opcode.MVI_D, emu_8085.Opcode.MOV_M_D, emu_8085.Data.byte(0x04), emu_8085.Data.words(0x04, 0x00)),
            (emu_8085.Opcode.MVI_E, emu_8085.Opcode.MOV_M_E, emu_8085.Data.byte(0x05), emu_8085.Data.words(0x05, 0x00)),
            (emu_8085.Opcode.MVI_H, emu_8085.Opcode.MOV_M_H, emu_8085.Data.byte(0x06), emu_8085.Data.words(0x06, 0x00)),
            (emu_8085.Opcode.MVI_L, emu_8085.Opcode.MOV_M_L, emu_8085.Data.byte(0x07), emu_8085.Data.words(0x07, 0x00)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for o1, o2, val, mem in data
                for instruction in [
                    emu_8085.Instruction(o1, val),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, mem),
                    emu_8085.Instruction(o2),
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for _, o2, val, ptr in data[:5]:
            with self.subTest(op=o2.name):
                mem = emu_8085.Mem(ptr.reverse().value)
                self.assertEqual(val.value, ram.read(mem).value)

        _, o2, val, ptr = data[5]
        with self.subTest(op=o2.name):
            mem = emu_8085.Mem(ptr.reverse().value)
            self.assertEqual(0x00, ram.read(mem).value)

        _, o2, val, ptr = data[6]
        with self.subTest(op=o2.name):
            mem = emu_8085.Mem(ptr.reverse().value)
            self.assertEqual(0x07, ram.read(mem).value)


    def test_move_to_register_from_register_a(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_b, emu_8085.Opcode.MOV_B_A, emu_8085.Data.byte(0x02)),
            (cpu.reg_c, emu_8085.Opcode.MOV_C_A, emu_8085.Data.byte(0x03)),
            (cpu.reg_d, emu_8085.Opcode.MOV_D_A, emu_8085.Data.byte(0x04)),
            (cpu.reg_e, emu_8085.Opcode.MOV_E_A, emu_8085.Data.byte(0x05)),
            (cpu.reg_h, emu_8085.Opcode.MOV_H_A, emu_8085.Data.byte(0x06)),
            (cpu.reg_l, emu_8085.Opcode.MOV_L_A, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_b(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MOV_A_B, emu_8085.Data.byte(0x01)),
            (cpu.reg_c, emu_8085.Opcode.MOV_C_B, emu_8085.Data.byte(0x03)),
            (cpu.reg_d, emu_8085.Opcode.MOV_D_B, emu_8085.Data.byte(0x04)),
            (cpu.reg_e, emu_8085.Opcode.MOV_E_B, emu_8085.Data.byte(0x05)),
            (cpu.reg_h, emu_8085.Opcode.MOV_H_B, emu_8085.Data.byte(0x06)),
            (cpu.reg_l, emu_8085.Opcode.MOV_L_B, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_B, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_c(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MOV_A_C, emu_8085.Data.byte(0x01)),
            (cpu.reg_b, emu_8085.Opcode.MOV_B_C, emu_8085.Data.byte(0x02)),
            (cpu.reg_d, emu_8085.Opcode.MOV_D_C, emu_8085.Data.byte(0x04)),
            (cpu.reg_e, emu_8085.Opcode.MOV_E_C, emu_8085.Data.byte(0x05)),
            (cpu.reg_h, emu_8085.Opcode.MOV_H_C, emu_8085.Data.byte(0x06)),
            (cpu.reg_l, emu_8085.Opcode.MOV_L_C, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_C, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_d(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MOV_A_D, emu_8085.Data.byte(0x01)),
            (cpu.reg_b, emu_8085.Opcode.MOV_B_D, emu_8085.Data.byte(0x02)),
            (cpu.reg_c, emu_8085.Opcode.MOV_C_D, emu_8085.Data.byte(0x03)),
            (cpu.reg_e, emu_8085.Opcode.MOV_E_D, emu_8085.Data.byte(0x05)),
            (cpu.reg_h, emu_8085.Opcode.MOV_H_D, emu_8085.Data.byte(0x06)),
            (cpu.reg_l, emu_8085.Opcode.MOV_L_D, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_D, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_e(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MOV_A_E, emu_8085.Data.byte(0x01)),
            (cpu.reg_b, emu_8085.Opcode.MOV_B_E, emu_8085.Data.byte(0x02)),
            (cpu.reg_c, emu_8085.Opcode.MOV_C_E, emu_8085.Data.byte(0x03)),
            (cpu.reg_d, emu_8085.Opcode.MOV_D_E, emu_8085.Data.byte(0x04)),
            (cpu.reg_h, emu_8085.Opcode.MOV_H_E, emu_8085.Data.byte(0x06)),
            (cpu.reg_l, emu_8085.Opcode.MOV_L_E, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_E, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_h(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MOV_A_H, emu_8085.Data.byte(0x01)),
            (cpu.reg_b, emu_8085.Opcode.MOV_B_H, emu_8085.Data.byte(0x02)),
            (cpu.reg_c, emu_8085.Opcode.MOV_C_H, emu_8085.Data.byte(0x03)),
            (cpu.reg_d, emu_8085.Opcode.MOV_D_H, emu_8085.Data.byte(0x04)),
            (cpu.reg_e, emu_8085.Opcode.MOV_E_H, emu_8085.Data.byte(0x05)),
            (cpu.reg_l, emu_8085.Opcode.MOV_L_H, emu_8085.Data.byte(0x07)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_H, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_l(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, emu_8085.Opcode.MOV_A_L, emu_8085.Data.byte(0x01)),
            (cpu.reg_b, emu_8085.Opcode.MOV_B_L, emu_8085.Data.byte(0x02)),
            (cpu.reg_c, emu_8085.Opcode.MOV_C_L, emu_8085.Data.byte(0x03)),
            (cpu.reg_d, emu_8085.Opcode.MOV_D_L, emu_8085.Data.byte(0x04)),
            (cpu.reg_e, emu_8085.Opcode.MOV_E_L, emu_8085.Data.byte(0x05)),
            (cpu.reg_h, emu_8085.Opcode.MOV_H_L, emu_8085.Data.byte(0x06)),
        ]
        program = emu_8085.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    emu_8085.Instruction(emu_8085.Opcode.MVI_L, val),
                    emu_8085.Instruction(opcode)
                ]
            ),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_add_register(self):
        cpu = self.machine.cpu
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Opcode.MVI_B, emu_8085.Opcode.ADD_B, emu_8085.Data.byte(0x03), 0x08, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Opcode.MVI_C, emu_8085.Opcode.ADD_C, emu_8085.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0xFF), emu_8085.Opcode.MVI_D, emu_8085.Opcode.ADD_D, emu_8085.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (emu_8085.Data.byte(0x70), emu_8085.Opcode.MVI_E, emu_8085.Opcode.ADD_E, emu_8085.Data.byte(0x10), 0x80, 0, 0, 1, 0, 0),
            (emu_8085.Data.byte(0x0F), emu_8085.Opcode.MVI_H, emu_8085.Opcode.ADD_H, emu_8085.Data.byte(0x01), 0x10, 0, 0, 0, 0, 1),
            (emu_8085.Data.byte(0x01), emu_8085.Opcode.MVI_L, emu_8085.Opcode.ADD_L, emu_8085.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x10), emu_8085.Opcode.MVI_A, emu_8085.Opcode.ADD_A, emu_8085.Data.byte(0x10), 0x20, 0, 0, 0, 0, 0),
        ]

        for val_a, mvi_src, add_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=add_op.name):
                instructions = [emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a)]
                if add_op != emu_8085.Opcode.ADD_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(add_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                self.machine.load(program, self.mem_exec_entry)
                self.machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_add_memory(self):
        ram = self.machine.ram
        cpu = self.machine.cpu

        data = [
            (emu_8085.Data.byte(0x03), emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x05), 0x08, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0xFF), emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (emu_8085.Data.byte(0x70), emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x10), 0x80, 0, 0, 1, 0, 0),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                ram.write(mem, val_mem)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.ADD_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                self.machine.load(program, self.mem_exec_entry)
                self.machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_add_register_with_carry(self):
        data = [
            (emu_8085.Data.byte(0x05), 0, emu_8085.Opcode.MVI_B, emu_8085.Opcode.ADC_B, emu_8085.Data.byte(0x03), 0x08, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x05), 1, emu_8085.Opcode.MVI_C, emu_8085.Opcode.ADC_C, emu_8085.Data.byte(0x03), 0x09, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0xFE), 1, emu_8085.Opcode.MVI_D, emu_8085.Opcode.ADC_D, emu_8085.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (emu_8085.Data.byte(0x7F), 1, emu_8085.Opcode.MVI_E, emu_8085.Opcode.ADC_E, emu_8085.Data.byte(0x00), 0x80, 0, 0, 1, 0, 1),
            (emu_8085.Data.byte(0x0F), 1, emu_8085.Opcode.MVI_H, emu_8085.Opcode.ADC_H, emu_8085.Data.byte(0x00), 0x10, 0, 0, 0, 0, 1),
            (emu_8085.Data.byte(0x01), 1, emu_8085.Opcode.MVI_L, emu_8085.Opcode.ADC_L, emu_8085.Data.byte(0x01), 0x03, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x10), 1, emu_8085.Opcode.MVI_A, emu_8085.Opcode.ADC_A, emu_8085.Data.byte(0x10), 0x21, 0, 0, 0, 1, 0),
        ]

        for val_a, init_carry, mvi_src, adc_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=adc_op.name, carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.append(emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a))
                if adc_op != emu_8085.Opcode.ADC_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(adc_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_add_memory_with_carry(self):
        data = [
            (emu_8085.Data.byte(0x03), 0, emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x05), 0x08, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x03), 1, emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x05), 0x09, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0xFE), 1, emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (emu_8085.Data.byte(0x7F), 1, emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x00), 0x80, 0, 0, 1, 0, 1),
        ]

        for val_a, init_carry, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem), carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.extend([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.ADC_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_sub_register(self):
        data = [
            (emu_8085.Data.byte(0x08), emu_8085.Opcode.MVI_B, emu_8085.Opcode.SUB_B, emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Opcode.MVI_C, emu_8085.Opcode.SUB_C, emu_8085.Data.byte(0x05), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Opcode.MVI_D, emu_8085.Opcode.SUB_D, emu_8085.Data.byte(0x01), 0xFF, 0, 1, 1, 1, 1),
            (emu_8085.Data.byte(0x10), emu_8085.Opcode.MVI_E, emu_8085.Opcode.SUB_E, emu_8085.Data.byte(0x01), 0x0F, 0, 0, 0, 1, 1),
            (emu_8085.Data.byte(0x50), emu_8085.Opcode.MVI_H, emu_8085.Opcode.SUB_H, emu_8085.Data.byte(0x20), 0x30, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x07), emu_8085.Opcode.MVI_L, emu_8085.Opcode.SUB_L, emu_8085.Data.byte(0x02), 0x05, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x10), emu_8085.Opcode.MVI_A, emu_8085.Opcode.SUB_A, emu_8085.Data.byte(0x10), 0x00, 1, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, sub_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=sub_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a)]
                if sub_op != emu_8085.Opcode.SUB_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(sub_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_sub_memory(self):
        data = [
            (emu_8085.Data.byte(0x08), emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x05), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x01), 0xFF, 0, 1, 1, 1, 1),
            (emu_8085.Data.byte(0x10), emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x01), 0x0F, 0, 0, 0, 1, 1),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.SUB_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_sub_register_with_borrow(self):
        data = [
            (emu_8085.Data.byte(0x08), 0, emu_8085.Opcode.MVI_B, emu_8085.Opcode.SBB_B, emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x08), 1, emu_8085.Opcode.MVI_C, emu_8085.Opcode.SBB_C, emu_8085.Data.byte(0x03), 0x04, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x00), 1, emu_8085.Opcode.MVI_D, emu_8085.Opcode.SBB_D, emu_8085.Data.byte(0x00), 0xFF, 0, 1, 1, 1, 1),
            (emu_8085.Data.byte(0x10), 1, emu_8085.Opcode.MVI_E, emu_8085.Opcode.SBB_E, emu_8085.Data.byte(0x00), 0x0F, 0, 0, 0, 1, 1),
            (emu_8085.Data.byte(0x50), 1, emu_8085.Opcode.MVI_H, emu_8085.Opcode.SBB_H, emu_8085.Data.byte(0x20), 0x2F, 0, 0, 0, 0, 1),
            (emu_8085.Data.byte(0x07), 1, emu_8085.Opcode.MVI_L, emu_8085.Opcode.SBB_L, emu_8085.Data.byte(0x02), 0x04, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x10), 1, emu_8085.Opcode.MVI_A, emu_8085.Opcode.SBB_A, emu_8085.Data.byte(0x10), 0xFF, 0, 1, 1, 1, 1),
        ]

        for val_a, init_carry, mvi_src, sbb_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=sbb_op.name, carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.append(emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a))
                if sbb_op != emu_8085.Opcode.SBB_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(sbb_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_sub_memory_with_borrow(self):
        data = [
            (emu_8085.Data.byte(0x08), 0, emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x08), 1, emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x03), 0x04, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x00), 1, emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x00), 0xFF, 0, 1, 1, 1, 1),
            (emu_8085.Data.byte(0x10), 1, emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x00), 0x0F, 0, 0, 0, 1, 1),
        ]

        for val_a, init_carry, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem), carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.extend([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.SBB_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_inr_register(self):
        data = [
            (emu_8085.Opcode.MVI_A, emu_8085.Opcode.INR_A, emu_8085.Data.byte(0x05), 0x06, 0, 0, 1, 0),
            (emu_8085.Opcode.MVI_B, emu_8085.Opcode.INR_B, emu_8085.Data.byte(0xFF), 0x00, 1, 0, 1, 1),
            (emu_8085.Opcode.MVI_C, emu_8085.Opcode.INR_C, emu_8085.Data.byte(0x7F), 0x80, 0, 1, 0, 1),
            (emu_8085.Opcode.MVI_D, emu_8085.Opcode.INR_D, emu_8085.Data.byte(0x0F), 0x10, 0, 0, 0, 1),
            (emu_8085.Opcode.MVI_E, emu_8085.Opcode.INR_E, emu_8085.Data.byte(0x01), 0x02, 0, 0, 0, 0),
            (emu_8085.Opcode.MVI_H, emu_8085.Opcode.INR_H, emu_8085.Data.byte(0xFE), 0xFF, 0, 1, 1, 0),
            (emu_8085.Opcode.MVI_L, emu_8085.Opcode.INR_L, emu_8085.Data.byte(0x00), 0x01, 0, 0, 0, 0),
        ]

        for mvi_op, inr_op, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=inr_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(mvi_op, init_val),
                    emu_8085.Instruction(inr_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                reg_name = inr_op.name.split("_")[1].lower()
                target_reg = getattr(cpu, f"reg_{reg_name}")
                self.assertEqual(target_reg.value, exp_val)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_inr_memory(self):
        data = [
            (emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x05), 0x06, 0, 0, 1, 0),
            (emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0xFF), 0x00, 1, 0, 1, 1),
            (emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x7F), 0x80, 0, 1, 0, 1),
            (emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x0F), 0x10, 0, 0, 0, 1),
        ]

        for ptr, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, init_val)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.INR_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(ram.read(mem).value, exp_val)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_dcr_register(self):
        data = [
            (emu_8085.Opcode.MVI_A, emu_8085.Opcode.DCR_A, emu_8085.Data.byte(0x06), 0x05, 0, 0, 1, 0),
            (emu_8085.Opcode.MVI_B, emu_8085.Opcode.DCR_B, emu_8085.Data.byte(0x01), 0x00, 1, 0, 1, 0),
            (emu_8085.Opcode.MVI_C, emu_8085.Opcode.DCR_C, emu_8085.Data.byte(0x00), 0xFF, 0, 1, 1, 1),
            (emu_8085.Opcode.MVI_D, emu_8085.Opcode.DCR_D, emu_8085.Data.byte(0x10), 0x0F, 0, 0, 1, 1),
            (emu_8085.Opcode.MVI_E, emu_8085.Opcode.DCR_E, emu_8085.Data.byte(0x80), 0x7F, 0, 0, 0, 1),
            (emu_8085.Opcode.MVI_H, emu_8085.Opcode.DCR_H, emu_8085.Data.byte(0x02), 0x01, 0, 0, 0, 0),
            (emu_8085.Opcode.MVI_L, emu_8085.Opcode.DCR_L, emu_8085.Data.byte(0x03), 0x02, 0, 0, 0, 0),
        ]

        for mvi_op, dcr_op, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=dcr_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(mvi_op, init_val),
                    emu_8085.Instruction(dcr_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                reg_name = dcr_op.name.split("_")[1].lower()
                target_reg = getattr(cpu, f"reg_{reg_name}")
                self.assertEqual(target_reg.value, exp_val)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_dcr_memory(self):
        data = [
            (emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x06), 0x05, 0, 0, 1, 0),
            (emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x01), 0x00, 1, 0, 1, 0),
            (emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x00), 0xFF, 0, 1, 1, 1),
            (emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x10), 0x0F, 0, 0, 1, 1),
        ]

        for ptr, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, init_val)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.DCR_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(ram.read(mem).value, exp_val)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)

    def test_ana_register(self):
            data = [
                (emu_8085.Data.byte(0xFF), emu_8085.Opcode.MVI_B, emu_8085.Opcode.ANA_B, emu_8085.Data.byte(0x0F), 0x0F, 0, 0, 0, 1, 1),
                (emu_8085.Data.byte(0xAA), emu_8085.Opcode.MVI_C, emu_8085.Opcode.ANA_C, emu_8085.Data.byte(0x55), 0x00, 1, 0, 0, 1, 1),
                (emu_8085.Data.byte(0x80), emu_8085.Opcode.MVI_D, emu_8085.Opcode.ANA_D, emu_8085.Data.byte(0x80), 0x80, 0, 0, 1, 0, 1),
                (emu_8085.Data.byte(0xF0), emu_8085.Opcode.MVI_E, emu_8085.Opcode.ANA_E, emu_8085.Data.byte(0x0F), 0x00, 1, 0, 0, 1, 1),
                (emu_8085.Data.byte(0x33), emu_8085.Opcode.MVI_H, emu_8085.Opcode.ANA_H, emu_8085.Data.byte(0xCC), 0x00, 1, 0, 0, 1, 1),
                (emu_8085.Data.byte(0x0F), emu_8085.Opcode.MVI_L, emu_8085.Opcode.ANA_L, emu_8085.Data.byte(0x07), 0x07, 0, 0, 0, 0, 1),
                (emu_8085.Data.byte(0x12), emu_8085.Opcode.MVI_A, emu_8085.Opcode.ANA_A, emu_8085.Data.byte(0x12), 0x12, 0, 0, 0, 1, 1),
            ]

            for val_a, mvi_src, ana_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
                with self.subTest(op=ana_op.name):
                    machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                    cpu = machine.cpu

                    instructions = [emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a)]
                    if ana_op != emu_8085.Opcode.ANA_A:
                        instructions.append(emu_8085.Instruction(mvi_src, val_src))
                    instructions.extend([
                        emu_8085.Instruction(ana_op),
                        emu_8085.Instruction(emu_8085.Opcode.HLT),
                    ])

                    program = emu_8085.Program(instructions)
                    machine.load(program, self.mem_exec_entry)
                    machine.run()

                    self.assertEqual(cpu.reg_a.value, exp_a)

                    flags = cpu.flag_reg
                    self.assertEqual(flags.zero, exp_z)
                    self.assertEqual(flags.carry, exp_c)
                    self.assertEqual(flags.sign, exp_s)
                    self.assertEqual(flags.parity, exp_p)
                    self.assertEqual(flags.aux, exp_ac)


    def test_ana_memory(self):
        data = [
            (emu_8085.Data.byte(0xFF), emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x0F), 0x0F, 0, 0, 0, 1, 1),
            (emu_8085.Data.byte(0xAA), emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x55), 0x00, 1, 0, 0, 1, 1),
            (emu_8085.Data.byte(0x80), emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x80), 0x80, 0, 0, 1, 0, 1),
            (emu_8085.Data.byte(0xF0), emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x0F), 0x00, 1, 0, 0, 1, 1),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.ANA_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_ani_immediate(self):
        data = [
            (emu_8085.Data.byte(0xFF), emu_8085.Data.byte(0x0F), 0x0F, 0, 0, 0, 1, 1),
            (emu_8085.Data.byte(0xAA), emu_8085.Data.byte(0x55), 0x00, 1, 0, 0, 1, 1),
            (emu_8085.Data.byte(0x80), emu_8085.Data.byte(0x80), 0x80, 0, 0, 1, 0, 1),
            (emu_8085.Data.byte(0xF0), emu_8085.Data.byte(0x0F), 0x00, 1, 0, 0, 1, 1),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.ANI, val_imm),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_ora_register(self):
        data = [
            (emu_8085.Data.byte(0xF0), emu_8085.Opcode.MVI_B, emu_8085.Opcode.ORA_B, emu_8085.Data.byte(0x0F), 0xFF, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Opcode.MVI_C, emu_8085.Opcode.ORA_C, emu_8085.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x80), emu_8085.Opcode.MVI_D, emu_8085.Opcode.ORA_D, emu_8085.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x01), emu_8085.Opcode.MVI_E, emu_8085.Opcode.ORA_E, emu_8085.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x55), emu_8085.Opcode.MVI_H, emu_8085.Opcode.ORA_H, emu_8085.Data.byte(0xAA), 0xFF, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x10), emu_8085.Opcode.MVI_L, emu_8085.Opcode.ORA_L, emu_8085.Data.byte(0x20), 0x30, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x12), emu_8085.Opcode.MVI_A, emu_8085.Opcode.ORA_A, emu_8085.Data.byte(0x12), 0x12, 0, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, ora_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=ora_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a)]
                if ora_op != emu_8085.Opcode.ORA_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(ora_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_ora_memory(self):
        data = [
            (emu_8085.Data.byte(0xF0), emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x0F), 0xFF, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x80), emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x01), emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.ORA_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_ori_immediate(self):
        data = [
            (emu_8085.Data.byte(0xF0), emu_8085.Data.byte(0x0F), 0xFF, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x80), emu_8085.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x01), emu_8085.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.ORI, val_imm),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_xra_register(self):
        data = [
            (emu_8085.Data.byte(0xFF), emu_8085.Opcode.MVI_B, emu_8085.Opcode.XRA_B, emu_8085.Data.byte(0x0F), 0xF0, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0xAA), emu_8085.Opcode.MVI_C, emu_8085.Opcode.XRA_C, emu_8085.Data.byte(0xAA), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x80), emu_8085.Opcode.MVI_D, emu_8085.Opcode.XRA_D, emu_8085.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x03), emu_8085.Opcode.MVI_E, emu_8085.Opcode.XRA_E, emu_8085.Data.byte(0x01), 0x02, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x55), emu_8085.Opcode.MVI_H, emu_8085.Opcode.XRA_H, emu_8085.Data.byte(0x55), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x10), emu_8085.Opcode.MVI_L, emu_8085.Opcode.XRA_L, emu_8085.Data.byte(0x20), 0x30, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x12), emu_8085.Opcode.MVI_A, emu_8085.Opcode.XRA_A, emu_8085.Data.byte(0x12), 0x00, 1, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, xra_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=xra_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a)]
                if xra_op != emu_8085.Opcode.XRA_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(xra_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_xra_memory(self):
        data = [
            (emu_8085.Data.byte(0xFF), emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x0F), 0xF0, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0xAA), emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0xAA), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x80), emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x03), emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x01), 0x02, 0, 0, 0, 0, 0),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.XRA_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_xri_immediate(self):
        data = [
            (emu_8085.Data.byte(0xFF), emu_8085.Data.byte(0x0F), 0xF0, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0xAA), emu_8085.Data.byte(0xAA), 0x00, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x80), emu_8085.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (emu_8085.Data.byte(0x03), emu_8085.Data.byte(0x01), 0x02, 0, 0, 0, 0, 0),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.XRI, val_imm),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)

    def test_cmp_register(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Opcode.MVI_B, emu_8085.Opcode.CMP_B, emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Opcode.MVI_C, emu_8085.Opcode.CMP_C, emu_8085.Data.byte(0x05), 0x05, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x03), emu_8085.Opcode.MVI_D, emu_8085.Opcode.CMP_D, emu_8085.Data.byte(0x05), 0x03, 0, 1, 1, 0, 1),
            (emu_8085.Data.byte(0x00), emu_8085.Opcode.MVI_E, emu_8085.Opcode.CMP_E, emu_8085.Data.byte(0x01), 0x00, 0, 1, 1, 1, 1),
            (emu_8085.Data.byte(0x10), emu_8085.Opcode.MVI_H, emu_8085.Opcode.CMP_H, emu_8085.Data.byte(0x01), 0x10, 0, 0, 0, 1, 1),
            (emu_8085.Data.byte(0x50), emu_8085.Opcode.MVI_L, emu_8085.Opcode.CMP_L, emu_8085.Data.byte(0x20), 0x50, 0, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x12), emu_8085.Opcode.MVI_A, emu_8085.Opcode.CMP_A, emu_8085.Data.byte(0x12), 0x12, 1, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, cmp_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=cmp_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a)]
                if cmp_op != emu_8085.Opcode.CMP_A:
                    instructions.append(emu_8085.Instruction(mvi_src, val_src))
                instructions.extend([
                    emu_8085.Instruction(cmp_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_cmp_memory(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x05), 0x05, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x03), emu_8085.Data.words(0x03, 0x00), emu_8085.Data.byte(0x05), 0x03, 0, 1, 1, 0, 1),
            (emu_8085.Data.byte(0x00), emu_8085.Data.words(0x04, 0x00), emu_8085.Data.byte(0x01), 0x00, 0, 1, 1, 1, 1),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.CMP_M),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_cpi_immediate(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 0x05, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x05), 0x05, 1, 0, 0, 1, 0),
            (emu_8085.Data.byte(0x03), emu_8085.Data.byte(0x05), 0x03, 0, 1, 1, 0, 1),
            (emu_8085.Data.byte(0x00), emu_8085.Data.byte(0x01), 0x00, 0, 1, 1, 1, 1),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.CPI, val_imm),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_rotate_left_circular(self):
        data = [
            (emu_8085.Data.byte(0x82), 0x05, 1),
            (emu_8085.Data.byte(0x01), 0x02, 0),
            (emu_8085.Data.byte(0x80), 0x01, 1),
            (emu_8085.Data.byte(0x7F), 0xFE, 0),
        ]

        for val_a, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.RLC),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_rotate_right_circular(self):
        data = [
            (emu_8085.Data.byte(0x81), 0xC0, 1),
            (emu_8085.Data.byte(0x02), 0x01, 0),
            (emu_8085.Data.byte(0x01), 0x80, 1),
            (emu_8085.Data.byte(0xFE), 0x7F, 0),
        ]

        for val_a, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.RRC),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_rotate_left_through_carry(self):
        data = [
            (emu_8085.Data.byte(0x80), 0, 0x00, 1),
            (emu_8085.Data.byte(0x01), 1, 0x03, 0),
            (emu_8085.Data.byte(0x82), 0, 0x04, 1),
            (emu_8085.Data.byte(0x7F), 1, 0xFF, 0),
        ]

        for val_a, init_carry, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value), carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.extend([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.RAL),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_rotate_right_through_carry(self):
        data = [
            (emu_8085.Data.byte(0x01), 0, 0x00, 1),
            (emu_8085.Data.byte(0x80), 1, 0xC0, 0),
            (emu_8085.Data.byte(0x81), 0, 0x40, 1),
            (emu_8085.Data.byte(0xFE), 1, 0xFF, 0),
        ]

        for val_a, init_carry, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value), carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.extend([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.RAR),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_cma(self):
        data = [
            (emu_8085.Data.byte(0x55), 0xAA),
            (emu_8085.Data.byte(0x00), 0xFF),
            (emu_8085.Data.byte(0xFF), 0x00),
            (emu_8085.Data.byte(0x80), 0x7F),
        ]

        for val_a, exp_a in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.CMA),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)


    def test_cmc(self):
        data = [
            (0, 1),
            (1, 0),
        ]

        for init_carry, exp_c in data:
            with self.subTest(carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.extend([
                    emu_8085.Instruction(emu_8085.Opcode.CMC),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_stc(self):
        data = [0, 1]

        for init_carry in data:
            with self.subTest(carry=init_carry):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
                        emu_8085.Instruction(emu_8085.Opcode.MVI_B, emu_8085.Data.byte(0x01)),
                        emu_8085.Instruction(emu_8085.Opcode.ADD_B),
                    ])

                instructions.extend([
                    emu_8085.Instruction(emu_8085.Opcode.STC),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                program = emu_8085.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.flag_reg.carry, 1)


    def test_decimal_adjust_accumulator(self):
        data = [
            (emu_8085.Data.byte(0x7D), 0, 0, 0x83, 0, 0, 1, 0, 1),
            (emu_8085.Data.byte(0x9B), 0, 0, 0x01, 0, 1, 0, 0, 1),
            (emu_8085.Data.byte(0x12), 0, 0, 0x12, 0, 0, 0, 1, 0),
        ]

        for val_a, _, _, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.DAA),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_decimal_adjust_subtraction(self):
        data = [
            (emu_8085.Data.byte(0xEE), 1, 1, 0x88, 0, 1, 1, 1, 1),
            (emu_8085.Data.byte(0x12), 0, 0, 0x12, 0, 0, 0, 1, 0),
        ]

        for val_a, _, _, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.DAS),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_ascii_adjust_addition(self):
        data = [
            (emu_8085.Data.byte(0x0E), 0x04, 0, 1, 0, 0, 1),
            (emu_8085.Data.byte(0x05), 0x05, 0, 0, 0, 1, 0),
        ]

        for val_a, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.AAA),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_ascii_adjust_subtraction(self):
        data = [
            (emu_8085.Data.byte(0xFD), 0x07, 0, 1, 0, 0, 1),
            (emu_8085.Data.byte(0x05), 0x05, 0, 0, 0, 1, 0),
        ]

        for val_a, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val_a),
                    emu_8085.Instruction(emu_8085.Opcode.AAS),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)

                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_move_from_memory_to_register(self):
        data = [
            (emu_8085.Opcode.MOV_A_M, emu_8085.Data.byte(0x01), emu_8085.Data.words(0x01, 0x00)),
            (emu_8085.Opcode.MOV_B_M, emu_8085.Data.byte(0x02), emu_8085.Data.words(0x02, 0x00)),
            (emu_8085.Opcode.MOV_C_M, emu_8085.Data.byte(0x03), emu_8085.Data.words(0x03, 0x00)),
            (emu_8085.Opcode.MOV_D_M, emu_8085.Data.byte(0x04), emu_8085.Data.words(0x04, 0x00)),
            (emu_8085.Opcode.MOV_E_M, emu_8085.Data.byte(0x05), emu_8085.Data.words(0x05, 0x00)),
            (emu_8085.Opcode.MOV_H_M, emu_8085.Data.byte(0x06), emu_8085.Data.words(0x06, 0x00)),
            (emu_8085.Opcode.MOV_L_M, emu_8085.Data.byte(0x07), emu_8085.Data.words(0x07, 0x00)),
        ]

        for mov_op, val, ptr in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(op=mov_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.LXI, ptr),
                    emu_8085.Instruction(mov_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                reg_name = mov_op.name.split("_")[1].lower()
                reg_obj = getattr(cpu, f"reg_{reg_name}")
                self.assertEqual(reg_obj.value, val.value)


    def test_lda_indirect(self):
        data = [
            (emu_8085.Opcode.MVI_BC, emu_8085.Opcode.LDA_BC, emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x42)),
            (emu_8085.Opcode.MVI_DE, emu_8085.Opcode.LDA_DE, emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0x84)),
        ]

        for mvi_rp, lda_rp, ptr, val in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(op=lda_rp.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val)

                program = emu_8085.Program([
                    emu_8085.Instruction(mvi_rp, ptr),
                    emu_8085.Instruction(lda_rp),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, val.value)


    def test_sta_indirect(self):
        data = [
            (emu_8085.Opcode.MVI_BC, emu_8085.Opcode.STA_BC, emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x55)),
            (emu_8085.Opcode.MVI_DE, emu_8085.Opcode.STA_DE, emu_8085.Data.words(0x02, 0x00), emu_8085.Data.byte(0xAA)),
        ]

        for mvi_rp, sta_rp, ptr, val in data:
            mem = emu_8085.Mem(ptr.reverse().value)

            with self.subTest(op=sta_rp.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, val),
                    emu_8085.Instruction(mvi_rp, ptr),
                    emu_8085.Instruction(sta_rp),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(ram.read(mem).value, val.value)


    def test_lhld(self):
        data = [
            (emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x45), emu_8085.Data.byte(0x12)),
            (emu_8085.Data.words(0x05, 0x00), emu_8085.Data.byte(0xEF), emu_8085.Data.byte(0xCD)),
        ]

        for ptr, val_low, val_high in data:
            addr_low = emu_8085.Mem(ptr.reverse().value)
            addr_high = emu_8085.Mem(addr_low + 1)

            with self.subTest(addr=hex(addr_low)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(addr_low, val_low)
                ram.write(addr_high, val_high)

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.LHLD, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_l.value, val_low.value)
                self.assertEqual(cpu.reg_h.value, val_high.value)


    def test_shld(self):
        data = [
            (emu_8085.Data.words(0x01, 0x00), emu_8085.Data.byte(0x45), emu_8085.Data.byte(0x12)),
            (emu_8085.Data.words(0x05, 0x00), emu_8085.Data.byte(0xEF), emu_8085.Data.byte(0xCD)),
        ]

        for ptr, val_low, val_high in data:
            addr_low = emu_8085.Mem(ptr.reverse().value)
            addr_high = emu_8085.Mem(addr_low + 1)

            with self.subTest(addr=hex(addr_low)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_L, val_low),
                    emu_8085.Instruction(emu_8085.Opcode.MVI_H, val_high),
                    emu_8085.Instruction(emu_8085.Opcode.SHLD, ptr),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(ram.read(addr_low).value, val_low.value)
                self.assertEqual(ram.read(addr_high).value, val_high.value)


    def test_xchg(self):
        data = [
            (emu_8085.Data.byte(0x12), emu_8085.Data.byte(0x34), emu_8085.Data.byte(0x56), emu_8085.Data.byte(0x78)),
            (emu_8085.Data.byte(0xAA), emu_8085.Data.byte(0xBB), emu_8085.Data.byte(0xCC), emu_8085.Data.byte(0xDD)),
        ]

        for val_d, val_e, val_h, val_l in data:
            with self.subTest(d=hex(val_d.value), h=hex(val_h.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_D, val_d),
                    emu_8085.Instruction(emu_8085.Opcode.MVI_E, val_e),
                    emu_8085.Instruction(emu_8085.Opcode.MVI_H, val_h),
                    emu_8085.Instruction(emu_8085.Opcode.MVI_L, val_l),
                    emu_8085.Instruction(emu_8085.Opcode.XCHG),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_d.value, val_h.value)
                self.assertEqual(cpu.reg_e.value, val_l.value)
                self.assertEqual(cpu.reg_h.value, val_d.value)
                self.assertEqual(cpu.reg_l.value, val_e.value)


    def test_adi(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 0x08, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0xFF), emu_8085.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
        ]

        for init_a, imm_val, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, init_a),
                    emu_8085.Instruction(emu_8085.Opcode.ADI, imm_val),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_aci(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 0, 0x08, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 1, 0x09, 0, 0, 0, 1, 0),
        ]

        for init_a, imm_val, init_c, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value), cin=init_c):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu
                cpu.flag_reg.carry = init_c

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, init_a),
                    emu_8085.Instruction(emu_8085.Opcode.ACI, imm_val),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_sui(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 0x02, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x00), emu_8085.Data.byte(0x01), 0xFF, 0, 1, 1, 1, 1),
        ]

        for init_a, imm_val, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value)):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, init_a),
                    emu_8085.Instruction(emu_8085.Opcode.SUI, imm_val),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_sbi(self):
        data = [
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 0, 0x02, 0, 0, 0, 0, 0),
            (emu_8085.Data.byte(0x05), emu_8085.Data.byte(0x03), 1, 0x01, 0, 0, 0, 0, 0),
        ]

        for init_a, imm_val, init_c, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value), cin=init_c):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu
                cpu.flag_reg.carry = init_c

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_A, init_a),
                    emu_8085.Instruction(emu_8085.Opcode.SBI, imm_val),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                flags = cpu.flag_reg
                self.assertEqual(flags.zero, exp_z)
                self.assertEqual(flags.carry, exp_c)
                self.assertEqual(flags.sign, exp_s)
                self.assertEqual(flags.parity, exp_p)
                self.assertEqual(flags.aux, exp_ac)


    def test_inx(self):
        data = [
            (emu_8085.Opcode.MVI_BC, emu_8085.Opcode.INX_BC, "pair_bc", emu_8085.Data.words(0xFF, 0x00), 0x0100),
            (emu_8085.Opcode.MVI_DE, emu_8085.Opcode.INX_DE, "pair_de", emu_8085.Data.words(0xFF, 0x00), 0x0100),
            (emu_8085.Opcode.MVI_HL, emu_8085.Opcode.INX_HL, "pair_hl", emu_8085.Data.words(0xFF, 0x00), 0x0100),
        ]

        for mvi_op, inx_op, pair_attr, init_val, exp_val in data:
            with self.subTest(op=inx_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(mvi_op, init_val),
                    emu_8085.Instruction(inx_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(getattr(cpu, pair_attr).value, exp_val)


    def test_dcx(self):
        data = [
            (emu_8085.Opcode.MVI_BC, emu_8085.Opcode.DCX_BC, "pair_bc", emu_8085.Data.words(0x00, 0x01), 0x00FF),
            (emu_8085.Opcode.MVI_DE, emu_8085.Opcode.DCX_DE, "pair_de", emu_8085.Data.words(0x00, 0x01), 0x00FF),
            (emu_8085.Opcode.MVI_HL, emu_8085.Opcode.DCX_HL, "pair_hl", emu_8085.Data.words(0x00, 0x01), 0x00FF),
        ]

        for mvi_op, dcx_op, pair_attr, init_val, exp_val in data:
            with self.subTest(op=dcx_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(mvi_op, init_val),
                    emu_8085.Instruction(dcx_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(getattr(cpu, pair_attr).value, exp_val)


    def test_dad(self):
        data = [
            (emu_8085.Opcode.MVI_BC, emu_8085.Opcode.DAD_BC, emu_8085.Data.words(0x00, 0x10), emu_8085.Data.words(0x00, 0x20), 0x3000, 0),
            (emu_8085.Opcode.MVI_DE, emu_8085.Opcode.DAD_DE, emu_8085.Data.words(0x00, 0x10), emu_8085.Data.words(0x00, 0x20), 0x3000, 0),
            (emu_8085.Opcode.MVI_HL, emu_8085.Opcode.DAD_HL, emu_8085.Data.words(0x00, 0x80), emu_8085.Data.words(0x00, 0x80), 0x0000, 1),
        ]

        for mvi_rp, dad_op, init_hl, init_rp, exp_hl, exp_c in data:
            with self.subTest(op=dad_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = emu_8085.Program([
                    emu_8085.Instruction(emu_8085.Opcode.MVI_HL, init_hl),
                    *( [emu_8085.Instruction(mvi_rp, init_rp)] if dad_op != emu_8085.Opcode.DAD_HL else [] ),
                    emu_8085.Instruction(dad_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.pair_hl.value, exp_hl)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_inx_sp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.INX_SP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_sp.value, 0x0000)


    def test_dcx_sp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        cpu.reg_sp.write(0x0000)

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.DCX_SP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_sp.value, 0xFFFF)


    def test_dad_sp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_HL, emu_8085.Data.words(0x01, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.DAD_SP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.pair_hl.value, 0x0000)
        self.assertEqual(cpu.flag_reg.carry, 1)


    def test_push_pop(self):
        data = [
            (emu_8085.Opcode.MVI_BC, emu_8085.Opcode.PUSH_BC, emu_8085.Opcode.POP_BC, "pair_bc", emu_8085.Data.words(0x12, 0x34), 0x3412),
            (emu_8085.Opcode.MVI_DE, emu_8085.Opcode.PUSH_DE, emu_8085.Opcode.POP_DE, "pair_de", emu_8085.Data.words(0x56, 0x78), 0x7856),
            (emu_8085.Opcode.MVI_HL, emu_8085.Opcode.PUSH_HL, emu_8085.Opcode.POP_HL, "pair_hl", emu_8085.Data.words(0x90, 0xAB), 0xAB90),
        ]

        for mvi_op, push_op, pop_op, pair_attr, init_val, exp_val in data:
            with self.subTest(op=push_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                cpu.reg_sp.write(0x1000)

                program = emu_8085.Program([
                    emu_8085.Instruction(mvi_op, init_val),
                    emu_8085.Instruction(push_op),
                    emu_8085.Instruction(pop_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(getattr(cpu, pair_attr).value, exp_val)
                self.assertEqual(cpu.reg_sp.value, 0x1000)


    def test_push_pop_psw(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        cpu.reg_sp.write(0x1000)

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xAB)),
            emu_8085.Instruction(emu_8085.Opcode.ADI, emu_8085.Data.byte(0x01)),
            emu_8085.Instruction(emu_8085.Opcode.PUSH_PSW),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x00)),
            emu_8085.Instruction(emu_8085.Opcode.POP_PSW),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0xAC)
        self.assertEqual(cpu.reg_sp.value, 0x1000)


    def test_sphl(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_HL, emu_8085.Data.words(0x00, 0x20)),
            emu_8085.Instruction(emu_8085.Opcode.SPHL),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_sp.value, 0x2000)


    def test_xthl(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        cpu.reg_sp.write(0x1000)
        ram.write(emu_8085.Mem(0x1000), emu_8085.Data.byte(0x78))
        ram.write(emu_8085.Mem(0x1001), emu_8085.Data.byte(0x56))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_HL, emu_8085.Data.words(0x34, 0x12)),
            emu_8085.Instruction(emu_8085.Opcode.XTHL),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_l.value, 0x78)
        self.assertEqual(cpu.reg_h.value, 0x56)
        self.assertEqual(ram.read(emu_8085.Mem(0x1000)).value, 0x34)
        self.assertEqual(ram.read(emu_8085.Mem(0x1001)).value, 0x12)
        self.assertEqual(cpu.reg_sp.value, 0x1000)


    def test_jmp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0050)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x42))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JMP, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xFF)),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x42)

    def test_call_and_ret(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_pchl(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0060)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_HL, emu_8085.Data.words(0x60, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.PCHL),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x99)

    def test_jz(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.zero = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JZ, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.zero = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JZ, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jnz(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.zero = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JNZ, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.zero = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JNZ, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jc(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.carry = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JC, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.carry = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JC, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jnc(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.carry = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JNC, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.carry = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JNC, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.sign = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JP, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.sign = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JP, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jm(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.sign = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JM, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.sign = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JM, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jpe(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.parity = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JPE, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.parity = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JPE, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jpo(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = emu_8085.Mem(0x0070)
        ram.write(target_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(target_addr + 1), emu_8085.Data.byte(0x77))
        ram.write(emu_8085.Mem(target_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.HLT))

        cpu.flag_reg.parity = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JPO, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.parity = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.JPO, emu_8085.Data.words(0x70, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_cz(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.zero = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CZ, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CZ, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cnz(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.zero = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CNZ, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CNZ, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cc(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.carry = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CC, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CC, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cnc(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.carry = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CNC, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CNC, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.sign = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CP, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CP, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cm(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.sign = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CM, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CM, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cpe(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.parity = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CPE, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CPE, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cpo(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.parity = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CPO, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CPO, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x22)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rz(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RZ))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.zero = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rnz(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RNZ))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.zero = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rc(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RC))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.carry = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rnc(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RNC))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.carry = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rp(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RP))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.sign = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rm(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RM))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.sign = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rpe(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RPE))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.parity = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rpo(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = emu_8085.Mem(0x0050)
        ram.write(subroutine_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 1), emu_8085.Data.byte(0x55))
        ram.write(emu_8085.Mem(subroutine_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RPO))
        ram.write(emu_8085.Mem(subroutine_addr + 3), emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
        ram.write(emu_8085.Mem(subroutine_addr + 4), emu_8085.Data.byte(0x99))
        ram.write(emu_8085.Mem(subroutine_addr + 5), emu_8085.Data.byte(emu_8085.Opcode.RET))

        cpu.flag_reg.parity = 0
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 1
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, emu_8085.Data.words(0x50, 0x00)),
            emu_8085.Instruction(emu_8085.Opcode.INR_A),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_nop(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.NOP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_pc.value, self.mem_exec_entry + 2)

    def test_device_manager_tick(self):
        class MockDevice(emu_8085.Device):
            def __init__(self):
                self.ticks = 0

            @property
            def name(self) -> str:
                return "MockDevice"

            def tick(self, bus: emu_8085.SystemBus) -> None:
                self.ticks += 1

        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        mock_dev = MockDevice()
        machine.device_manager.attach_device(mock_dev)

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.NOP),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertTrue(mock_dev.ticks > 0)

    def test_rst_instructions(self):
        rst_data = [
            (emu_8085.Opcode.RST_0, 0x0000),
            (emu_8085.Opcode.RST_1, 0x0008),
            (emu_8085.Opcode.RST_2, 0x0010),
            (emu_8085.Opcode.RST_3, 0x0018),
            (emu_8085.Opcode.RST_4, 0x0020),
            (emu_8085.Opcode.RST_5, 0x0028),
            (emu_8085.Opcode.RST_6, 0x0030),
            (emu_8085.Opcode.RST_7, 0x0038),
        ]

        for rst_op, exp_vector in rst_data:
            with self.subTest(op=rst_op.name):
                machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu
                ram = machine.ram

                cpu.reg_sp.write(0x1000)

                # Set up ISR at target vector: MVI A, 0x77; RET
                isr_addr = emu_8085.Mem(exp_vector)
                ram.write(isr_addr, emu_8085.Data.byte(emu_8085.Opcode.MVI_A))
                ram.write(emu_8085.Mem(isr_addr + 1), emu_8085.Data.byte(0x77))
                ram.write(emu_8085.Mem(isr_addr + 2), emu_8085.Data.byte(emu_8085.Opcode.RET))

                program = emu_8085.Program([
                    emu_8085.Instruction(rst_op),
                    emu_8085.Instruction(emu_8085.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, 0x77)
                self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_ei_di(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        self.assertFalse(cpu.inte)

        program_ei = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.EI),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program_ei, self.mem_exec_entry)
        machine.run()
        self.assertTrue(cpu.inte)

        program_di = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.DI),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])
        machine.load(program_di, self.mem_exec_entry)
        machine.run()
        self.assertFalse(cpu.inte)

    def test_in_out_port(self):
        class OutputDevice(emu_8085.Device):
            def __init__(self):
                self.received = []

            @property
            def name(self) -> str:
                return "OutputDevice"

            def port_write(self, port: int, data: int) -> None:
                self.received.append((port, data))

        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        kbd = emu_8085.KeyboardDevice()
        out_dev = OutputDevice()

        machine.device_manager.attach_device(kbd, ports=[0x05])
        machine.device_manager.attach_device(out_dev, ports=[0x0A])

        kbd.trigger_key_press('K')

        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.IN, emu_8085.Data.byte(0x05)),
            emu_8085.Instruction(emu_8085.Opcode.OUT, emu_8085.Data.byte(0x0A)),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(machine.cpu.reg_a.value, ord('K'))
        self.assertEqual(out_dev.received, [(0x0A, ord('K'))])

    def test_rim_sim(self):
        machine = emu_8085.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        # Set Accumulator to SIM configuration: MSE=1, M5.5=1, M7.5=1, SDE=1, SOD=1 -> 0xCD
        # Binary: 1100 1101 = 0xCD (SOD=1, SDE=1, R7.5=0, MSE=1, M7.5=1, M6.5=0, M5.5=1)
        sim_program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0xCD)),
            emu_8085.Instruction(emu_8085.Opcode.SIM),
            emu_8085.Instruction(emu_8085.Opcode.EI),
            emu_8085.Instruction(emu_8085.Opcode.RIM),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        cpu.sid = True
        machine.load(sim_program, self.mem_exec_entry)
        machine.run()

        self.assertTrue(cpu.sod)
        self.assertTrue(cpu.mask_5_5)
        self.assertFalse(cpu.mask_6_5)
        self.assertTrue(cpu.mask_7_5)

        # RIM result in A: SID=1(bit7), IE=1(bit3), M7.5=1(bit2), M5.5=1(bit0) -> 1000 1101 = 0x8D
        self.assertEqual(cpu.reg_a.value, 0x8D)


