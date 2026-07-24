import unittest

import main


# NOTE:
# 1) Never use default test memory address or default test data value '0x0'. Due to default
# initial value.
# 2) Inside instruction provide address as Data instead Mem.
class TestExecution(unittest.TestCase):

    address_lines: int = 16
    data_lines: int = 8
    mem_exec_entry: main.Mem = main.Mem(0x00A0)

    _machine: main.Machine | None


    def setUp(self):
        super().setUp()
        self._machine = main.Machine.create(
            address_lines=self.address_lines,
            data_lines=self.data_lines,
        )

    def tearDown(self) -> None:
        super().tearDown()
        self._machine = None

    @property
    def machine(self) -> main.Machine:
        assert(self._machine is not None)
        return self._machine


    def test_move_immediate_to_register(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MVI_A, main.Data.byte(0x01)),
            (cpu.reg_b, main.Opcode.MVI_B, main.Data.byte(0x02)),
            (cpu.reg_c, main.Opcode.MVI_C, main.Data.byte(0x03)),
            (cpu.reg_d, main.Opcode.MVI_D, main.Data.byte(0x04)),
            (cpu.reg_e, main.Opcode.MVI_E, main.Data.byte(0x05)),
            (cpu.reg_h, main.Opcode.MVI_H, main.Data.byte(0x06)),
            (cpu.reg_l, main.Opcode.MVI_L, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(main.Instruction(opcode, arg) for _, opcode, arg in data),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_immediate_to_register_pair(self):
        cpu = self.machine.cpu
        data = [
            ((cpu.reg_b, cpu.reg_c), main.Opcode.MVI_BC, main.Data.words(0x01, 0x02)),
            ((cpu.reg_d, cpu.reg_e), main.Opcode.MVI_DE, main.Data.words(0x03, 0x04)),
            ((cpu.reg_h, cpu.reg_l), main.Opcode.MVI_HL, main.Data.words(0x05, 0x06)),
        ]
        program = main.Program([
            *(main.Instruction(opcode, arg) for _, opcode, arg in data),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        m1 = main.Mask.bits(8, 0)
        m2 = main.Mask.bits(8, 8)

        for (reg1, reg2), opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg1.value, m1.apply(val).value)
                self.assertEqual(reg2.value, m2.apply(val).value)


    def test_move_to_memory_from_register(self):
        ram = self.machine.ram
        data = [
            (main.Opcode.MVI_A, main.Opcode.MOV_M_A, main.Data.byte(0x01), main.Data.words(0x01, 0x00)),
            (main.Opcode.MVI_B, main.Opcode.MOV_M_B, main.Data.byte(0x02), main.Data.words(0x02, 0x00)),
            (main.Opcode.MVI_C, main.Opcode.MOV_M_C, main.Data.byte(0x03), main.Data.words(0x03, 0x00)),
            (main.Opcode.MVI_D, main.Opcode.MOV_M_D, main.Data.byte(0x04), main.Data.words(0x04, 0x00)),
            (main.Opcode.MVI_E, main.Opcode.MOV_M_E, main.Data.byte(0x05), main.Data.words(0x05, 0x00)),
            (main.Opcode.MVI_H, main.Opcode.MOV_M_H, main.Data.byte(0x06), main.Data.words(0x06, 0x00)),
            (main.Opcode.MVI_L, main.Opcode.MOV_M_L, main.Data.byte(0x07), main.Data.words(0x07, 0x00)),
        ]
        program = main.Program([
            *(
                instruction
                for o1, o2, val, mem in data
                for instruction in [
                    main.Instruction(o1, val),
                    main.Instruction(main.Opcode.LXI, mem),
                    main.Instruction(o2),
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for _, o2, val, ptr in data[:5]:
            with self.subTest(op=o2.name):
                mem = main.Mem(ptr.reverse().value)
                self.assertEqual(val.value, ram.read(mem).value)

        _, o2, val, ptr = data[5]
        with self.subTest(op=o2.name):
            mem = main.Mem(ptr.reverse().value)
            self.assertEqual(0x00, ram.read(mem).value)

        _, o2, val, ptr = data[6]
        with self.subTest(op=o2.name):
            mem = main.Mem(ptr.reverse().value)
            self.assertEqual(0x07, ram.read(mem).value)


    def test_move_to_register_from_register_a(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_b, main.Opcode.MOV_B_A, main.Data.byte(0x02)),
            (cpu.reg_c, main.Opcode.MOV_C_A, main.Data.byte(0x03)),
            (cpu.reg_d, main.Opcode.MOV_D_A, main.Data.byte(0x04)),
            (cpu.reg_e, main.Opcode.MOV_E_A, main.Data.byte(0x05)),
            (cpu.reg_h, main.Opcode.MOV_H_A, main.Data.byte(0x06)),
            (cpu.reg_l, main.Opcode.MOV_L_A, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_A, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_b(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MOV_A_B, main.Data.byte(0x01)),
            (cpu.reg_c, main.Opcode.MOV_C_B, main.Data.byte(0x03)),
            (cpu.reg_d, main.Opcode.MOV_D_B, main.Data.byte(0x04)),
            (cpu.reg_e, main.Opcode.MOV_E_B, main.Data.byte(0x05)),
            (cpu.reg_h, main.Opcode.MOV_H_B, main.Data.byte(0x06)),
            (cpu.reg_l, main.Opcode.MOV_L_B, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_B, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_c(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MOV_A_C, main.Data.byte(0x01)),
            (cpu.reg_b, main.Opcode.MOV_B_C, main.Data.byte(0x02)),
            (cpu.reg_d, main.Opcode.MOV_D_C, main.Data.byte(0x04)),
            (cpu.reg_e, main.Opcode.MOV_E_C, main.Data.byte(0x05)),
            (cpu.reg_h, main.Opcode.MOV_H_C, main.Data.byte(0x06)),
            (cpu.reg_l, main.Opcode.MOV_L_C, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_C, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_d(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MOV_A_D, main.Data.byte(0x01)),
            (cpu.reg_b, main.Opcode.MOV_B_D, main.Data.byte(0x02)),
            (cpu.reg_c, main.Opcode.MOV_C_D, main.Data.byte(0x03)),
            (cpu.reg_e, main.Opcode.MOV_E_D, main.Data.byte(0x05)),
            (cpu.reg_h, main.Opcode.MOV_H_D, main.Data.byte(0x06)),
            (cpu.reg_l, main.Opcode.MOV_L_D, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_D, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_e(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MOV_A_E, main.Data.byte(0x01)),
            (cpu.reg_b, main.Opcode.MOV_B_E, main.Data.byte(0x02)),
            (cpu.reg_c, main.Opcode.MOV_C_E, main.Data.byte(0x03)),
            (cpu.reg_d, main.Opcode.MOV_D_E, main.Data.byte(0x04)),
            (cpu.reg_h, main.Opcode.MOV_H_E, main.Data.byte(0x06)),
            (cpu.reg_l, main.Opcode.MOV_L_E, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_E, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_h(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MOV_A_H, main.Data.byte(0x01)),
            (cpu.reg_b, main.Opcode.MOV_B_H, main.Data.byte(0x02)),
            (cpu.reg_c, main.Opcode.MOV_C_H, main.Data.byte(0x03)),
            (cpu.reg_d, main.Opcode.MOV_D_H, main.Data.byte(0x04)),
            (cpu.reg_e, main.Opcode.MOV_E_H, main.Data.byte(0x05)),
            (cpu.reg_l, main.Opcode.MOV_L_H, main.Data.byte(0x07)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_H, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_move_to_register_from_register_l(self):
        cpu = self.machine.cpu
        data = [
            (cpu.reg_a, main.Opcode.MOV_A_L, main.Data.byte(0x01)),
            (cpu.reg_b, main.Opcode.MOV_B_L, main.Data.byte(0x02)),
            (cpu.reg_c, main.Opcode.MOV_C_L, main.Data.byte(0x03)),
            (cpu.reg_d, main.Opcode.MOV_D_L, main.Data.byte(0x04)),
            (cpu.reg_e, main.Opcode.MOV_E_L, main.Data.byte(0x05)),
            (cpu.reg_h, main.Opcode.MOV_H_L, main.Data.byte(0x06)),
        ]
        program = main.Program([
            *(
                instruction
                for _, opcode, val in data
                for instruction in [
                    main.Instruction(main.Opcode.MVI_L, val),
                    main.Instruction(opcode)
                ]
            ),
            main.Instruction(main.Opcode.HLT),
        ])

        self.machine.load(program, self.mem_exec_entry)
        self.machine.run()

        for reg, opcode, val in data:
            with self.subTest(op=opcode.name):
                self.assertEqual(reg.value, val.value)


    def test_add_register(self):
        cpu = self.machine.cpu
        data = [
            (main.Data.byte(0x05), main.Opcode.MVI_B, main.Opcode.ADD_B, main.Data.byte(0x03), 0x08, 0, 0, 0, 0, 0),
            (main.Data.byte(0x00), main.Opcode.MVI_C, main.Opcode.ADD_C, main.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0xFF), main.Opcode.MVI_D, main.Opcode.ADD_D, main.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (main.Data.byte(0x70), main.Opcode.MVI_E, main.Opcode.ADD_E, main.Data.byte(0x10), 0x80, 0, 0, 1, 0, 0),
            (main.Data.byte(0x0F), main.Opcode.MVI_H, main.Opcode.ADD_H, main.Data.byte(0x01), 0x10, 0, 0, 0, 0, 1),
            (main.Data.byte(0x01), main.Opcode.MVI_L, main.Opcode.ADD_L, main.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
            (main.Data.byte(0x10), main.Opcode.MVI_A, main.Opcode.ADD_A, main.Data.byte(0x10), 0x20, 0, 0, 0, 0, 0),
        ]

        for val_a, mvi_src, add_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=add_op.name):
                instructions = [main.Instruction(main.Opcode.MVI_A, val_a)]
                if add_op != main.Opcode.ADD_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(add_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0x03), main.Data.words(0x01, 0x00), main.Data.byte(0x05), 0x08, 0, 0, 0, 0, 0),
            (main.Data.byte(0x00), main.Data.words(0x02, 0x00), main.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0xFF), main.Data.words(0x03, 0x00), main.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (main.Data.byte(0x70), main.Data.words(0x04, 0x00), main.Data.byte(0x10), 0x80, 0, 0, 1, 0, 0),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                ram.write(mem, val_mem)

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.ADD_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x05), 0, main.Opcode.MVI_B, main.Opcode.ADC_B, main.Data.byte(0x03), 0x08, 0, 0, 0, 0, 0),
            (main.Data.byte(0x05), 1, main.Opcode.MVI_C, main.Opcode.ADC_C, main.Data.byte(0x03), 0x09, 0, 0, 0, 1, 0),
            (main.Data.byte(0xFE), 1, main.Opcode.MVI_D, main.Opcode.ADC_D, main.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (main.Data.byte(0x7F), 1, main.Opcode.MVI_E, main.Opcode.ADC_E, main.Data.byte(0x00), 0x80, 0, 0, 1, 0, 1),
            (main.Data.byte(0x0F), 1, main.Opcode.MVI_H, main.Opcode.ADC_H, main.Data.byte(0x00), 0x10, 0, 0, 0, 0, 1),
            (main.Data.byte(0x01), 1, main.Opcode.MVI_L, main.Opcode.ADC_L, main.Data.byte(0x01), 0x03, 0, 0, 0, 1, 0),
            (main.Data.byte(0x10), 1, main.Opcode.MVI_A, main.Opcode.ADC_A, main.Data.byte(0x10), 0x21, 0, 0, 0, 1, 0),
        ]

        for val_a, init_carry, mvi_src, adc_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=adc_op.name, carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.append(main.Instruction(main.Opcode.MVI_A, val_a))
                if adc_op != main.Opcode.ADC_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(adc_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0x03), 0, main.Data.words(0x01, 0x00), main.Data.byte(0x05), 0x08, 0, 0, 0, 0, 0),
            (main.Data.byte(0x03), 1, main.Data.words(0x02, 0x00), main.Data.byte(0x05), 0x09, 0, 0, 0, 1, 0),
            (main.Data.byte(0xFE), 1, main.Data.words(0x03, 0x00), main.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
            (main.Data.byte(0x7F), 1, main.Data.words(0x04, 0x00), main.Data.byte(0x00), 0x80, 0, 0, 1, 0, 1),
        ]

        for val_a, init_carry, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem), carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.extend([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.ADC_M),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0x08), main.Opcode.MVI_B, main.Opcode.SUB_B, main.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (main.Data.byte(0x05), main.Opcode.MVI_C, main.Opcode.SUB_C, main.Data.byte(0x05), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x00), main.Opcode.MVI_D, main.Opcode.SUB_D, main.Data.byte(0x01), 0xFF, 0, 1, 1, 1, 1),
            (main.Data.byte(0x10), main.Opcode.MVI_E, main.Opcode.SUB_E, main.Data.byte(0x01), 0x0F, 0, 0, 0, 1, 1),
            (main.Data.byte(0x50), main.Opcode.MVI_H, main.Opcode.SUB_H, main.Data.byte(0x20), 0x30, 0, 0, 0, 1, 0),
            (main.Data.byte(0x07), main.Opcode.MVI_L, main.Opcode.SUB_L, main.Data.byte(0x02), 0x05, 0, 0, 0, 1, 0),
            (main.Data.byte(0x10), main.Opcode.MVI_A, main.Opcode.SUB_A, main.Data.byte(0x10), 0x00, 1, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, sub_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=sub_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [main.Instruction(main.Opcode.MVI_A, val_a)]
                if sub_op != main.Opcode.SUB_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(sub_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0x08), main.Data.words(0x01, 0x00), main.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (main.Data.byte(0x05), main.Data.words(0x02, 0x00), main.Data.byte(0x05), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x00), main.Data.words(0x03, 0x00), main.Data.byte(0x01), 0xFF, 0, 1, 1, 1, 1),
            (main.Data.byte(0x10), main.Data.words(0x04, 0x00), main.Data.byte(0x01), 0x0F, 0, 0, 0, 1, 1),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.SUB_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x08), 0, main.Opcode.MVI_B, main.Opcode.SBB_B, main.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (main.Data.byte(0x08), 1, main.Opcode.MVI_C, main.Opcode.SBB_C, main.Data.byte(0x03), 0x04, 0, 0, 0, 0, 0),
            (main.Data.byte(0x00), 1, main.Opcode.MVI_D, main.Opcode.SBB_D, main.Data.byte(0x00), 0xFF, 0, 1, 1, 1, 1),
            (main.Data.byte(0x10), 1, main.Opcode.MVI_E, main.Opcode.SBB_E, main.Data.byte(0x00), 0x0F, 0, 0, 0, 1, 1),
            (main.Data.byte(0x50), 1, main.Opcode.MVI_H, main.Opcode.SBB_H, main.Data.byte(0x20), 0x2F, 0, 0, 0, 0, 1),
            (main.Data.byte(0x07), 1, main.Opcode.MVI_L, main.Opcode.SBB_L, main.Data.byte(0x02), 0x04, 0, 0, 0, 0, 0),
            (main.Data.byte(0x10), 1, main.Opcode.MVI_A, main.Opcode.SBB_A, main.Data.byte(0x10), 0xFF, 0, 1, 1, 1, 1),
        ]

        for val_a, init_carry, mvi_src, sbb_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=sbb_op.name, carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.append(main.Instruction(main.Opcode.MVI_A, val_a))
                if sbb_op != main.Opcode.SBB_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(sbb_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0x08), 0, main.Data.words(0x01, 0x00), main.Data.byte(0x03), 0x05, 0, 0, 0, 1, 0),
            (main.Data.byte(0x08), 1, main.Data.words(0x02, 0x00), main.Data.byte(0x03), 0x04, 0, 0, 0, 0, 0),
            (main.Data.byte(0x00), 1, main.Data.words(0x03, 0x00), main.Data.byte(0x00), 0xFF, 0, 1, 1, 1, 1),
            (main.Data.byte(0x10), 1, main.Data.words(0x04, 0x00), main.Data.byte(0x00), 0x0F, 0, 0, 0, 1, 1),
        ]

        for val_a, init_carry, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem), carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.extend([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.SBB_M),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Opcode.MVI_A, main.Opcode.INR_A, main.Data.byte(0x05), 0x06, 0, 0, 1, 0),
            (main.Opcode.MVI_B, main.Opcode.INR_B, main.Data.byte(0xFF), 0x00, 1, 0, 1, 1),
            (main.Opcode.MVI_C, main.Opcode.INR_C, main.Data.byte(0x7F), 0x80, 0, 1, 0, 1),
            (main.Opcode.MVI_D, main.Opcode.INR_D, main.Data.byte(0x0F), 0x10, 0, 0, 0, 1),
            (main.Opcode.MVI_E, main.Opcode.INR_E, main.Data.byte(0x01), 0x02, 0, 0, 0, 0),
            (main.Opcode.MVI_H, main.Opcode.INR_H, main.Data.byte(0xFE), 0xFF, 0, 1, 1, 0),
            (main.Opcode.MVI_L, main.Opcode.INR_L, main.Data.byte(0x00), 0x01, 0, 0, 0, 0),
        ]

        for mvi_op, inr_op, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=inr_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(mvi_op, init_val),
                    main.Instruction(inr_op),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.words(0x01, 0x00), main.Data.byte(0x05), 0x06, 0, 0, 1, 0),
            (main.Data.words(0x02, 0x00), main.Data.byte(0xFF), 0x00, 1, 0, 1, 1),
            (main.Data.words(0x03, 0x00), main.Data.byte(0x7F), 0x80, 0, 1, 0, 1),
            (main.Data.words(0x04, 0x00), main.Data.byte(0x0F), 0x10, 0, 0, 0, 1),
        ]

        for ptr, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, init_val)

                program = main.Program([
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.INR_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Opcode.MVI_A, main.Opcode.DCR_A, main.Data.byte(0x06), 0x05, 0, 0, 1, 0),
            (main.Opcode.MVI_B, main.Opcode.DCR_B, main.Data.byte(0x01), 0x00, 1, 0, 1, 0),
            (main.Opcode.MVI_C, main.Opcode.DCR_C, main.Data.byte(0x00), 0xFF, 0, 1, 1, 1),
            (main.Opcode.MVI_D, main.Opcode.DCR_D, main.Data.byte(0x10), 0x0F, 0, 0, 1, 1),
            (main.Opcode.MVI_E, main.Opcode.DCR_E, main.Data.byte(0x80), 0x7F, 0, 0, 0, 1),
            (main.Opcode.MVI_H, main.Opcode.DCR_H, main.Data.byte(0x02), 0x01, 0, 0, 0, 0),
            (main.Opcode.MVI_L, main.Opcode.DCR_L, main.Data.byte(0x03), 0x02, 0, 0, 0, 0),
        ]

        for mvi_op, dcr_op, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=dcr_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(mvi_op, init_val),
                    main.Instruction(dcr_op),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.words(0x01, 0x00), main.Data.byte(0x06), 0x05, 0, 0, 1, 0),
            (main.Data.words(0x02, 0x00), main.Data.byte(0x01), 0x00, 1, 0, 1, 0),
            (main.Data.words(0x03, 0x00), main.Data.byte(0x00), 0xFF, 0, 1, 1, 1),
            (main.Data.words(0x04, 0x00), main.Data.byte(0x10), 0x0F, 0, 0, 1, 1),
        ]

        for ptr, init_val, exp_val, exp_z, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, init_val)

                program = main.Program([
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.DCR_M),
                    main.Instruction(main.Opcode.HLT),
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
                (main.Data.byte(0xFF), main.Opcode.MVI_B, main.Opcode.ANA_B, main.Data.byte(0x0F), 0x0F, 0, 0, 0, 1, 1),
                (main.Data.byte(0xAA), main.Opcode.MVI_C, main.Opcode.ANA_C, main.Data.byte(0x55), 0x00, 1, 0, 0, 1, 1),
                (main.Data.byte(0x80), main.Opcode.MVI_D, main.Opcode.ANA_D, main.Data.byte(0x80), 0x80, 0, 0, 1, 0, 1),
                (main.Data.byte(0xF0), main.Opcode.MVI_E, main.Opcode.ANA_E, main.Data.byte(0x0F), 0x00, 1, 0, 0, 1, 1),
                (main.Data.byte(0x33), main.Opcode.MVI_H, main.Opcode.ANA_H, main.Data.byte(0xCC), 0x00, 1, 0, 0, 1, 1),
                (main.Data.byte(0x0F), main.Opcode.MVI_L, main.Opcode.ANA_L, main.Data.byte(0x07), 0x07, 0, 0, 0, 0, 1),
                (main.Data.byte(0x12), main.Opcode.MVI_A, main.Opcode.ANA_A, main.Data.byte(0x12), 0x12, 0, 0, 0, 1, 1),
            ]

            for val_a, mvi_src, ana_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
                with self.subTest(op=ana_op.name):
                    machine = main.Machine.create(self.address_lines, self.data_lines)
                    cpu = machine.cpu

                    instructions = [main.Instruction(main.Opcode.MVI_A, val_a)]
                    if ana_op != main.Opcode.ANA_A:
                        instructions.append(main.Instruction(mvi_src, val_src))
                    instructions.extend([
                        main.Instruction(ana_op),
                        main.Instruction(main.Opcode.HLT),
                    ])

                    program = main.Program(instructions)
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
            (main.Data.byte(0xFF), main.Data.words(0x01, 0x00), main.Data.byte(0x0F), 0x0F, 0, 0, 0, 1, 1),
            (main.Data.byte(0xAA), main.Data.words(0x02, 0x00), main.Data.byte(0x55), 0x00, 1, 0, 0, 1, 1),
            (main.Data.byte(0x80), main.Data.words(0x03, 0x00), main.Data.byte(0x80), 0x80, 0, 0, 1, 0, 1),
            (main.Data.byte(0xF0), main.Data.words(0x04, 0x00), main.Data.byte(0x0F), 0x00, 1, 0, 0, 1, 1),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.ANA_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xFF), main.Data.byte(0x0F), 0x0F, 0, 0, 0, 1, 1),
            (main.Data.byte(0xAA), main.Data.byte(0x55), 0x00, 1, 0, 0, 1, 1),
            (main.Data.byte(0x80), main.Data.byte(0x80), 0x80, 0, 0, 1, 0, 1),
            (main.Data.byte(0xF0), main.Data.byte(0x0F), 0x00, 1, 0, 0, 1, 1),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.ANI, val_imm),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xF0), main.Opcode.MVI_B, main.Opcode.ORA_B, main.Data.byte(0x0F), 0xFF, 0, 0, 1, 1, 0),
            (main.Data.byte(0x00), main.Opcode.MVI_C, main.Opcode.ORA_C, main.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x80), main.Opcode.MVI_D, main.Opcode.ORA_D, main.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (main.Data.byte(0x01), main.Opcode.MVI_E, main.Opcode.ORA_E, main.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
            (main.Data.byte(0x55), main.Opcode.MVI_H, main.Opcode.ORA_H, main.Data.byte(0xAA), 0xFF, 0, 0, 1, 1, 0),
            (main.Data.byte(0x10), main.Opcode.MVI_L, main.Opcode.ORA_L, main.Data.byte(0x20), 0x30, 0, 0, 0, 1, 0),
            (main.Data.byte(0x12), main.Opcode.MVI_A, main.Opcode.ORA_A, main.Data.byte(0x12), 0x12, 0, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, ora_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=ora_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [main.Instruction(main.Opcode.MVI_A, val_a)]
                if ora_op != main.Opcode.ORA_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(ora_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0xF0), main.Data.words(0x01, 0x00), main.Data.byte(0x0F), 0xFF, 0, 0, 1, 1, 0),
            (main.Data.byte(0x00), main.Data.words(0x02, 0x00), main.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x80), main.Data.words(0x03, 0x00), main.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (main.Data.byte(0x01), main.Data.words(0x04, 0x00), main.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.ORA_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xF0), main.Data.byte(0x0F), 0xFF, 0, 0, 1, 1, 0),
            (main.Data.byte(0x00), main.Data.byte(0x00), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x80), main.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (main.Data.byte(0x01), main.Data.byte(0x02), 0x03, 0, 0, 0, 1, 0),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.ORI, val_imm),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xFF), main.Opcode.MVI_B, main.Opcode.XRA_B, main.Data.byte(0x0F), 0xF0, 0, 0, 1, 1, 0),
            (main.Data.byte(0xAA), main.Opcode.MVI_C, main.Opcode.XRA_C, main.Data.byte(0xAA), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x80), main.Opcode.MVI_D, main.Opcode.XRA_D, main.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (main.Data.byte(0x03), main.Opcode.MVI_E, main.Opcode.XRA_E, main.Data.byte(0x01), 0x02, 0, 0, 0, 0, 0),
            (main.Data.byte(0x55), main.Opcode.MVI_H, main.Opcode.XRA_H, main.Data.byte(0x55), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x10), main.Opcode.MVI_L, main.Opcode.XRA_L, main.Data.byte(0x20), 0x30, 0, 0, 0, 1, 0),
            (main.Data.byte(0x12), main.Opcode.MVI_A, main.Opcode.XRA_A, main.Data.byte(0x12), 0x00, 1, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, xra_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=xra_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [main.Instruction(main.Opcode.MVI_A, val_a)]
                if xra_op != main.Opcode.XRA_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(xra_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0xFF), main.Data.words(0x01, 0x00), main.Data.byte(0x0F), 0xF0, 0, 0, 1, 1, 0),
            (main.Data.byte(0xAA), main.Data.words(0x02, 0x00), main.Data.byte(0xAA), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x80), main.Data.words(0x03, 0x00), main.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (main.Data.byte(0x03), main.Data.words(0x04, 0x00), main.Data.byte(0x01), 0x02, 0, 0, 0, 0, 0),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.XRA_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xFF), main.Data.byte(0x0F), 0xF0, 0, 0, 1, 1, 0),
            (main.Data.byte(0xAA), main.Data.byte(0xAA), 0x00, 1, 0, 0, 1, 0),
            (main.Data.byte(0x80), main.Data.byte(0x01), 0x81, 0, 0, 1, 1, 0),
            (main.Data.byte(0x03), main.Data.byte(0x01), 0x02, 0, 0, 0, 0, 0),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.XRI, val_imm),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x05), main.Opcode.MVI_B, main.Opcode.CMP_B, main.Data.byte(0x03), 0x05, 0, 0, 0, 0, 0),
            (main.Data.byte(0x05), main.Opcode.MVI_C, main.Opcode.CMP_C, main.Data.byte(0x05), 0x05, 1, 0, 0, 1, 0),
            (main.Data.byte(0x03), main.Opcode.MVI_D, main.Opcode.CMP_D, main.Data.byte(0x05), 0x03, 0, 1, 1, 0, 1),
            (main.Data.byte(0x00), main.Opcode.MVI_E, main.Opcode.CMP_E, main.Data.byte(0x01), 0x00, 0, 1, 1, 1, 1),
            (main.Data.byte(0x10), main.Opcode.MVI_H, main.Opcode.CMP_H, main.Data.byte(0x01), 0x10, 0, 0, 0, 1, 1),
            (main.Data.byte(0x50), main.Opcode.MVI_L, main.Opcode.CMP_L, main.Data.byte(0x20), 0x50, 0, 0, 0, 1, 0),
            (main.Data.byte(0x12), main.Opcode.MVI_A, main.Opcode.CMP_A, main.Data.byte(0x12), 0x12, 1, 0, 0, 1, 0),
        ]

        for val_a, mvi_src, cmp_op, val_src, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(op=cmp_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = [main.Instruction(main.Opcode.MVI_A, val_a)]
                if cmp_op != main.Opcode.CMP_A:
                    instructions.append(main.Instruction(mvi_src, val_src))
                instructions.extend([
                    main.Instruction(cmp_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
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
            (main.Data.byte(0x05), main.Data.words(0x01, 0x00), main.Data.byte(0x03), 0x05, 0, 0, 0, 0, 0),
            (main.Data.byte(0x05), main.Data.words(0x02, 0x00), main.Data.byte(0x05), 0x05, 1, 0, 0, 1, 0),
            (main.Data.byte(0x03), main.Data.words(0x03, 0x00), main.Data.byte(0x05), 0x03, 0, 1, 1, 0, 1),
            (main.Data.byte(0x00), main.Data.words(0x04, 0x00), main.Data.byte(0x01), 0x00, 0, 1, 1, 1, 1),
        ]

        for val_a, ptr, val_mem, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(addr=hex(mem)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val_mem)

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(main.Opcode.CMP_M),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x05), main.Data.byte(0x03), 0x05, 0, 0, 0, 0, 0),
            (main.Data.byte(0x05), main.Data.byte(0x05), 0x05, 1, 0, 0, 1, 0),
            (main.Data.byte(0x03), main.Data.byte(0x05), 0x03, 0, 1, 1, 0, 1),
            (main.Data.byte(0x00), main.Data.byte(0x01), 0x00, 0, 1, 1, 1, 1),
        ]

        for val_a, val_imm, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(imm=hex(val_imm.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.CPI, val_imm),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x82), 0x05, 1),
            (main.Data.byte(0x01), 0x02, 0),
            (main.Data.byte(0x80), 0x01, 1),
            (main.Data.byte(0x7F), 0xFE, 0),
        ]

        for val_a, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.RLC),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_rotate_right_circular(self):
        data = [
            (main.Data.byte(0x81), 0xC0, 1),
            (main.Data.byte(0x02), 0x01, 0),
            (main.Data.byte(0x01), 0x80, 1),
            (main.Data.byte(0xFE), 0x7F, 0),
        ]

        for val_a, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.RRC),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_rotate_left_through_carry(self):
        data = [
            (main.Data.byte(0x80), 0, 0x00, 1),
            (main.Data.byte(0x01), 1, 0x03, 0),
            (main.Data.byte(0x82), 0, 0x04, 1),
            (main.Data.byte(0x7F), 1, 0xFF, 0),
        ]

        for val_a, init_carry, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value), carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.extend([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.RAL),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_rotate_right_through_carry(self):
        data = [
            (main.Data.byte(0x01), 0, 0x00, 1),
            (main.Data.byte(0x80), 1, 0xC0, 0),
            (main.Data.byte(0x81), 0, 0x40, 1),
            (main.Data.byte(0xFE), 1, 0xFF, 0),
        ]

        for val_a, init_carry, exp_a, exp_c in data:
            with self.subTest(val=hex(val_a.value), carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.extend([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.RAR),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, exp_a)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_cma(self):
        data = [
            (main.Data.byte(0x55), 0xAA),
            (main.Data.byte(0x00), 0xFF),
            (main.Data.byte(0xFF), 0x00),
            (main.Data.byte(0x80), 0x7F),
        ]

        for val_a, exp_a in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.CMA),
                    main.Instruction(main.Opcode.HLT),
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
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.extend([
                    main.Instruction(main.Opcode.CMC),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_stc(self):
        data = [0, 1]

        for init_carry in data:
            with self.subTest(carry=init_carry):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                instructions = []
                if init_carry == 1:
                    instructions.extend([
                        main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
                        main.Instruction(main.Opcode.MVI_B, main.Data.byte(0x01)),
                        main.Instruction(main.Opcode.ADD_B),
                    ])

                instructions.extend([
                    main.Instruction(main.Opcode.STC),
                    main.Instruction(main.Opcode.HLT),
                ])

                program = main.Program(instructions)
                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.flag_reg.carry, 1)


    def test_decimal_adjust_accumulator(self):
        data = [
            (main.Data.byte(0x7D), 0, 0, 0x83, 0, 0, 1, 0, 1),
            (main.Data.byte(0x9B), 0, 0, 0x01, 0, 1, 0, 0, 1),
            (main.Data.byte(0x12), 0, 0, 0x12, 0, 0, 0, 1, 0),
        ]

        for val_a, _, _, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.DAA),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xEE), 1, 1, 0x88, 0, 1, 1, 1, 1),
            (main.Data.byte(0x12), 0, 0, 0x12, 0, 0, 0, 1, 0),
        ]

        for val_a, _, _, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.DAS),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x0E), 0x04, 0, 1, 0, 0, 1),
            (main.Data.byte(0x05), 0x05, 0, 0, 0, 1, 0),
        ]

        for val_a, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.AAA),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0xFD), 0x07, 0, 1, 0, 0, 1),
            (main.Data.byte(0x05), 0x05, 0, 0, 0, 1, 0),
        ]

        for val_a, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(val=hex(val_a.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val_a),
                    main.Instruction(main.Opcode.AAS),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Opcode.MOV_A_M, main.Data.byte(0x01), main.Data.words(0x01, 0x00)),
            (main.Opcode.MOV_B_M, main.Data.byte(0x02), main.Data.words(0x02, 0x00)),
            (main.Opcode.MOV_C_M, main.Data.byte(0x03), main.Data.words(0x03, 0x00)),
            (main.Opcode.MOV_D_M, main.Data.byte(0x04), main.Data.words(0x04, 0x00)),
            (main.Opcode.MOV_E_M, main.Data.byte(0x05), main.Data.words(0x05, 0x00)),
            (main.Opcode.MOV_H_M, main.Data.byte(0x06), main.Data.words(0x06, 0x00)),
            (main.Opcode.MOV_L_M, main.Data.byte(0x07), main.Data.words(0x07, 0x00)),
        ]

        for mov_op, val, ptr in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(op=mov_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val)

                program = main.Program([
                    main.Instruction(main.Opcode.LXI, ptr),
                    main.Instruction(mov_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                reg_name = mov_op.name.split("_")[1].lower()
                reg_obj = getattr(cpu, f"reg_{reg_name}")
                self.assertEqual(reg_obj.value, val.value)


    def test_lda_indirect(self):
        data = [
            (main.Opcode.MVI_BC, main.Opcode.LDA_BC, main.Data.words(0x01, 0x00), main.Data.byte(0x42)),
            (main.Opcode.MVI_DE, main.Opcode.LDA_DE, main.Data.words(0x02, 0x00), main.Data.byte(0x84)),
        ]

        for mvi_rp, lda_rp, ptr, val in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(op=lda_rp.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(mem, val)

                program = main.Program([
                    main.Instruction(mvi_rp, ptr),
                    main.Instruction(lda_rp),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, val.value)


    def test_sta_indirect(self):
        data = [
            (main.Opcode.MVI_BC, main.Opcode.STA_BC, main.Data.words(0x01, 0x00), main.Data.byte(0x55)),
            (main.Opcode.MVI_DE, main.Opcode.STA_DE, main.Data.words(0x02, 0x00), main.Data.byte(0xAA)),
        ]

        for mvi_rp, sta_rp, ptr, val in data:
            mem = main.Mem(ptr.reverse().value)

            with self.subTest(op=sta_rp.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, val),
                    main.Instruction(mvi_rp, ptr),
                    main.Instruction(sta_rp),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(ram.read(mem).value, val.value)


    def test_lhld(self):
        data = [
            (main.Data.words(0x01, 0x00), main.Data.byte(0x45), main.Data.byte(0x12)),
            (main.Data.words(0x05, 0x00), main.Data.byte(0xEF), main.Data.byte(0xCD)),
        ]

        for ptr, val_low, val_high in data:
            addr_low = main.Mem(ptr.reverse().value)
            addr_high = main.Mem(addr_low + 1)

            with self.subTest(addr=hex(addr_low)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram
                cpu = machine.cpu

                ram.write(addr_low, val_low)
                ram.write(addr_high, val_high)

                program = main.Program([
                    main.Instruction(main.Opcode.LHLD, ptr),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_l.value, val_low.value)
                self.assertEqual(cpu.reg_h.value, val_high.value)


    def test_shld(self):
        data = [
            (main.Data.words(0x01, 0x00), main.Data.byte(0x45), main.Data.byte(0x12)),
            (main.Data.words(0x05, 0x00), main.Data.byte(0xEF), main.Data.byte(0xCD)),
        ]

        for ptr, val_low, val_high in data:
            addr_low = main.Mem(ptr.reverse().value)
            addr_high = main.Mem(addr_low + 1)

            with self.subTest(addr=hex(addr_low)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                ram = machine.ram

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_L, val_low),
                    main.Instruction(main.Opcode.MVI_H, val_high),
                    main.Instruction(main.Opcode.SHLD, ptr),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(ram.read(addr_low).value, val_low.value)
                self.assertEqual(ram.read(addr_high).value, val_high.value)


    def test_xchg(self):
        data = [
            (main.Data.byte(0x12), main.Data.byte(0x34), main.Data.byte(0x56), main.Data.byte(0x78)),
            (main.Data.byte(0xAA), main.Data.byte(0xBB), main.Data.byte(0xCC), main.Data.byte(0xDD)),
        ]

        for val_d, val_e, val_h, val_l in data:
            with self.subTest(d=hex(val_d.value), h=hex(val_h.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_D, val_d),
                    main.Instruction(main.Opcode.MVI_E, val_e),
                    main.Instruction(main.Opcode.MVI_H, val_h),
                    main.Instruction(main.Opcode.MVI_L, val_l),
                    main.Instruction(main.Opcode.XCHG),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_d.value, val_h.value)
                self.assertEqual(cpu.reg_e.value, val_l.value)
                self.assertEqual(cpu.reg_h.value, val_d.value)
                self.assertEqual(cpu.reg_l.value, val_e.value)


    def test_adi(self):
        data = [
            (main.Data.byte(0x05), main.Data.byte(0x03), 0x08, 0, 0, 0, 0, 0),
            (main.Data.byte(0xFF), main.Data.byte(0x01), 0x00, 1, 1, 0, 1, 1),
        ]

        for init_a, imm_val, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, init_a),
                    main.Instruction(main.Opcode.ADI, imm_val),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x05), main.Data.byte(0x03), 0, 0x08, 0, 0, 0, 0, 0),
            (main.Data.byte(0x05), main.Data.byte(0x03), 1, 0x09, 0, 0, 0, 1, 0),
        ]

        for init_a, imm_val, init_c, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value), cin=init_c):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu
                cpu.flag_reg.carry = init_c

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, init_a),
                    main.Instruction(main.Opcode.ACI, imm_val),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x05), main.Data.byte(0x03), 0x02, 0, 0, 0, 0, 0),
            (main.Data.byte(0x00), main.Data.byte(0x01), 0xFF, 0, 1, 1, 1, 1),
        ]

        for init_a, imm_val, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value)):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, init_a),
                    main.Instruction(main.Opcode.SUI, imm_val),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Data.byte(0x05), main.Data.byte(0x03), 0, 0x02, 0, 0, 0, 0, 0),
            (main.Data.byte(0x05), main.Data.byte(0x03), 1, 0x01, 0, 0, 0, 0, 0),
        ]

        for init_a, imm_val, init_c, exp_a, exp_z, exp_c, exp_s, exp_p, exp_ac in data:
            with self.subTest(a=hex(init_a.value), imm=hex(imm_val.value), cin=init_c):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu
                cpu.flag_reg.carry = init_c

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_A, init_a),
                    main.Instruction(main.Opcode.SBI, imm_val),
                    main.Instruction(main.Opcode.HLT),
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
            (main.Opcode.MVI_BC, main.Opcode.INX_BC, "pair_bc", main.Data.words(0xFF, 0x00), 0x0100),
            (main.Opcode.MVI_DE, main.Opcode.INX_DE, "pair_de", main.Data.words(0xFF, 0x00), 0x0100),
            (main.Opcode.MVI_HL, main.Opcode.INX_HL, "pair_hl", main.Data.words(0xFF, 0x00), 0x0100),
        ]

        for mvi_op, inx_op, pair_attr, init_val, exp_val in data:
            with self.subTest(op=inx_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(mvi_op, init_val),
                    main.Instruction(inx_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(getattr(cpu, pair_attr).value, exp_val)


    def test_dcx(self):
        data = [
            (main.Opcode.MVI_BC, main.Opcode.DCX_BC, "pair_bc", main.Data.words(0x00, 0x01), 0x00FF),
            (main.Opcode.MVI_DE, main.Opcode.DCX_DE, "pair_de", main.Data.words(0x00, 0x01), 0x00FF),
            (main.Opcode.MVI_HL, main.Opcode.DCX_HL, "pair_hl", main.Data.words(0x00, 0x01), 0x00FF),
        ]

        for mvi_op, dcx_op, pair_attr, init_val, exp_val in data:
            with self.subTest(op=dcx_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(mvi_op, init_val),
                    main.Instruction(dcx_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(getattr(cpu, pair_attr).value, exp_val)


    def test_dad(self):
        data = [
            (main.Opcode.MVI_BC, main.Opcode.DAD_BC, main.Data.words(0x00, 0x10), main.Data.words(0x00, 0x20), 0x3000, 0),
            (main.Opcode.MVI_DE, main.Opcode.DAD_DE, main.Data.words(0x00, 0x10), main.Data.words(0x00, 0x20), 0x3000, 0),
            (main.Opcode.MVI_HL, main.Opcode.DAD_HL, main.Data.words(0x00, 0x80), main.Data.words(0x00, 0x80), 0x0000, 1),
        ]

        for mvi_rp, dad_op, init_hl, init_rp, exp_hl, exp_c in data:
            with self.subTest(op=dad_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                program = main.Program([
                    main.Instruction(main.Opcode.MVI_HL, init_hl),
                    *( [main.Instruction(mvi_rp, init_rp)] if dad_op != main.Opcode.DAD_HL else [] ),
                    main.Instruction(dad_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.pair_hl.value, exp_hl)
                self.assertEqual(cpu.flag_reg.carry, exp_c)


    def test_inx_sp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = main.Program([
            main.Instruction(main.Opcode.INX_SP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_sp.value, 0x0000)


    def test_dcx_sp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        cpu.reg_sp.write(0x0000)

        program = main.Program([
            main.Instruction(main.Opcode.DCX_SP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_sp.value, 0xFFFF)


    def test_dad_sp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = main.Program([
            main.Instruction(main.Opcode.MVI_HL, main.Data.words(0x01, 0x00)),
            main.Instruction(main.Opcode.DAD_SP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.pair_hl.value, 0x0000)
        self.assertEqual(cpu.flag_reg.carry, 1)


    def test_push_pop(self):
        data = [
            (main.Opcode.MVI_BC, main.Opcode.PUSH_BC, main.Opcode.POP_BC, "pair_bc", main.Data.words(0x12, 0x34), 0x3412),
            (main.Opcode.MVI_DE, main.Opcode.PUSH_DE, main.Opcode.POP_DE, "pair_de", main.Data.words(0x56, 0x78), 0x7856),
            (main.Opcode.MVI_HL, main.Opcode.PUSH_HL, main.Opcode.POP_HL, "pair_hl", main.Data.words(0x90, 0xAB), 0xAB90),
        ]

        for mvi_op, push_op, pop_op, pair_attr, init_val, exp_val in data:
            with self.subTest(op=push_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu

                cpu.reg_sp.write(0x1000)

                program = main.Program([
                    main.Instruction(mvi_op, init_val),
                    main.Instruction(push_op),
                    main.Instruction(pop_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(getattr(cpu, pair_attr).value, exp_val)
                self.assertEqual(cpu.reg_sp.value, 0x1000)


    def test_push_pop_psw(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        cpu.reg_sp.write(0x1000)

        program = main.Program([
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xAB)),
            main.Instruction(main.Opcode.ADI, main.Data.byte(0x01)),
            main.Instruction(main.Opcode.PUSH_PSW),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x00)),
            main.Instruction(main.Opcode.POP_PSW),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0xAC)
        self.assertEqual(cpu.reg_sp.value, 0x1000)


    def test_sphl(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = main.Program([
            main.Instruction(main.Opcode.MVI_HL, main.Data.words(0x00, 0x20)),
            main.Instruction(main.Opcode.SPHL),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_sp.value, 0x2000)


    def test_xthl(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        cpu.reg_sp.write(0x1000)
        ram.write(main.Mem(0x1000), main.Data.byte(0x78))
        ram.write(main.Mem(0x1001), main.Data.byte(0x56))

        program = main.Program([
            main.Instruction(main.Opcode.MVI_HL, main.Data.words(0x34, 0x12)),
            main.Instruction(main.Opcode.XTHL),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_l.value, 0x78)
        self.assertEqual(cpu.reg_h.value, 0x56)
        self.assertEqual(ram.read(main.Mem(0x1000)).value, 0x34)
        self.assertEqual(ram.read(main.Mem(0x1001)).value, 0x12)
        self.assertEqual(cpu.reg_sp.value, 0x1000)


    def test_jmp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0050)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x42))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        program = main.Program([
            main.Instruction(main.Opcode.JMP, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xFF)),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x42)

    def test_call_and_ret(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_pchl(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0060)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x99))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        program = main.Program([
            main.Instruction(main.Opcode.MVI_HL, main.Data.words(0x60, 0x00)),
            main.Instruction(main.Opcode.PCHL),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x99)

    def test_jz(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.zero = 1
        program = main.Program([
            main.Instruction(main.Opcode.JZ, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.zero = 0
        program = main.Program([
            main.Instruction(main.Opcode.JZ, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jnz(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.zero = 0
        program = main.Program([
            main.Instruction(main.Opcode.JNZ, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.zero = 1
        program = main.Program([
            main.Instruction(main.Opcode.JNZ, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jc(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.carry = 1
        program = main.Program([
            main.Instruction(main.Opcode.JC, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.carry = 0
        program = main.Program([
            main.Instruction(main.Opcode.JC, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jnc(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.carry = 0
        program = main.Program([
            main.Instruction(main.Opcode.JNC, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.carry = 1
        program = main.Program([
            main.Instruction(main.Opcode.JNC, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.sign = 0
        program = main.Program([
            main.Instruction(main.Opcode.JP, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.sign = 1
        program = main.Program([
            main.Instruction(main.Opcode.JP, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jm(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.sign = 1
        program = main.Program([
            main.Instruction(main.Opcode.JM, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.sign = 0
        program = main.Program([
            main.Instruction(main.Opcode.JM, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jpe(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.parity = 1
        program = main.Program([
            main.Instruction(main.Opcode.JPE, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.parity = 0
        program = main.Program([
            main.Instruction(main.Opcode.JPE, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_jpo(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram

        target_addr = main.Mem(0x0070)
        ram.write(target_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(target_addr + 1), main.Data.byte(0x77))
        ram.write(main.Mem(target_addr + 2), main.Data.byte(main.Opcode.HLT))

        cpu.flag_reg.parity = 0
        program = main.Program([
            main.Instruction(main.Opcode.JPO, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x11)),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x77)

        cpu.flag_reg.parity = 1
        program = main.Program([
            main.Instruction(main.Opcode.JPO, main.Data.words(0x70, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)

    def test_cz(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.zero = 1
        program = main.Program([
            main.Instruction(main.Opcode.CZ, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 0
        program = main.Program([
            main.Instruction(main.Opcode.CZ, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cnz(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.zero = 0
        program = main.Program([
            main.Instruction(main.Opcode.CNZ, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 1
        program = main.Program([
            main.Instruction(main.Opcode.CNZ, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cc(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.carry = 1
        program = main.Program([
            main.Instruction(main.Opcode.CC, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 0
        program = main.Program([
            main.Instruction(main.Opcode.CC, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cnc(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.carry = 0
        program = main.Program([
            main.Instruction(main.Opcode.CNC, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 1
        program = main.Program([
            main.Instruction(main.Opcode.CNC, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.sign = 0
        program = main.Program([
            main.Instruction(main.Opcode.CP, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 1
        program = main.Program([
            main.Instruction(main.Opcode.CP, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cm(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.sign = 1
        program = main.Program([
            main.Instruction(main.Opcode.CM, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 0
        program = main.Program([
            main.Instruction(main.Opcode.CM, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cpe(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.parity = 1
        program = main.Program([
            main.Instruction(main.Opcode.CPE, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 0
        program = main.Program([
            main.Instruction(main.Opcode.CPE, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_cpo(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.parity = 0
        program = main.Program([
            main.Instruction(main.Opcode.CPO, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 1
        program = main.Program([
            main.Instruction(main.Opcode.CPO, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x22)),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x22)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rz(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RZ))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.zero = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rnz(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RNZ))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.zero = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.zero = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rc(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RC))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.carry = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rnc(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RNC))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.carry = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.carry = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rp(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RP))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.sign = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rm(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RM))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.sign = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.sign = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rpe(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RPE))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.parity = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_rpo(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu
        ram = machine.ram
        cpu.reg_sp.write(0x1000)

        subroutine_addr = main.Mem(0x0050)
        ram.write(subroutine_addr, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 1), main.Data.byte(0x55))
        ram.write(main.Mem(subroutine_addr + 2), main.Data.byte(main.Opcode.RPO))
        ram.write(main.Mem(subroutine_addr + 3), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(subroutine_addr + 4), main.Data.byte(0x99))
        ram.write(main.Mem(subroutine_addr + 5), main.Data.byte(main.Opcode.RET))

        cpu.flag_reg.parity = 0
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x56)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

        cpu.flag_reg.parity = 1
        program = main.Program([
            main.Instruction(main.Opcode.CALL, main.Data.words(0x50, 0x00)),
            main.Instruction(main.Opcode.INR_A),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, self.mem_exec_entry)
        machine.run()
        self.assertEqual(cpu.reg_a.value, 0x9A)
        self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_nop(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        program = main.Program([
            main.Instruction(main.Opcode.NOP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(cpu.reg_pc.value, self.mem_exec_entry + 2)

    def test_device_manager_tick(self):
        class MockDevice(main.Device):
            def __init__(self):
                self.ticks = 0

            @property
            def name(self) -> str:
                return "MockDevice"

            def tick(self, bus: main.SystemBus) -> None:
                self.ticks += 1

        machine = main.Machine.create(self.address_lines, self.data_lines)
        mock_dev = MockDevice()
        machine.device_manager.attach_device(mock_dev)

        program = main.Program([
            main.Instruction(main.Opcode.NOP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertTrue(mock_dev.ticks > 0)

    def test_rst_instructions(self):
        rst_data = [
            (main.Opcode.RST_0, 0x0000),
            (main.Opcode.RST_1, 0x0008),
            (main.Opcode.RST_2, 0x0010),
            (main.Opcode.RST_3, 0x0018),
            (main.Opcode.RST_4, 0x0020),
            (main.Opcode.RST_5, 0x0028),
            (main.Opcode.RST_6, 0x0030),
            (main.Opcode.RST_7, 0x0038),
        ]

        for rst_op, exp_vector in rst_data:
            with self.subTest(op=rst_op.name):
                machine = main.Machine.create(self.address_lines, self.data_lines)
                cpu = machine.cpu
                ram = machine.ram

                cpu.reg_sp.write(0x1000)

                # Set up ISR at target vector: MVI A, 0x77; RET
                isr_addr = main.Mem(exp_vector)
                ram.write(isr_addr, main.Data.byte(main.Opcode.MVI_A))
                ram.write(main.Mem(isr_addr + 1), main.Data.byte(0x77))
                ram.write(main.Mem(isr_addr + 2), main.Data.byte(main.Opcode.RET))

                program = main.Program([
                    main.Instruction(rst_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, self.mem_exec_entry)
                machine.run()

                self.assertEqual(cpu.reg_a.value, 0x77)
                self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_ei_di(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        self.assertFalse(cpu.inte)

        program_ei = main.Program([
            main.Instruction(main.Opcode.EI),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program_ei, self.mem_exec_entry)
        machine.run()
        self.assertTrue(cpu.inte)

        program_di = main.Program([
            main.Instruction(main.Opcode.DI),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program_di, self.mem_exec_entry)
        machine.run()
        self.assertFalse(cpu.inte)

    def test_in_out_port(self):
        class OutputDevice(main.Device):
            def __init__(self):
                self.received = []

            @property
            def name(self) -> str:
                return "OutputDevice"

            def port_write(self, port: int, data: int) -> None:
                self.received.append((port, data))

        machine = main.Machine.create(self.address_lines, self.data_lines)
        kbd = main.KeyboardDevice()
        out_dev = OutputDevice()

        machine.device_manager.attach_device(kbd, ports=[0x05])
        machine.device_manager.attach_device(out_dev, ports=[0x0A])

        kbd.trigger_key_press('K')

        program = main.Program([
            main.Instruction(main.Opcode.IN, main.Data.byte(0x05)),
            main.Instruction(main.Opcode.OUT, main.Data.byte(0x0A)),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, self.mem_exec_entry)
        machine.run()

        self.assertEqual(machine.cpu.reg_a.value, ord('K'))
        self.assertEqual(out_dev.received, [(0x0A, ord('K'))])

    def test_rim_sim(self):
        machine = main.Machine.create(self.address_lines, self.data_lines)
        cpu = machine.cpu

        # Set Accumulator to SIM configuration: MSE=1, M5.5=1, M7.5=1, SDE=1, SOD=1 -> 0xCD
        # Binary: 1100 1101 = 0xCD (SOD=1, SDE=1, R7.5=0, MSE=1, M7.5=1, M6.5=0, M5.5=1)
        sim_program = main.Program([
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0xCD)),
            main.Instruction(main.Opcode.SIM),
            main.Instruction(main.Opcode.EI),
            main.Instruction(main.Opcode.RIM),
            main.Instruction(main.Opcode.HLT),
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


class TestDevice(unittest.TestCase):

    def test_keyboard_device(self):
        kbd = main.KeyboardDevice()
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

        machine = main.Machine.create(address_lines=16, data_lines=8)
        machine.device_manager.attach_device(kbd, ports=[0x01])

        kbd.trigger_key_press('X')

        program = main.Program([
            main.Instruction(main.Opcode.IN, main.Data.byte(0x01)),
            main.Instruction(main.Opcode.MOV_B_A),
            main.Instruction(main.Opcode.IN, main.Data.byte(0x01)),
            main.Instruction(main.Opcode.MOV_C_A),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x00A0))
        machine.run()

        self.assertEqual(machine.cpu.reg_b.value, ord('X'))
        self.assertEqual(machine.cpu.reg_c.value, 0x00)

    def test_keyboard_device_interrupt(self):
        kbd = main.KeyboardDevice(interrupt_vector=1)
        self.assertEqual(kbd.on_inta(), 0xCF)  # RST 1 opcode = 0xCF

        kbd_vector3 = main.KeyboardDevice(interrupt_vector=3)
        self.assertEqual(kbd_vector3.on_inta(), 0xDF)  # RST 3 opcode = 0xDF

    def test_hardware_interrupt_trap(self):
        machine = main.Machine.create(16, 8)
        cpu = machine.cpu
        ram = machine.ram

        cpu.reg_sp.write(0x1000)

        # Set up TRAP ISR at vector 0x0024: MVI A, 0x88; RET
        ram.write(main.Mem(0x0024), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(0x0025), main.Data.byte(0x88))
        ram.write(main.Mem(0x0026), main.Data.byte(main.Opcode.RET))

        program = main.Program([
            main.Instruction(main.Opcode.NOP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x00A0))
        cpu.trap = True

        machine.tick()
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x88)

    def test_dma_hold_hlda(self):
        machine = main.Machine.create(16, 8)
        bus = machine.bus

        program = main.Program([
            main.Instruction(main.Opcode.NOP),
            main.Instruction(main.Opcode.HLT),
        ])
        machine.load(program, main.Mem(0x00A0))

        bus.hold = main.Data.on()
        machine.tick()

        self.assertEqual(bus.hlda.value, 1)

        bus.hold = main.Data.off()
        machine.tick()

        self.assertEqual(bus.hlda.value, 0)

    def test_hardware_reset(self):
        machine = main.Machine.create(16, 8)
        bus = machine.bus
        cpu = machine.cpu

        cpu.reg_pc.write(0x1234)
        cpu.inte = True

        bus.reset_in = main.Data.on()
        machine.tick()

        self.assertEqual(cpu.reg_pc.value, 0x0000)
        self.assertFalse(cpu.inte)
        self.assertEqual(bus.reset_out.value, 1)

        bus.reset_in = main.Data.off()
        machine.tick()
        self.assertEqual(bus.reset_out.value, 0)


class TestUSBDevice(unittest.TestCase):

    def test_usb_device_dma(self):
        machine = main.Machine.create(16, 8)
        usb = main.USBDevice()
        machine.device_manager.attach_device(usb, ports=[0x10])

        self.assertEqual(usb.name, "USBDevice")

        write_data = b"HELLO_DMA"
        usb.dma_write(machine, start_addr=0x0200, data=write_data)

        read_data = usb.dma_read(machine, start_addr=0x0200, length=len(write_data))
        self.assertEqual(read_data, write_data)

    def test_machine_create_with_devices_tuple(self):
        kbd = main.KeyboardDevice()
        usb = main.USBDevice()
        machine = main.Machine.create(
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

        kbd = main.KeyboardDevice()
        printer = main.PrinterDevice(output_callback=custom_printer_callback)

        machine = main.Machine.create(
            16, 8,
            devices=[(kbd, [0x01]), (printer, [0x02])]
        )

        self.assertEqual(printer.name, "PrinterDevice")

        kbd.trigger_key_press('H')
        kbd.trigger_key_press('I')

        program = main.Program([
            main.Instruction(main.Opcode.IN, main.Data.byte(0x01)),
            main.Instruction(main.Opcode.OUT, main.Data.byte(0x02)),
            main.Instruction(main.Opcode.IN, main.Data.byte(0x01)),
            main.Instruction(main.Opcode.OUT, main.Data.byte(0x02)),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x00A0))
        machine.run()

        self.assertEqual(printed_output, ['H', 'I'])
        self.assertEqual(printer.history, ['H', 'I'])


class TestSoftwareInterrupt(unittest.TestCase):

    def test_software_interrupt_vectors(self):
        rst_vectors = [
            (main.Opcode.RST_0, main.VEC_RST_0),
            (main.Opcode.RST_1, main.VEC_RST_1),
            (main.Opcode.RST_2, main.VEC_RST_2),
            (main.Opcode.RST_3, main.VEC_RST_3),
            (main.Opcode.RST_4, main.VEC_RST_4),
            (main.Opcode.RST_5, main.VEC_RST_5),
            (main.Opcode.RST_6, main.VEC_RST_6),
            (main.Opcode.RST_7, main.VEC_RST_7),
        ]

        for rst_op, expected_vector in rst_vectors:
            with self.subTest(rst=rst_op.name):
                machine = main.Machine.create(16, 8)
                cpu, ram = machine.cpu, machine.ram
                cpu.reg_sp.write(0x1000)

                isr_addr = expected_vector
                ram.write(isr_addr, main.Data.byte(main.Opcode.MVI_A))
                ram.write(main.Mem(isr_addr + 1), main.Data.byte(0x55))
                ram.write(main.Mem(isr_addr + 2), main.Data.byte(main.Opcode.RET))

                program = main.Program([
                    main.Instruction(rst_op),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, main.Mem(0x00A0))
                machine.run()

                self.assertEqual(cpu.reg_a.value, 0x55)
                self.assertEqual(cpu.reg_sp.value, 0x1000)

    def test_nested_software_interrupts(self):
        machine = main.Machine.create(16, 8)
        cpu, ram = machine.cpu, machine.ram
        cpu.reg_sp.write(0x1000)

        ram.write(main.VEC_RST_1, main.Data.byte(main.Opcode.MVI_B))
        ram.write(main.Mem(main.VEC_RST_1 + 1), main.Data.byte(0x11))
        ram.write(main.Mem(main.VEC_RST_1 + 2), main.Data.byte(main.Opcode.RST_3))
        ram.write(main.Mem(main.VEC_RST_1 + 3), main.Data.byte(main.Opcode.RET))

        ram.write(main.VEC_RST_3, main.Data.byte(main.Opcode.MVI_C))
        ram.write(main.Mem(main.VEC_RST_3 + 1), main.Data.byte(0x33))
        ram.write(main.Mem(main.VEC_RST_3 + 2), main.Data.byte(main.Opcode.RET))

        program = main.Program([
            main.Instruction(main.Opcode.RST_1),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x00A0))
        machine.run()

        self.assertEqual(cpu.reg_b.value, 0x11)
        self.assertEqual(cpu.reg_c.value, 0x33)
        self.assertEqual(cpu.reg_sp.value, 0x1000)


class TestHardwareInterrupt(unittest.TestCase):

    def test_trap_interrupt(self):
        machine = main.Machine.create(16, 8)
        cpu, ram = machine.cpu, machine.ram
        cpu.reg_sp.write(0x1000)

        ram.write(main.VEC_TRAP, main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(main.VEC_TRAP + 1), main.Data.byte(0x88))
        ram.write(main.Mem(main.VEC_TRAP + 2), main.Data.byte(main.Opcode.RET))

        program = main.Program([
            main.Instruction(main.Opcode.NOP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x00A0))
        cpu.trap = True

        machine.tick()
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x88)

    def test_rst_7_5_6_5_5_5_interrupts(self):
        tests_data = [
            ("rst_7_5", main.VEC_RST_7_5, 0x75),
            ("rst_6_5", main.VEC_RST_6_5, 0x65),
            ("rst_5_5", main.VEC_RST_5_5, 0x55),
        ]

        for pin_name, vector_addr, reg_val in tests_data:
            with self.subTest(pin=pin_name):
                machine = main.Machine.create(16, 8)
                cpu, ram = machine.cpu, machine.ram
                cpu.reg_sp.write(0x1000)
                cpu.inte = True

                ram.write(main.Mem(vector_addr), main.Data.byte(main.Opcode.MVI_A))
                ram.write(main.Mem(vector_addr + 1), main.Data.byte(reg_val))
                ram.write(main.Mem(vector_addr + 2), main.Data.byte(main.Opcode.RET))

                program = main.Program([
                    main.Instruction(main.Opcode.NOP),
                    main.Instruction(main.Opcode.HLT),
                ])

                machine.load(program, main.Mem(0x00A0))
                setattr(cpu, pin_name, True)

                machine.tick()
                machine.run()

                self.assertEqual(cpu.reg_a.value, reg_val)

    def test_intr_inta_cycle(self):
        kbd = main.KeyboardDevice(interrupt_vector=2)
        machine = main.Machine.create(16, 8, devices=[(kbd, [0x01])])
        cpu, ram = machine.cpu, machine.ram
        cpu.reg_sp.write(0x1000)

        ram.write(main.Mem(0x0010), main.Data.byte(main.Opcode.MVI_A))
        ram.write(main.Mem(0x0011), main.Data.byte(0x99))
        ram.write(main.Mem(0x0012), main.Data.byte(main.Opcode.RET))

        kbd.trigger_key_press('K')

        program = main.Program([
            main.Instruction(main.Opcode.EI),
            main.Instruction(main.Opcode.NOP),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x00A0))
        cpu.intr = True

        machine.run()

        self.assertEqual(kbd.on_inta(), 0xD7)


class TestLabels(unittest.TestCase):
    """Tests for the instruction label referencing feature."""

    def test_jmp_with_label(self):
        machine = main.Machine.create(16, 8)
        cpu = machine.cpu

        # LOOP: DCR A
        #       JNZ LOOP
        #       HLT
        program = main.Program([
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(5)),
            main.Instruction(main.Opcode.DCR_A, label="LOOP"),
            main.Instruction(main.Opcode.JNZ, "LOOP"),
            main.Instruction(main.Opcode.HLT),
        ])

        machine.load(program, main.Mem(0x0000))
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0)

    def test_call_with_label(self):
        machine = main.Machine.create(16, 8)
        cpu = machine.cpu
        cpu.reg_sp.write(0x1000)

        #       CALL SUBR
        #       HLT
        # SUBR: MVI A, 0x77
        #       RET
        program = main.Program([
            main.Instruction(main.Opcode.CALL, "SUBR"),
            main.Instruction(main.Opcode.HLT),
            main.Instruction(main.Opcode.MVI_A, main.Data.byte(0x77), label="SUBR"),
            main.Instruction(main.Opcode.RET),
        ])

        machine.load(program, main.Mem(0x0000))
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x77)

    def test_lxi_with_label(self):
        machine = main.Machine.create(16, 8)
        cpu = machine.cpu

        #       LXI H, TARGET
        #       HLT
        # TARGET: NOP
        program = main.Program([
            main.Instruction(main.Opcode.LXI, "TARGET"),
            main.Instruction(main.Opcode.HLT),
            main.Instruction(main.Opcode.NOP, label="TARGET"),
        ])

        machine.load(program, main.Mem(0x0100))
        machine.run()

        # TARGET is the 3rd instruction.
        # size of LXI is 3 bytes (loaded at 0x0100 -> 0x0100, 0x0101, 0x0102).
        # size of HLT is 1 byte (loaded at 0x0103).
        # TARGET is loaded at 0x0104.
        self.assertEqual(cpu.pair_hl.value, 0x0104)
