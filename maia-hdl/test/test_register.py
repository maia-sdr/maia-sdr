#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *

import unittest

from maia_hdl.register import Access, Field, Register, Registers
from .amaranth_sim import AmaranthSim


class TestRegister(AmaranthSim):
    def setUp(self):
        self.dut = m = Module()
        self.register = Register(
            'dut_register',
            [Field('rw_field', Access.RW, 7, 42),
             Field('r_field', Access.R, 5, 0),
             Field('w_field', Access.W, 12, 1873),
             Field('wpulse_field', Access.Wpulse, 1, 0),
             Field('rsticky_field', Access.Rsticky, 1, 0)],
            interrupt=True)
        self.readable = Signal(5, reset=17)
        self.interrupt_enable = Signal()
        m.submodules.register = self.register
        m.d.comb += [
            self.register['r_field'].eq(self.readable),
            self.register['rsticky_field'].eq(self.interrupt_enable),
        ]

    def test_initial_value(self):
        def bench():
            yield self.register.ren.eq(1)
            yield
            read = yield self.register.rdata
            assert read & (2**7 - 1) == 42
            assert (read >> 7) & (2**5 - 1) == 17
            assert (read >> (7 + 5)) & (2**12 - 1) == 0
            assert read >> (7 + 5 + 12) == 0
            yield self.register.ren.eq(0)
            yield
            assert (yield self.register.rdata) == 0

        self.simulate(bench)

    def test_write(self):
        def bench():
            yield self.register.wstrobe.eq(0xf)
            value = 0xdeadbeef
            yield self.register.wdata.eq(value)
            yield
            yield self.register.wstrobe.eq(0)
            yield
            rw_field = yield self.register['rw_field']
            assert rw_field == value & (2**7 - 1)
            r_field = yield self.readable
            assert r_field == 17
            w_field = yield self.register['w_field']
            assert w_field == (value >> (7 + 5)) & (2**12 - 1)
            assert (yield self.register.rdata) == 0
            yield self.register.ren.eq(1)
            yield
            yield
            read = yield self.register.rdata
            assert read == (value & (2**7 - 1)) | (17 << 7)

        self.simulate(bench)

    def test_pulse(self):
        def bench():
            reg = self.register['wpulse_field']
            yield self.register.wstrobe.eq(0xf)
            value = 1 << (7 + 5 + 12)
            yield self.register.wdata.eq(value)
            assert not (yield reg)
            yield
            yield self.register.wstrobe.eq(0)
            assert not (yield reg)
            yield
            assert (yield reg)
            for _ in range(10):
                yield
                assert not (yield reg)

        self.simulate(bench)

    def test_interrupt(self):
        def bench():
            for _ in range(4):
                assert not (yield self.register.interrupt)
                yield
            yield self.interrupt_enable.eq(1)
            assert not (yield self.register.interrupt)
            yield
            yield self.interrupt_enable.eq(0)
            assert not (yield self.register.interrupt)
            yield
            assert not (yield self.register.interrupt)
            for _ in range(10):
                yield
                assert (yield self.register.interrupt)
            yield self.register.ren.eq(1)
            intr_bit = 7 + 5 + 12 + 1
            yield
            assert (yield self.register.interrupt)
            assert (yield self.register.rdata[intr_bit])
            yield self.register.ren.eq(0)
            yield
            assert (yield self.register.interrupt)
            yield
            for _ in range(6):
                assert not (yield self.register.interrupt)
                yield
                assert not (yield self.register.interrupt)
            yield self.interrupt_enable.eq(1)
            yield
            assert not (yield self.register.interrupt)
            yield self.interrupt_enable.eq(0)
            yield self.register.ren.eq(1)
            yield
            assert not (yield self.register.interrupt)
            assert (yield self.register.rdata[intr_bit])
            yield self.register.ren.eq(0)
            yield
            assert (yield self.register.interrupt)
            yield
            assert not (yield self.register.interrupt)
            yield
            assert not (yield self.register.interrupt)
            yield self.interrupt_enable.eq(1)
            yield self.register.ren.eq(1)
            yield
            assert not (yield self.register.rdata[intr_bit])
            yield self.interrupt_enable.eq(0)
            yield self.register.ren.eq(0)
            yield
            assert not (yield self.register.interrupt)
            yield
            assert (yield self.register.interrupt)
            yield
            assert (yield self.register.interrupt)
            yield self.interrupt_enable.eq(1)
            yield self.register.ren.eq(1)
            yield
            assert (yield self.register.interrupt)
            yield self.interrupt_enable.eq(0)
            assert (yield self.register.rdata[intr_bit])
            yield self.register.ren.eq(0)
            yield
            assert (yield self.register.interrupt)
            yield self.register.ren.eq(1)
            yield
            assert (yield self.register.interrupt)
            assert (yield self.register.rdata[intr_bit])
            yield self.register.ren.eq(0)
            yield
            assert (yield self.register.interrupt)
            yield
            assert not (yield self.register.interrupt)

        self.simulate(bench, 'interrupt.vcd')


class TestRegisters(AmaranthSim):
    def setUp(self):
        self.dut = Registers(
            'registers',
            {0b00: Register(
                'rega', [Field('field0', Access.RW, 32, 12345)]),
             0b10: Register(
                 'regb', [Field('field1', Access.RW, 32, 6789)]),
             },
            2)

    def test_registers(self):
        def bench():
            yield self.dut.address.eq(0b10)
            yield self.dut.ren.eq(1)
            yield
            yield self.dut.ren.eq(0)
            yield
            assert (yield self.dut.rdone) == 1
            assert (yield self.dut.rdata) == 6789
            assert (yield self.dut.wdone) == 0
            yield
            assert (yield self.dut.rdone) == 0
            assert (yield self.dut.rdata) == 0
            assert (yield self.dut.wdone) == 0
            yield self.dut.address.eq(0b00)
            yield self.dut.ren.eq(1)
            yield
            yield self.dut.ren.eq(0)
            yield
            assert (yield self.dut.rdone) == 1
            assert (yield self.dut.rdata) == 12345
            assert (yield self.dut.wdone) == 0
            yield self.dut.address.eq(0b10)
            yield self.dut.wstrobe.eq(0x8)
            yield self.dut.wdata.eq(0xffffffff)
            yield
            yield self.dut.wstrobe.eq(0)
            yield
            assert (yield self.dut.rdone) == 0
            assert (yield self.dut.rdata) == 0
            assert (yield self.dut.wdone) == 1
            yield self.dut.ren.eq(1)
            yield
            yield self.dut.ren.eq(0)
            yield
            assert (yield self.dut.rdone) == 1
            assert (yield self.dut.rdata) == (0xff << 24) | 6789
            assert (yield self.dut.wdone) == 0
            yield
            yield self.dut.address.eq(0b01)
            yield self.dut.ren.eq(1)
            yield
            yield self.dut.ren.eq(0)
            yield
            assert (yield self.dut.rdone) == 1
            assert (yield self.dut.rdata) == 0
            assert (yield self.dut.wdone) == 0
            yield self.dut.wstrobe.eq(0xf)
            yield
            yield self.dut.wstrobe.eq(0)
            yield
            assert (yield self.dut.rdone) == 0
            assert (yield self.dut.rdata) == 0
            assert (yield self.dut.wdone) == 1

        self.simulate(bench)


if __name__ == '__main__':
    unittest.main()
