import unittest

import emu_8085


# NOTE:
# 1) Never use default test memory address or default test data value '0x0'. Due to default
# initial value.
# 2) Inside instruction provide address as Data instead Mem.
class TestLabels(unittest.TestCase):
    """Tests for the instruction label referencing feature."""

    def test_jmp_with_label(self):
        machine = emu_8085.Machine.create(16, 8)
        cpu = machine.cpu

        # LOOP: DCR A
        #       JNZ LOOP
        #       HLT
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(5)),
            emu_8085.Instruction(emu_8085.Opcode.DCR_A, label="LOOP"),
            emu_8085.Instruction(emu_8085.Opcode.JNZ, "LOOP"),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
        ])

        machine.load(program, emu_8085.Mem(0x0000))
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0)

    def test_call_with_label(self):
        machine = emu_8085.Machine.create(16, 8)
        cpu = machine.cpu
        cpu.reg_sp.write(0x1000)

        #       CALL SUBR
        #       HLT
        # SUBR: MVI A, 0x77
        #       RET
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.CALL, "SUBR"),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
            emu_8085.Instruction(emu_8085.Opcode.MVI_A, emu_8085.Data.byte(0x77), label="SUBR"),
            emu_8085.Instruction(emu_8085.Opcode.RET),
        ])

        machine.load(program, emu_8085.Mem(0x0000))
        machine.run()

        self.assertEqual(cpu.reg_a.value, 0x77)

    def test_lxi_with_label(self):
        machine = emu_8085.Machine.create(16, 8)
        cpu = machine.cpu

        #       LXI H, TARGET
        #       HLT
        # TARGET: NOP
        program = emu_8085.Program([
            emu_8085.Instruction(emu_8085.Opcode.LXI, "TARGET"),
            emu_8085.Instruction(emu_8085.Opcode.HLT),
            emu_8085.Instruction(emu_8085.Opcode.NOP, label="TARGET"),
        ])

        machine.load(program, emu_8085.Mem(0x0100))
        machine.run()

        # TARGET is the 3rd instruction.
        # size of LXI is 3 bytes (loaded at 0x0100 -> 0x0100, 0x0101, 0x0102).
        # size of HLT is 1 byte (loaded at 0x0103).
        # TARGET is loaded at 0x0104.
        self.assertEqual(cpu.pair_hl.value, 0x0104)


