"""
This module provides the cpu hardware component.
"""

from collections.abc import Sequence
from dataclasses import dataclass, field
from enum import StrEnum, auto
from typing import Callable, Literal

from emu_8085.core import (
    VEC_RST_0,
    VEC_RST_1,
    VEC_RST_2,
    VEC_RST_3,
    VEC_RST_4,
    VEC_RST_5,
    VEC_RST_5_5,
    VEC_RST_6,
    VEC_RST_6_5,
    VEC_RST_7,
    VEC_RST_7_5,
    VEC_TRAP,
    Data,
    DataSize,
    Mem,
)
from emu_8085.program.opcode import Opcode

from .registers import FlagRegister, InstructionRegister, Register, RegisterRef
from .system_bus import SystemBus


class MachineCycle(StrEnum):
    """Represents a one machine cycle."""

    FETCH = auto()
    EXECUTE = auto()
    HOLD = auto()
    WAIT = auto()


@dataclass(repr=False)
class CPU:
    """Central processing unit."""

    reg_a: Register = field(init=False, default_factory=lambda: Register.byte('a'))
    """General register 'A'."""

    reg_b: Register = field(init=False, default_factory=lambda: Register.byte('b'))
    """General register 'B'."""

    reg_c: Register = field(init=False, default_factory=lambda: Register.byte('c'))
    """General register 'C'."""

    reg_d: Register = field(init=False, default_factory=lambda: Register.byte('d'))
    """General register 'D'."""

    reg_e: Register = field(init=False, default_factory=lambda: Register.byte('e'))
    """General register 'E'."""

    reg_h: Register = field(init=False, default_factory=lambda: Register.byte('h'))
    """General register 'H'."""

    reg_l: Register = field(init=False, default_factory=lambda: Register.byte('l'))
    """General register 'L'."""

    reg_w: Register = field(init=False, default_factory=lambda: Register.byte('w'))
    """Internal register 'W'."""

    reg_z: Register = field(init=False, default_factory=lambda: Register.byte('z'))
    """Internal register 'Z'."""

    reg_tmp: Register = field(init=False, default_factory=lambda: Register.byte('tmp'))
    """Internal temporary register."""

    reg_sp: Register = field(init=False, default_factory=lambda: Register.word('sp', value=0xFFFF))
    """Stack pointer."""

    flag_reg: FlagRegister = field(init=False, default_factory=lambda: FlagRegister())
    """Flag resgister."""

    reg_pc: Register = field(init=False, default_factory=lambda: InstructionRegister.word('pc'))
    """Program counter."""

    _cycle: MachineCycle = field(init=False, default=MachineCycle.FETCH)
    """Represents machine cycle."""

    t_state: int = field(init=False, default=0)
    """Represents cpu t state."""

    ireg: InstructionRegister = field(init=False, default_factory=lambda: InstructionRegister.word('ir'))
    """Represents instruction register."""

    is_halt: bool = field(init=False, default=True)
    """CPU halt state."""

    inte: bool = field(init=False, default=False)
    """Interrupt enable flip-flop."""

    mask_5_5: bool = field(init=False, default=False)
    """RST 5.5 mask bit."""

    mask_6_5: bool = field(init=False, default=False)
    """RST 6.5 mask bit."""

    mask_7_5: bool = field(init=False, default=False)
    """RST 7.5 mask bit."""

    pending_5_5: bool = field(init=False, default=False)
    """RST 5.5 pending flag."""

    pending_6_5: bool = field(init=False, default=False)
    """RST 6.5 pending flag."""

    pending_7_5: bool = field(init=False, default=False)
    """RST 7.5 pending flag."""

    sid: bool = field(init=False, default=False)
    """Serial input data pin."""

    sod: bool = field(init=False, default=False)
    """Serial output data pin."""

    trap: bool = field(init=False, default=False)
    """TRAP non-maskable hardware interrupt pin."""

    rst_7_5: bool = field(init=False, default=False)
    """RST 7.5 hardware interrupt pin."""

    rst_6_5: bool = field(init=False, default=False)
    """RST 6.5 hardware interrupt pin."""

    rst_5_5: bool = field(init=False, default=False)
    """RST 5.5 hardware interrupt pin."""

    intr: bool = field(init=False, default=False)
    """INTR maskable hardware interrupt pin."""

    _is_inta_cycle: bool = field(init=False, default=False)
    """Tracks if the current fetch is an INTA cycle."""

    _reg_src: RegisterRef = field(init=False, default_factory=lambda: RegisterRef())
    """Source (read) register to consider during execute."""

    _reg_dst: RegisterRef = field(init=False, default_factory=lambda: RegisterRef())
    """Destination (write) register to consider during execute."""

    _exec_mem: Mem = field(init=False, default=Mem(0))
    """Memory address to consider."""

    _decoder_matrix: dict[int, Callable[[SystemBus], MachineCycle]] = field(init=False, default_factory=dict)
    """Decoder decodes the instruction next cycle."""

    _dispatch_table: dict[int, Sequence[Callable[[SystemBus], None]]] = field(init=False, default_factory=dict)
    """Dispatch table for execution order of excute machine cycle."""

    def __post_init__(self):
        def _bind(
            dst: Sequence[Register] | None = None,
            src: Sequence[Register] | None = None,
        ) -> None:
            if dst is not None:
                self._reg_dst.set(dst)
            if src is not None:
                self._reg_src.set(src)

        def decode_exec(
            dst: Sequence[Register] | None = None,
            src: Sequence[Register] | None = None,
        ) -> Callable[[SystemBus], MachineCycle]:
            def _fn(bus: SystemBus) -> MachineCycle:
                _bind(dst, src)
                return MachineCycle.EXECUTE
            return _fn

        def decode_fetch(
            action: Callable[[SystemBus], None],
            dst: Sequence[Register] | None = None,
            src: Sequence[Register] | None = None,
        ) -> Callable[[SystemBus], MachineCycle]:
            def _fn(bus: SystemBus) -> MachineCycle:
                _bind(dst, src)
                action(bus)
                return MachineCycle.FETCH
            return _fn

        matrix: dict[int, Callable[[SystemBus], MachineCycle]] = {
            Opcode.MVI_A: decode_exec(dst=[self.reg_a]),
            Opcode.MVI_B: decode_exec(dst=[self.reg_b]),
            Opcode.MVI_C: decode_exec(dst=[self.reg_c]),
            Opcode.MVI_D: decode_exec(dst=[self.reg_d]),
            Opcode.MVI_E: decode_exec(dst=[self.reg_e]),
            Opcode.MVI_H: decode_exec(dst=[self.reg_h]),
            Opcode.MVI_L: decode_exec(dst=[self.reg_l]),
            Opcode.MVI_BC: decode_exec(dst=[self.reg_b, self.reg_c]),
            Opcode.MVI_DE: decode_exec(dst=[self.reg_d, self.reg_e]),
            Opcode.MVI_HL: decode_exec(dst=[self.reg_h, self.reg_l]),
            Opcode.LXI: decode_exec(dst=[self.reg_h, self.reg_l]),
            Opcode.MVI_M: decode_exec(dst=[self.reg_h, self.reg_l]),

            Opcode.MOV_M_A: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_a]),
            Opcode.MOV_M_B: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_b]),
            Opcode.MOV_M_C: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_c]),
            Opcode.MOV_M_D: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_d]),
            Opcode.MOV_M_E: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_e]),
            Opcode.MOV_M_H: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_h]),
            Opcode.MOV_M_L: decode_exec(dst=[self.reg_h, self.reg_l], src=[self.reg_l]),

            Opcode.MOV_A_M: decode_exec(dst=[self.reg_a]),
            Opcode.MOV_B_M: decode_exec(dst=[self.reg_b]),
            Opcode.MOV_C_M: decode_exec(dst=[self.reg_c]),
            Opcode.MOV_D_M: decode_exec(dst=[self.reg_d]),
            Opcode.MOV_E_M: decode_exec(dst=[self.reg_e]),
            Opcode.MOV_H_M: decode_exec(dst=[self.reg_h]),
            Opcode.MOV_L_M: decode_exec(dst=[self.reg_l]),

            Opcode.LDA: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.STA: decode_exec(dst=[self.reg_w, self.reg_z], src=[self.reg_a]),
            Opcode.LDA_BC: decode_exec(),
            Opcode.LDA_DE: decode_exec(),
            Opcode.STA_BC: decode_exec(),
            Opcode.STA_DE: decode_exec(),
            Opcode.LHLD: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.SHLD: decode_exec(dst=[self.reg_w, self.reg_z]),

            Opcode.ADD_M: decode_exec(),
            Opcode.ADC_M: decode_exec(),
            Opcode.SUB_M: decode_exec(),
            Opcode.SBB_M: decode_exec(),
            Opcode.INR_M: decode_exec(),
            Opcode.DCR_M: decode_exec(),
            Opcode.ANA_M: decode_exec(),
            Opcode.ANI: decode_exec(),
            Opcode.ORA_M: decode_exec(),
            Opcode.ORI: decode_exec(),
            Opcode.XRA_M: decode_exec(),
            Opcode.XRI: decode_exec(),
            Opcode.CMP_M: decode_exec(),
            Opcode.CPI: decode_exec(),
            Opcode.ADI: decode_exec(),
            Opcode.ACI: decode_exec(),
            Opcode.SUI: decode_exec(),
            Opcode.SBI: decode_exec(),

            Opcode.INX_BC: decode_exec(),
            Opcode.INX_DE: decode_exec(),
            Opcode.INX_HL: decode_exec(),
            Opcode.INX_SP: decode_exec(),
            Opcode.DCX_BC: decode_exec(),
            Opcode.DCX_DE: decode_exec(),
            Opcode.DCX_HL: decode_exec(),
            Opcode.DCX_SP: decode_exec(),
            Opcode.DAD_BC: decode_exec(),
            Opcode.DAD_DE: decode_exec(),
            Opcode.DAD_HL: decode_exec(),
            Opcode.DAD_SP: decode_exec(),

            Opcode.PUSH_BC: decode_exec(),
            Opcode.PUSH_DE: decode_exec(),
            Opcode.PUSH_HL: decode_exec(),
            Opcode.PUSH_PSW: decode_exec(),
            Opcode.POP_BC: decode_exec(),
            Opcode.POP_DE: decode_exec(),
            Opcode.POP_HL: decode_exec(),
            Opcode.POP_PSW: decode_exec(),
            Opcode.XTHL: decode_exec(),
            Opcode.SPHL: decode_exec(),

            # 4 T-State single-byte operations (Execute at T4 of FETCH)
            Opcode.NOP: decode_fetch(self._ts_exec_nop),
            Opcode.XCHG: decode_fetch(self._ts_exec_xchg),
            Opcode.RLC: decode_fetch(self._ts_exec_rlc),
            Opcode.RRC: decode_fetch(self._ts_exec_rrc),
            Opcode.RAL: decode_fetch(self._ts_exec_ral),
            Opcode.RAR: decode_fetch(self._ts_exec_rar),
            Opcode.CMA: decode_fetch(self._ts_exec_cma),
            Opcode.CMC: decode_fetch(self._ts_exec_cmc),
            Opcode.STC: decode_fetch(self._ts_exec_stc),
            Opcode.DAA: decode_fetch(self._ts_exec_daa),
            Opcode.DAS: decode_fetch(self._ts_exec_das),
            Opcode.AAA: decode_fetch(self._ts_exec_aaa),
            Opcode.AAS: decode_fetch(self._ts_exec_aas),

            Opcode.JMP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JNZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JNC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JM: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JPE: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.JPO: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CALL: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CNZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CNC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CM: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CPE: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.CPO: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RET: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RNZ: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RNC: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RP: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RM: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RPE: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.RPO: decode_exec(dst=[self.reg_w, self.reg_z]),
            Opcode.PCHL: decode_exec(dst=[self.reg_h, self.reg_l]),
            Opcode.RST_0: decode_exec(),
            Opcode.RST_1: decode_exec(),
            Opcode.RST_2: decode_exec(),
            Opcode.RST_3: decode_exec(),
            Opcode.RST_4: decode_exec(),
            Opcode.RST_5: decode_exec(),
            Opcode.RST_6: decode_exec(),
            Opcode.RST_7: decode_exec(),
            Opcode.EI: decode_fetch(self._ts_exec_ei),
            Opcode.DI: decode_fetch(self._ts_exec_di),
            Opcode.RIM: decode_fetch(self._ts_exec_rim),
            Opcode.SIM: decode_fetch(self._ts_exec_sim),
            Opcode.IN: decode_exec(dst=[self.reg_z]),
            Opcode.OUT: decode_exec(dst=[self.reg_z], src=[self.reg_a]),
        }

        # MOV r1, r2
        regs = ("A", "B", "C", "D", "E", "H", "L")
        for r1 in regs:
            for r2 in regs:
                op = getattr(Opcode, f"MOV_{r1}_{r2}", None)
                if op:
                    matrix[op.value] = decode_fetch(
                        self._ts_exce_set_reg_from_reg,
                        dst=[self.reg(r1)],
                        src=[self.reg(r2)],
                    )

        # Register arithmetic / logic
        ops_map = [
            ("ADD", self._ts_exec_add),
            ("ADC", self._ts_exec_add_with_carry),
            ("SUB", self._ts_exec_sub),
            ("SBB", self._ts_exec_sub_with_borrow),
            ("INR", self._ts_exec_inr),
            ("DCR", self._ts_exec_dcr),
            ("ANA", self._ts_exec_ana),
            ("ORA", self._ts_exec_ora),
            ("XRA", self._ts_exec_xra),
            ("CMP", self._ts_exec_cmp),
        ]
        for prefix, handler in ops_map:
            for r in regs:
                op = getattr(Opcode, f"{prefix}_{r}", None)
                if op:
                    if prefix in ("INR", "DCR"):
                        matrix[op.value] = decode_fetch(handler, dst=[self.reg(r)])
                    else:
                        matrix[op.value] = decode_fetch(handler, src=[self.reg(r)])

        self._decoder_matrix = matrix

        set_reg: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_pc,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
        ]

        set_reg_pair: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_pc,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
            self._ts_exec_set_bus_addr_from_pc,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
        ]

        set_reg_to_mem: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_hl_reg,
            self._ts_exec_set_bus_data_from_reg,
            self._ts_exec_set_bus_mw,
        ]

        set_mem_to_reg: Sequence[Callable[[SystemBus], None]] = [
            self._ts_exec_set_bus_addr_from_hl_reg,
            self._ts_exec_set_bus_mr,
            lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
        ]

        self._dispatch_table = {
            Opcode.MVI_A: set_reg,
            Opcode.MVI_B: set_reg,
            Opcode.MVI_C: set_reg,
            Opcode.MVI_D: set_reg,
            Opcode.MVI_E: set_reg,
            Opcode.MVI_H: set_reg,
            Opcode.MVI_L: set_reg,
            Opcode.MVI_BC: set_reg_pair,
            Opcode.MVI_DE: set_reg_pair,
            Opcode.MVI_HL: set_reg_pair,
            Opcode.LXI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
            ],
            Opcode.MOV_M_A: set_reg_to_mem,
            Opcode.MOV_M_B: set_reg_to_mem,
            Opcode.MOV_M_C: set_reg_to_mem,
            Opcode.MOV_M_D: set_reg_to_mem,
            Opcode.MOV_M_E: set_reg_to_mem,
            Opcode.MOV_M_H: set_reg_to_mem,
            Opcode.MOV_M_L: set_reg_to_mem,
            Opcode.MOV_A_M: set_mem_to_reg,
            Opcode.MOV_B_M: set_mem_to_reg,
            Opcode.MOV_C_M: set_mem_to_reg,
            Opcode.MOV_D_M: set_mem_to_reg,
            Opcode.MOV_E_M: set_mem_to_reg,
            Opcode.MOV_H_M: set_mem_to_reg,
            Opcode.MOV_L_M: set_mem_to_reg,
            Opcode.MVI_M: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_tmp_reg_val_from_bus_data,
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_data_from_tmp_reg,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.LDA: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_a_val_from_bus_data,
            ],
            Opcode.STA: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_data_from_reg,
                self._ts_exec_set_bus_mw
            ],
            Opcode.LDA_BC: [
                self._ts_exec_set_bus_addr_from_bc_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_a_val_from_bus_data,
            ],
            Opcode.LDA_DE: [
                self._ts_exec_set_bus_addr_from_de_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_a_val_from_bus_data,
            ],
            Opcode.STA_BC: [
                self._ts_exec_set_bus_addr_from_bc_reg,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.STA_DE: [
                self._ts_exec_set_bus_addr_from_de_reg,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.LHLD: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_l_val_from_bus_data,
                self._ts_exec_set_bus_addr_from_wz_plus_1,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_reg_h_val_from_bus_data,
            ],
            Opcode.SHLD: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_set_bus_addr_from_wz_reg,
                self._ts_exec_set_bus_data_from_reg_l,
                self._ts_exec_set_bus_mw,
                self._ts_exec_set_bus_addr_from_wz_plus_1,
                self._ts_exec_set_bus_data_from_reg_h,
                self._ts_exec_set_bus_mw,
            ],
            Opcode.ADD_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.ADC_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.SUB_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.SBB_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.INR_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_tmp_reg_val_from_bus_data,
                self._ts_exec_inr,
                self._ts_exec_set_bus_mw,
                self._ts_exec_set_bus_data_from_tmp_reg,
            ],
            Opcode.DCR_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_tmp_reg_val_from_bus_data,
                self._ts_exec_dcr,
                self._ts_exec_set_bus_mw,
                self._ts_exec_set_bus_data_from_tmp_reg,
            ],
            Opcode.ANA_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.ANI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.ORA_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.ORI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.XRA_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.XRI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.CMP_M: [
                self._ts_exec_set_bus_addr_from_hl_reg,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_mem_and_exec,
            ],
            Opcode.CPI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.ADI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.ACI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.SUI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.SBI: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_read_imm_and_exec,
            ],
            Opcode.INX_BC: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.INX_DE: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.INX_HL: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.INX_SP: [self._ts_exec_internal_delay, self._ts_exec_inx],
            Opcode.DCX_BC: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DCX_DE: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DCX_HL: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DCX_SP: [self._ts_exec_internal_delay, self._ts_exec_dcx],
            Opcode.DAD_BC: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.DAD_DE: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.DAD_HL: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.DAD_SP: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_dad,
            ],
            Opcode.PUSH_BC: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_b,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_c,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.PUSH_DE: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_d,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_e,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.PUSH_HL: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_h,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_l,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.POP_BC: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_c,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_b,
            ],
            Opcode.POP_DE: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_e,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_d,
            ],
            Opcode.POP_HL: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_l,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_h,
            ],
            Opcode.PUSH_PSW: [
                self._ts_exec_internal_delay,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_flag_reg,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.POP_PSW: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_flag_reg,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_a,
            ],
            Opcode.XTHL: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_xthl_read_l,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_sp_plus_1,
                self._ts_exec_set_bus_mr,
                self._ts_exec_xthl_read_h,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_mw,
                self._ts_exec_internal_delay,
            ],
            Opcode.SPHL: [self._ts_exec_internal_delay, self._ts_exec_sphl],
            Opcode.PCHL: [self._ts_exec_internal_delay, self._ts_exec_pchl],
            Opcode.JMP: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_set_pc_from_reg_wz,
            ],
            Opcode.JZ: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JNZ: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JC: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JNC: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JP: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JM: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JPE: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.JPO: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_jump,
            ],
            Opcode.CALL: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 1),
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CNZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CC: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CNC: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CP: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CM: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CPE: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.CPO: [
                self._ts_exec_internal_delay,
                self._ts_exec_internal_delay,
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                self._ts_exec_cond_call,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_high,
                self._ts_exec_set_bus_mw,
                self._ts_exec_push_step,
                self._ts_exec_set_bus_data_from_pc_low,
                self._ts_exec_call_jump,
            ],
            Opcode.RET: [
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RNZ: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RC: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RNC: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RP: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RM: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RPE: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
            Opcode.RPO: [
                self._ts_exec_internal_delay,
                self._ts_exec_cond_ret_check,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_pop_reg_z,
                self._ts_exec_set_bus_addr_from_sp,
                self._ts_exec_set_bus_mr,
                self._ts_exec_ret_jump,
            ],
        }

        rst_steps = [
            self._ts_exec_internal_delay,
            self._ts_exec_internal_delay,
            self._ts_exec_push_step,
            self._ts_exec_set_bus_data_from_pc_high,
            self._ts_exec_set_bus_mw,
            self._ts_exec_push_step,
            self._ts_exec_set_bus_data_from_pc_low,
            self._ts_exec_set_bus_mw,
            self._ts_exec_rst_jump,
        ]
        self._dispatch_table.update({
            Opcode.RST_0: rst_steps,
            Opcode.RST_1: rst_steps,
            Opcode.RST_2: rst_steps,
            Opcode.RST_3: rst_steps,
            Opcode.RST_4: rst_steps,
            Opcode.RST_5: rst_steps,
            Opcode.RST_6: rst_steps,
            Opcode.RST_7: rst_steps,
            Opcode.IN: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_z_port,
                self._ts_exec_set_bus_ior,
                self._ts_exec_in_read_a,
            ],
            Opcode.OUT: [
                self._ts_exec_set_bus_addr_from_pc,
                self._ts_exec_set_bus_mr,
                lambda bus: self._ts_exec_set_reg_val_from_bus_data(bus, 0),
                self._ts_exec_set_bus_addr_from_z_port,
                self._ts_exec_set_bus_data_from_reg_a,
                self._ts_exec_set_bus_iow,
            ],
        })

    @property
    def pair_bc(self) -> Data:
        """Reads register B as High-byte and register C as Low-byte to form a 16-bit word."""
        val_b, val_c = self.reg_b.read(), self.reg_c.read()
        return (val_b << 8) | val_c

    @pair_bc.setter
    def pair_bc(self, value: Data):
        """Splits a 16-bit word and writes the pieces straight into B and C."""
        self.reg_b.write(value >> 8)
        self.reg_c.write(value)

    @property
    def pair_de(self) -> Data:
        """Reads register D as High-byte and register E as Low-byte to form a 16-bit word."""
        val_d, val_e = self.reg_d.read(), self.reg_e.read()
        return (val_d << 8) | val_e

    @pair_de.setter
    def pair_de(self, value: Data):
        """Splits a 16-bit word and writes the pieces straight into D and E."""
        self.reg_d.write(value >> 8)
        self.reg_e.write(value)

    @property
    def pair_hl(self) -> Data:
        """Reads register H as High-byte and register L as Low-byte to form a 16-bit word."""
        val_h, val_l = self.reg_h.read(), self.reg_l.read()
        return (val_h << 8) | val_l

    @pair_hl.setter
    def pair_hl(self, value: Data):
        """Splits a 16-bit word and writes the pieces straight into H and L."""
        self.reg_h.write(value >> 8)
        self.reg_l.write(value)

    def reg(self, name: str) -> Register:
        """Provides the register by name."""
        return getattr(self, f"reg_{name.lower()}")

    @property
    def cycle(self) -> MachineCycle:
        """Current runing machine cycle."""
        return self._cycle

    @property
    def registers(self) -> tuple[Register, ...]:
        """Provides the registers."""
        return (self.reg_a, self.reg_b, self.reg_c, self.reg_d, self.reg_e, self.reg_h, self.reg_l)

    def __repr__(self) -> str:
        return f"CPU({', '.join(repr(reg) for reg in self.registers)})"

    def process(self, bus: SystemBus):
        """Process one t-state for a machine cycle."""
        if bus.reset_in == 1:
            self.reg_pc.write(0x0000)
            self.inte = False
            self.is_halt = False
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
            bus.reset_out = Data.on()
            return
        else:
            bus.reset_out = Data.off()

        if bus.hold == 1:
            bus.hlda = Data.on()
            self._cycle = MachineCycle.HOLD
            bus.mr = Data.off()
            bus.mw = Data.off()
            bus.ior = Data.off()
            bus.iow = Data.off()
            return
        elif self._cycle == MachineCycle.HOLD:
            bus.hlda = Data.off()
            self._cycle = MachineCycle.FETCH
            self.t_state = 1

        if bus.ready == 0:
            return

        if self.t_state == 100:
            bus.address = Mem(getattr(self, '_pending_int_sp', 0))
            bus.data = Data.byte(getattr(self, '_pending_int_low', 0))
            bus.mw = Data.on()
            self.reg_sp.write(getattr(self, '_pending_int_sp', 0))
            self.reg_pc.write(getattr(self, '_pending_int_vector', 0))
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
            return

        if self.is_halt:
            if self.trap or (self.inte and (self.rst_7_5 or self.rst_6_5 or self.rst_5_5 or self.intr)):
                self.is_halt = False

        if self.is_halt:
            return

        if self._cycle == MachineCycle.FETCH and self.t_state <= 1:
            if self._check_hardware_interrupts(bus):
                return

        match self._cycle:
            case MachineCycle.FETCH:
                self._fetch(bus)
            case MachineCycle.EXECUTE:
                self._execute(bus)
            case _:
                pass

    def _check_hardware_interrupts(self, bus: SystemBus) -> bool:
        """Checks and services hardware interrupts based on 8085 priority."""
        if self.trap:
            self.trap = False
            self._trigger_vector_interrupt(bus, VEC_TRAP)
            return True

        if not self.inte:
            return False

        if self.rst_7_5 and not self.mask_7_5:
            self.rst_7_5 = False
            self.pending_7_5 = False
            self._trigger_vector_interrupt(bus, VEC_RST_7_5)
            return True

        if self.rst_6_5 and not self.mask_6_5:
            self.rst_6_5 = False
            self.pending_6_5 = False
            self._trigger_vector_interrupt(bus, VEC_RST_6_5)
            return True

        if self.rst_5_5 and not self.mask_5_5:
            self.rst_5_5 = False
            self.pending_5_5 = False
            self._trigger_vector_interrupt(bus, VEC_RST_5_5)
            return True

        if self.intr:
            self.intr = False
            self.inte = False
            self._is_inta_cycle = True
            return False

        return False

    def _trigger_vector_interrupt(self, bus: SystemBus, vector_addr: int):
        """Pushes current PC and jumps to fixed interrupt vector address."""
        self.inte = False
        sp_val = self.reg_sp.read().value
        pc_val = self.reg_pc.read().value

        high_byte = (pc_val >> 8) & 0xFF
        low_byte = pc_val & 0xFF

        sp_val = (sp_val - 1) & 0xFFFF
        bus.address = Mem(sp_val)
        bus.data = Data.byte(high_byte)
        bus.mw = Data.on()

        self._pending_int_sp = (sp_val - 1) & 0xFFFF
        self._pending_int_low = low_byte
        self._pending_int_vector = vector_addr
        self.t_state = 100

    def _fetch(self, bus: SystemBus):
        """Exectues one t-state fetch machine cycle."""
        if self.t_state == 0:
            self.t_state += 1

        if self.t_state == 1:
            bus.reset()
            if getattr(self, "_is_inta_cycle", False):
                bus.inta = Data.on()
            else:
                bus.address = Mem(self.reg_pc.read().value)
                self.reg_pc.increment()
            self.t_state += 1
        elif self.t_state == 2:
            if not getattr(self, "_is_inta_cycle", False):
                bus.mr = Data.on()
            self.t_state += 1
        elif self.t_state == 3:
            self.ireg.write(Opcode(bus.data.value))
            if getattr(self, "_is_inta_cycle", False):
                bus.inta = Data.off()
                self._is_inta_cycle = False
            else:
                bus.mr = Data.off()
            self.t_state += 1
        elif self.t_state == 4:
            self._decode(bus)

    def _decode(self, bus: SystemBus):
        """Executes one t-state decode machine cycle."""
        if self.t_state == 4:
            opcode = Opcode(self.ireg.read().value)

            if opcode == Opcode.HLT:
                self.is_halt = True
                self.t_state = 0
                return

            decoder_fn = self._decoder_matrix.get(opcode.value)
            if decoder_fn:
                self._cycle = decoder_fn(bus)
            else:
                self._cycle = MachineCycle.FETCH

            self.t_state = 1

    def _execute(self, bus: SystemBus):
        """Executes one t-state execute machine cycle."""
        steps = self._dispatch_table.get(self.ireg.read().value)
        if not steps:
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
            return

        step_index = self.t_state - 1
        if step_index < len(steps):
            steps[step_index](bus)

        if (self.t_state - 1) >= len(steps):
            self._cycle = MachineCycle.FETCH
            self.t_state = 1

    def _ts_exec_set_bus_addr_from_pc(self, bus: SystemBus):
        """Sets the address on system bus from program counter."""
        bus.address = Mem(self.reg_pc.read().value)
        self.reg_pc.increment()
        self.t_state += 1

    def _ts_exec_set_bus_mr(self, bus: SystemBus):
        """Enables the memory read (MR) signal on system bus."""
        bus.mr = Data.on()
        self.t_state += 1

    def _ts_exec_set_bus_mw(self, bus: SystemBus):
        """Enables the memory write (MW) signal on system bus."""
        bus.mw = Data.on()
        self.t_state += 1

    def _ts_exec_set_reg_val_from_bus_data(self, bus: SystemBus, order: Literal[0, 1, 2, 3]):
        """
        Sets the register in the instruction with immediate value and disabled memory read
        signal.
        """
        self._reg_dst.write_byte(bus.data, order)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_reg_a_val_from_bus_data(self, bus: SystemBus):
        """Sets the register a in the bus data value."""
        self.reg_a.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_reg_l_val_from_bus_data(self, bus: SystemBus):
        """Sets register L from bus data value."""
        self.reg_l.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_reg_h_val_from_bus_data(self, bus: SystemBus):
        """Sets register H from bus data value."""
        self.reg_h.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_wz_plus_1(self, bus: SystemBus):
        """Sets bus address from WZ + 1."""
        addr = Data.words(self.reg_w.read().value, self.reg_z.read().value).value + 1
        bus.address = Mem(addr)
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_l(self, bus: SystemBus):
        """Sets bus data from register L."""
        bus.data = self.reg_l.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_h(self, bus: SystemBus):
        """Sets bus data from register H."""
        bus.data = self.reg_h.read()
        self.t_state += 1

    def _ts_exec_push_step(self, bus: SystemBus):
        """Disables MW signal, decrements SP by 1, and sets bus address from SP."""
        bus.mw = Data.off()
        self.reg_sp.decrement()
        sp_val = self.reg_sp.read().value
        bus.address = Mem(sp_val)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_sp(self, bus: SystemBus):
        """Sets bus address from SP."""
        bus.address = Mem(self.reg_sp.read().value)
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_b(self, bus: SystemBus):
        """Sets bus data from register B."""
        bus.data = self.reg_b.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_c(self, bus: SystemBus):
        """Sets bus data from register C."""
        bus.data = self.reg_c.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_d(self, bus: SystemBus):
        """Sets bus data from register D."""
        bus.data = self.reg_d.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_e(self, bus: SystemBus):
        """Sets bus data from register E."""
        bus.data = self.reg_e.read()
        self.t_state += 1

    def _ts_exec_pop_reg_c(self, bus: SystemBus):
        """Writes bus data to C and increments SP by 1."""
        self.reg_c.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_b(self, bus: SystemBus):
        """Writes bus data to B and increments SP by 1."""
        self.reg_b.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_e(self, bus: SystemBus):
        """Writes bus data to E and increments SP by 1."""
        self.reg_e.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_d(self, bus: SystemBus):
        """Writes bus data to D and increments SP by 1."""
        self.reg_d.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_l(self, bus: SystemBus):
        """Writes bus data to L and increments SP by 1."""
        self.reg_l.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_h(self, bus: SystemBus):
        """Writes bus data to H and increments SP by 1."""
        self.reg_h.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_flag_reg(self, bus: SystemBus):
        """Sets bus data from flag register."""
        bus.data = Data(self.flag_reg.value, size=DataSize.BYTE)
        self.t_state += 1

    def _ts_exec_pop_flag_reg(self, bus: SystemBus):
        """Writes bus data to flag register and increments SP by 1."""
        self.flag_reg.value = bus.data
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_a(self, bus: SystemBus):
        """Writes bus data to register A and increments SP by 1."""
        self.reg_a.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_pop_reg_z(self, bus: SystemBus):
        """Writes bus data to register Z and increments SP by 1."""
        self.reg_z.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_ret_jump(self, bus: SystemBus):
        """Writes bus data to register W, increments SP by 1, and sets PC = WZ."""
        self.reg_w.write(bus.data)
        bus.mr = Data.off()
        self.reg_sp.increment()
        self.reg_pc.write(self._reg_dst.read_word())
        self.t_state += 1

    def _ts_exec_ei(self, bus: SystemBus):
        """Enables interrupts (sets INTE = True)."""
        self.inte = True
        self.t_state += 1

    def _ts_exec_di(self, bus: SystemBus):
        """Disables interrupts (clears INTE = False)."""
        self.inte = False
        self.t_state += 1

    def _ts_exec_rim(self, bus: SystemBus):
        """Reads interrupt mask, pending flags, INTE status, and SID bit into Accumulator A."""
        val = 0
        if self.mask_5_5:
            val |= (1 << 0)
        if self.mask_6_5:
            val |= (1 << 1)
        if self.mask_7_5:
            val |= (1 << 2)
        if self.inte:
            val |= (1 << 3)
        if self.pending_5_5:
            val |= (1 << 4)
        if self.pending_6_5:
            val |= (1 << 5)
        if self.pending_7_5:
            val |= (1 << 6)
        if self.sid:
            val |= (1 << 7)

        self.reg_a.write(Data.byte(val))
        self.t_state += 1

    def _ts_exec_sim(self, bus: SystemBus):
        """Sets interrupt masks, clears RST 7.5 latch, and updates SOD pin from Accumulator A."""
        val = self.reg_a.read().value
        mse = bool(val & (1 << 3))
        if mse:
            self.mask_5_5 = bool(val & (1 << 0))
            self.mask_6_5 = bool(val & (1 << 1))
            self.mask_7_5 = bool(val & (1 << 2))

        r7_5 = bool(val & (1 << 4))
        if r7_5:
            self.pending_7_5 = False

        sde = bool(val & (1 << 6))
        if sde:
            self.sod = bool(val & (1 << 7))

        self.t_state += 1

    def _ts_exec_set_bus_addr_from_z_port(self, bus: SystemBus):
        """Sets bus address from register Z for I/O port operation."""
        bus.mr = Data.off()
        port_val = self.reg_z.read().value & 0xFF
        bus.address = Mem((port_val << 8) | port_val)
        self.t_state += 1

    def _ts_exec_set_bus_ior(self, bus: SystemBus):
        """Enables I/O Read (IOR) signal on system bus."""
        bus.ior = Data.on()
        self.t_state += 1

    def _ts_exec_in_read_a(self, bus: SystemBus):
        """Reads bus data into Accumulator A and disables IOR."""
        self.reg_a.write(bus.data)
        bus.ior = Data.off()
        self._cycle = MachineCycle.FETCH
        self.t_state = 1

    def _ts_exec_set_bus_iow(self, bus: SystemBus):
        """Enables I/O Write (IOW) signal on system bus and finishes cycle."""
        bus.iow = Data.on()
        self._cycle = MachineCycle.FETCH
        self.t_state = 1

    def _ts_exec_set_bus_data_from_pc_high(self, bus: SystemBus):
        """Sets bus data from PC High byte."""
        bus.data = Data.byte(self.reg_pc.read().byte_at(1))
        self.t_state += 1

    def _ts_exec_set_bus_data_from_pc_low(self, bus: SystemBus):
        """Sets bus data from PC Low byte."""
        bus.data = Data.byte(self.reg_pc.read().byte_at(0))
        self.t_state += 1

    def _ts_exec_call_jump(self, bus: SystemBus):
        """Enables memory write for return address low byte and jumps to WZ."""
        bus.mw = Data.on()
        self.reg_pc.write(self._reg_dst.read_word())
        self.t_state += 1

    def _ts_exec_rst_jump(self, bus: SystemBus):
        """Sets PC to the restart vector address (8 * n) based on RST opcode."""
        bus.mw = Data.off()
        opcode_val = self.ireg.read().value
        vector_num = (opcode_val >> 3) & 0x07
        vectors = (VEC_RST_0, VEC_RST_1, VEC_RST_2, VEC_RST_3, VEC_RST_4, VEC_RST_5, VEC_RST_6, VEC_RST_7)
        self.reg_pc.write(vectors[vector_num])
        self._cycle = MachineCycle.FETCH
        self.t_state = 1

    def _ts_exec_xthl_read_l(self, bus: SystemBus):
        """Swaps L with bus data and puts old L on bus data for memory write."""
        old_l = self.reg_l.read()
        self.reg_l.write(bus.data)
        bus.mr = Data.off()
        bus.data = old_l
        self.t_state += 1

    def _ts_exec_xthl_read_h(self, bus: SystemBus):
        """Swaps H with bus data and puts old H on bus data for memory write."""
        old_h = self.reg_h.read()
        self.reg_h.write(bus.data)
        bus.mr = Data.off()
        bus.data = old_h
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_sp_plus_1(self, bus: SystemBus):
        """Sets bus address from SP + 1."""
        bus.mw = Data.off()
        bus.address = Mem((self.reg_sp.read().value + 1) & 0xFFFF)
        self.t_state += 1

    def _ts_exec_sphl(self, bus: SystemBus):
        """Loads stack pointer SP from register pair HL."""
        self.reg_sp.write(self.pair_hl.value)
        self.t_state += 1

    def _ts_exec_pchl(self, bus: SystemBus):
        """Loads program counter PC from register pair HL."""
        self.reg_pc.write(self.pair_hl.value)
        self.t_state += 1

    def _ts_exec_nop(self, bus: SystemBus):
        """No operation."""
        self.t_state += 1

    def _ts_exec_xchg(self, bus: SystemBus):
        """Exchanges contents of DE and HL register pairs."""
        val_d = self.reg_d.read().value
        val_e = self.reg_e.read().value
        val_h = self.reg_h.read().value
        val_l = self.reg_l.read().value

        self.reg_d.write(val_h)
        self.reg_e.write(val_l)
        self.reg_h.write(val_d)
        self.reg_l.write(val_e)

        self.t_state += 1

    def _ts_exec_set_bus_addr_from_hl_reg(self, bus: SystemBus):
        """Sets bus address from HL register pair."""
        bus.address = Mem(Data.words(
            self.reg_h.read().value,
            self.reg_l.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_bc_reg(self, bus: SystemBus):
        """Sets bus address from BC register pair."""
        bus.address = Mem(Data.words(
            self.reg_b.read().value,
            self.reg_c.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_de_reg(self, bus: SystemBus):
        """Sets bus address from DE register pair."""
        bus.address = Mem(Data.words(
            self.reg_d.read().value,
            self.reg_e.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_addr_from_wz_reg(self, bus: SystemBus):
        """Sets bus address from WZ register pair."""
        bus.address = Mem(Data.words(
            self.reg_w.read().value,
            self.reg_z.read().value,
        ).value)
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg_a(self, bus: SystemBus):
        """Sets bus data from register A."""
        bus.data = self.reg_a.read()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_reg(self, bus: SystemBus):
        """Sets bus data from register."""
        bus.data = self._reg_src.read_byte(order=0)
        self.t_state += 1

    def _ts_exce_set_reg_from_reg(self, bus: SystemBus):
        """Sets register from another register."""
        data = self._reg_src.read_byte(order=0)
        self._reg_dst.write_byte(data, order=0)
        self.t_state += 1

    def _ts_exec_set_tmp_reg_val_from_bus_data(self, bus: SystemBus):
        """Sets temp register value from bus data."""
        self.reg_tmp.write(bus.data)
        bus.mr = Data.off()
        self.t_state += 1

    def _ts_exec_set_bus_data_from_tmp_reg(self, bus: SystemBus):
        """Sets bus data from temp register value."""
        bus.data = self.reg_tmp.read()
        self.t_state += 1

    def _ts_exec_internal_delay(self, bus: SystemBus):
        """Internal operation cycle / bus idle state."""
        self.t_state += 1

    def _ts_exec_read_imm_and_exec(self, bus: SystemBus):
        """Reads immediate data byte from bus and executes arithmetic operation."""
        self.reg_tmp.write(bus.data)
        bus.mr = Data.off()
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.ADI:
            self._ts_exec_add(bus)
        elif opcode == Opcode.ACI:
            self._ts_exec_add_with_carry(bus)
        elif opcode == Opcode.SUI:
            self._ts_exec_sub(bus)
        elif opcode == Opcode.SBI:
            self._ts_exec_sub_with_borrow(bus)
        elif opcode == Opcode.ANI:
            self._ts_exec_ana(bus)
        elif opcode == Opcode.ORI:
            self._ts_exec_ora(bus)
        elif opcode == Opcode.XRI:
            self._ts_exec_xra(bus)
        elif opcode == Opcode.CPI:
            self._ts_exec_cmp(bus)

    def _ts_exec_read_mem_and_exec(self, bus: SystemBus):
        """Reads memory byte from bus and executes arithmetic operation."""
        self.reg_tmp.write(bus.data)
        bus.mr = Data.off()
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.ADD_M:
            self._ts_exec_add(bus)
        elif opcode == Opcode.ADC_M:
            self._ts_exec_add_with_carry(bus)
        elif opcode == Opcode.SUB_M:
            self._ts_exec_sub(bus)
        elif opcode == Opcode.SBB_M:
            self._ts_exec_sub_with_borrow(bus)
        elif opcode == Opcode.ANA_M:
            self._ts_exec_ana(bus)
        elif opcode == Opcode.ORA_M:
            self._ts_exec_ora(bus)
        elif opcode == Opcode.XRA_M:
            self._ts_exec_xra(bus)
        elif opcode == Opcode.CMP_M:
            self._ts_exec_cmp(bus)

    def _ts_exec_add(self, bus: SystemBus):
        """Adds the register in insturction with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ADD_M, Opcode.ADI):
            self.reg_tmp.write(self._reg_src.read_byte().value)
        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        res = val1 + val2
        res8 = res & 0xFF
        res4 = (val1 & 0x0F) + (val2 & 0x0F)

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = (res >> 8) & 1
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = (res4 >> 4) & 1
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_add_with_carry(self, bus: SystemBus):
        """Adds the register in instruction and Carry flag with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ADC_M, Opcode.ACI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        c_in = self.flag_reg.carry
        self.reg_tmp.write(val1)

        res = val1 + val2 + c_in
        res8 = res & 0xFF
        res4 = (val1 & 0x0F) + (val2 & 0x0F) + c_in

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = (res >> 8) & 1
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = (res4 >> 4) & 1
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_sub(self, bus: SystemBus):
        """Subtracts the register/temp value from register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.SUB_M, Opcode.SUI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        res = val2 - val1
        res8 = res & 0xFF
        res4 = (val2 & 0x0F) - (val1 & 0x0F)

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = 1 if res < 0 else 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_sub_with_borrow(self, bus: SystemBus):
        """Subtracts the register/temp value and Carry flag from register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.SBB_M, Opcode.SBI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        c_in = self.flag_reg.carry

        res = val2 - val1 - c_in
        res8 = res & 0xFF
        res4 = (val2 & 0x0F) - (val1 & 0x0F) - c_in

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res8)
        self.flag_reg.carry = 1 if res < 0 else 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_inr(self, bus: SystemBus):
        """Increments the register/temp value by 1 and updates status flags."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.INR_M:
            val = self.reg_tmp.read().value
        else:
            val = self._reg_dst.read_byte().value

        res = (val + 1) & 0xFF
        res4 = (val & 0x0F) + 1

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        if opcode == Opcode.INR_M:
            self.reg_tmp.write(res)
        else:
            self._reg_dst.write_byte(res)

        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = (res4 >> 4) & 1
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_dcr(self, bus: SystemBus):
        """Decrements the register/temp value by 1 and updates status flags."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.DCR_M:
            val = self.reg_tmp.read().value
        else:
            val = self._reg_dst.read_byte().value

        res = (val - 1) & 0xFF
        res4 = (val & 0x0F) - 1

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        if opcode == Opcode.DCR_M:
            self.reg_tmp.write(res)
        else:
            self._reg_dst.write_byte(res)

        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_ana(self, bus: SystemBus):
        """Performs logical AND of register/temp value with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ANA_M, Opcode.ANI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        res = val1 & val2

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_ora(self, bus: SystemBus):
        """Performs logical OR of register/temp value with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.ORA_M, Opcode.ORI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        res = val1 | val2

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 0
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_xra(self, bus: SystemBus):
        """Performs logical XOR of register/temp value with register A and writes it to register A."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.XRA_M, Opcode.XRI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value
        res = val1 ^ val2

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 0
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_cmp(self, bus: SystemBus):
        """Compares register/temp value with register A and updates status flags."""
        opcode = Opcode(self.ireg.read().value)
        if opcode not in (Opcode.CMP_M, Opcode.CPI):
            self.reg_tmp.write(self._reg_src.read_byte().value)

        val1 = self.reg_tmp.read().value
        val2 = self.reg_a.read().value

        res = val2 - val1
        res8 = res & 0xFF
        res4 = (val2 & 0x0F) - (val1 & 0x0F)

        p = res8
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.flag_reg.carry = 1 if res < 0 else 0
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = 1 if res4 < 0 else 0
        self.flag_reg.zero = 1 if res8 == 0 else 0
        self.flag_reg.sign = (res8 >> 7) & 1
        self.t_state += 1

    def _ts_exec_rlc(self, bus: SystemBus):
        """Rotates register A left circular; updates Carry flag."""
        val = self.reg_a.read().value
        bit7 = (val >> 7) & 1
        res = ((val << 1) & 0xFE) | bit7

        self.reg_a.write(res)
        self.flag_reg.carry = bit7
        self.t_state += 1

    def _ts_exec_rrc(self, bus: SystemBus):
        """Rotates register A right circular; updates Carry flag."""
        val = self.reg_a.read().value
        bit0 = val & 1
        res = ((val >> 1) & 0x7F) | (bit0 << 7)

        self.reg_a.write(res)
        self.flag_reg.carry = bit0
        self.t_state += 1

    def _ts_exec_ral(self, bus: SystemBus):
        """Rotates register A left through Carry flag; updates Carry flag."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        bit7 = (val >> 7) & 1
        res = ((val << 1) & 0xFE) | c_in

        self.reg_a.write(res)
        self.flag_reg.carry = bit7
        self.t_state += 1

    def _ts_exec_rar(self, bus: SystemBus):
        """Rotates register A right through Carry flag; updates Carry flag."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        bit0 = val & 1
        res = ((val >> 1) & 0x7F) | (c_in << 7)

        self.reg_a.write(res)
        self.flag_reg.carry = bit0
        self.t_state += 1

    def _ts_exec_cma(self, bus: SystemBus):
        """Complements register A value; status flags are unchanged."""
        val = self.reg_a.read().value
        self.reg_a.write((~val) & 0xFF)
        self.t_state += 1

    def _ts_exec_cmc(self, bus: SystemBus):
        """Complements Carry flag; updates Carry flag."""
        self.flag_reg.carry = (~self.flag_reg.carry) & 1
        self.t_state += 1

    def _ts_exec_stc(self, bus: SystemBus):
        """Sets Carry flag to 1; updates Carry flag."""
        self.flag_reg.carry = 1
        self.t_state += 1

    def _ts_exec_daa(self, bus: SystemBus):
        """Decimal adjusts register A value after addition; updates status flags."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        ac_in = self.flag_reg.aux

        inc = 0
        carry = c_in
        aux = 0

        if (val & 0x0F) > 9 or ac_in == 1:
            inc += 0x06
            aux = 1

        if val > 0x99 or c_in == 1:
            inc += 0x60
            carry = 1

        res = (val + inc) & 0xFF

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_das(self, bus: SystemBus):
        """Decimal adjusts register A value after subtraction; updates status flags."""
        val = self.reg_a.read().value
        c_in = self.flag_reg.carry
        ac_in = self.flag_reg.aux

        res = val
        carry = c_in
        aux = 0

        if (val & 0x0F) > 9 or ac_in == 1:
            res = (res - 0x06) & 0xFF
            aux = 1

        if val > 0x99 or c_in == 1:
            res = (res - 0x60) & 0xFF
            carry = 1

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_aaa(self, bus: SystemBus):
        """ASCII adjusts register A value after addition; updates status flags."""
        val = self.reg_a.read().value
        ac_in = self.flag_reg.aux

        if (val & 0x0F) > 9 or ac_in == 1:
            res = (val + 0x06) & 0x0F
            carry = 1
            aux = 1
        else:
            res = val & 0x0F
            carry = 0
            aux = 0

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_aas(self, bus: SystemBus):
        """ASCII adjusts register A value after subtraction; updates status flags."""
        val = self.reg_a.read().value
        ac_in = self.flag_reg.aux

        if (val & 0x0F) > 9 or ac_in == 1:
            res = (val - 0x06) & 0x0F
            carry = 1
            aux = 1
        else:
            res = val & 0x0F
            carry = 0
            aux = 0

        p = res
        p ^= p >> 4
        p ^= p >> 2
        p ^= p >> 1

        self.reg_a.write(res)
        self.flag_reg.carry = carry
        self.flag_reg.parity = (~p) & 1
        self.flag_reg.aux = aux
        self.flag_reg.zero = 1 if res == 0 else 0
        self.flag_reg.sign = (res >> 7) & 1
        self.t_state += 1

    def _ts_exec_inx(self, bus: SystemBus):
        """Increments 16-bit register pair by 1; status flags are unchanged."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.INX_BC:
            self.pair_bc = Data.word(self.pair_bc.value + 1)
        elif opcode == Opcode.INX_DE:
            self.pair_de = Data.word(self.pair_de.value + 1)
        elif opcode == Opcode.INX_HL:
            self.pair_hl = Data.word(self.pair_hl.value + 1)
        elif opcode == Opcode.INX_SP:
            self.reg_sp.increment()
        self.t_state += 1

    def _ts_exec_dcx(self, bus: SystemBus):
        """Decrements 16-bit register pair by 1; status flags are unchanged."""
        opcode = Opcode(self.ireg.read().value)
        if opcode == Opcode.DCX_BC:
            self.pair_bc = Data.word(self.pair_bc.value - 1)
        elif opcode == Opcode.DCX_DE:
            self.pair_de = Data.word(self.pair_de.value - 1)
        elif opcode == Opcode.DCX_HL:
            self.pair_hl = Data.word(self.pair_hl.value - 1)
        elif opcode == Opcode.DCX_SP:
            self.reg_sp.decrement()
        self.t_state += 1

    def _ts_exec_dad(self, bus: SystemBus):
        """Adds 16-bit register pair to HL pair; updates Carry flag only."""
        opcode = Opcode(self.ireg.read().value)
        val_hl = self.pair_hl.value

        if opcode == Opcode.DAD_BC:
            val_rp = self.pair_bc.value
        elif opcode == Opcode.DAD_DE:
            val_rp = self.pair_de.value
        elif opcode == Opcode.DAD_HL:
            val_rp = val_hl
        elif opcode == Opcode.DAD_SP:
            val_rp = self.reg_sp.read().value
        else:
            val_rp = 0

        res = val_hl + val_rp
        self.pair_hl = Data.word(res)
        self.flag_reg.carry = (res >> 16) & 1
        self.t_state += 1

    def _ts_exec_set_pc_from_reg_wz(self, bus: SystemBus):
        """Sets program counter register with address in WZ register pair."""
        self._reg_dst.write_byte(bus.data, 1)
        bus.mr = Data.off()
        self.reg_pc.write(self._reg_dst.read_word())
        self.t_state += 1

    def _ts_exec_cond_jump(self, bus: SystemBus):
        """Reads High byte into W, evaluates opcode jump condition, and updates PC if condition is met."""
        self._reg_dst.write_byte(bus.data, 1)
        bus.mr = Data.off()

        opcode = Opcode(self.ireg.read().value)
        jump = False
        if opcode == Opcode.JZ and self.flag_reg.zero == 1:
            jump = True
        elif opcode == Opcode.JNZ and self.flag_reg.zero == 0:
            jump = True
        elif opcode == Opcode.JC and self.flag_reg.carry == 1:
            jump = True
        elif opcode == Opcode.JNC and self.flag_reg.carry == 0:
            jump = True
        elif opcode == Opcode.JP and self.flag_reg.sign == 0:
            jump = True
        elif opcode == Opcode.JM and self.flag_reg.sign == 1:
            jump = True
        elif opcode == Opcode.JPE and self.flag_reg.parity == 1:
            jump = True
        elif opcode == Opcode.JPO and self.flag_reg.parity == 0:
            jump = True

        if jump:
            self.reg_pc.write(self._reg_dst.read_word())

        self.t_state += 1

    def _ts_exec_cond_call(self, bus: SystemBus):
        """
        Reads High byte into W, evaluates opcode call condition. If met, continues
        subroutine call; else ends cycle.
        """
        self._reg_dst.write_byte(bus.data, 1)
        bus.mr = Data.off()

        opcode = Opcode(self.ireg.read().value)
        call = False
        if opcode == Opcode.CZ and self.flag_reg.zero == 1:
            call = True
        elif opcode == Opcode.CNZ and self.flag_reg.zero == 0:
            call = True
        elif opcode == Opcode.CC and self.flag_reg.carry == 1:
            call = True
        elif opcode == Opcode.CNC and self.flag_reg.carry == 0:
            call = True
        elif opcode == Opcode.CP and self.flag_reg.sign == 0:
            call = True
        elif opcode == Opcode.CM and self.flag_reg.sign == 1:
            call = True
        elif opcode == Opcode.CPE and self.flag_reg.parity == 1:
            call = True
        elif opcode == Opcode.CPO and self.flag_reg.parity == 0:
            call = True

        if call:
            self.t_state += 1
        else:
            self._cycle = MachineCycle.FETCH
            self.t_state = 1

    def _ts_exec_cond_ret_check(self, bus: SystemBus):
        """Evaluates opcode return condition. If met, continues return sequence; else ends cycle."""
        opcode = Opcode(self.ireg.read().value)
        ret = False
        if opcode == Opcode.RZ and self.flag_reg.zero == 1:
            ret = True
        elif opcode == Opcode.RNZ and self.flag_reg.zero == 0:
            ret = True
        elif opcode == Opcode.RC and self.flag_reg.carry == 1:
            ret = True
        elif opcode == Opcode.RNC and self.flag_reg.carry == 0:
            ret = True
        elif opcode == Opcode.RP and self.flag_reg.sign == 0:
            ret = True
        elif opcode == Opcode.RM and self.flag_reg.sign == 1:
            ret = True
        elif opcode == Opcode.RPE and self.flag_reg.parity == 1:
            ret = True
        elif opcode == Opcode.RPO and self.flag_reg.parity == 0:
            ret = True

        if ret:
            self.t_state += 1
        else:
            self._cycle = MachineCycle.FETCH
            self.t_state = 1
