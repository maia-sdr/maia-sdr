#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
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
        self.readable = Signal(5, init=17)
        self.interrupt_enable = Signal()
        m.submodules.register = self.register
        m.d.comb += [
            self.register['r_field'].eq(self.readable),
            self.register['rsticky_field'].eq(self.interrupt_enable),
        ]

    def test_initial_value(self):
        async def bench(ctx):
            ctx.set(self.register.ren, 1)
            await ctx.tick()
            read = ctx.get(self.register.rdata)
            assert read & (2**7 - 1) == 42
            assert (read >> 7) & (2**5 - 1) == 17
            assert (read >> (7 + 5)) & (2**12 - 1) == 0
            assert read >> (7 + 5 + 12) == 0
            ctx.set(self.register.ren, 0)
            await ctx.tick()
            assert ctx.get(self.register.rdata) == 0

        self.simulate(bench)

    def test_write(self):
        async def bench(ctx):
            ctx.set(self.register.wstrobe, 0xf)
            value = 0xdeadbeef
            ctx.set(self.register.wdata, value)
            await ctx.tick()
            ctx.set(self.register.wstrobe, 0)
            await ctx.tick()
            rw_field = ctx.get(self.register['rw_field'])
            assert rw_field == value & (2**7 - 1)
            r_field = ctx.get(self.readable)
            assert r_field == 17
            w_field = ctx.get(self.register['w_field'])
            assert w_field == (value >> (7 + 5)) & (2**12 - 1)
            assert ctx.get(self.register.rdata) == 0
            ctx.set(self.register.ren, 1)
            await ctx.tick()
            read = ctx.get(self.register.rdata)
            assert read == (value & (2**7 - 1)) | (17 << 7)

        self.simulate(bench)

    def test_pulse(self):
        async def bench(ctx):
            await ctx.tick()
            reg = self.register['wpulse_field']
            ctx.set(self.register.wstrobe, 0xf)
            value = 1 << (7 + 5 + 12)
            ctx.set(self.register.wdata, value)
            assert not ctx.get(reg)
            await ctx.tick()
            ctx.set(self.register.wstrobe, 0)
            assert ctx.get(reg)
            for _ in range(10):
                await ctx.tick()
                assert not ctx.get(reg)

        self.simulate(bench)

    def test_interrupt(self):
        async def bench(ctx):
            for _ in range(4):
                assert not ctx.get(self.register.interrupt)
                await ctx.tick()
            ctx.set(self.interrupt_enable, 1)
            assert not ctx.get(self.register.interrupt)
            await ctx.tick()
            ctx.set(self.interrupt_enable, 0)
            assert not ctx.get(self.register.interrupt)
            for _ in range(10):
                await ctx.tick()
                assert ctx.get(self.register.interrupt)
            ctx.set(self.register.ren, 1)
            intr_bit = 7 + 5 + 12 + 1
            assert ctx.get(self.register.interrupt)
            assert ctx.get(self.register.rdata[intr_bit])
            await ctx.tick()
            ctx.set(self.register.ren, 0)
            assert ctx.get(self.register.interrupt)
            await ctx.tick()
            for _ in range(6):
                assert not ctx.get(self.register.interrupt)
                await ctx.tick()
                assert not ctx.get(self.register.interrupt)
            ctx.set(self.interrupt_enable, 1)
            await ctx.tick()
            assert not ctx.get(self.register.interrupt)
            ctx.set(self.interrupt_enable, 0)
            ctx.set(self.register.ren, 1)
            assert not ctx.get(self.register.interrupt)
            assert ctx.get(self.register.rdata[intr_bit])
            await ctx.tick()
            ctx.set(self.register.ren, 0)
            assert ctx.get(self.register.interrupt)
            await ctx.tick()
            assert not ctx.get(self.register.interrupt)
            await ctx.tick()
            assert not ctx.get(self.register.interrupt)
            ctx.set(self.interrupt_enable, 1)
            ctx.set(self.register.ren, 1)
            assert not ctx.get(self.register.rdata[intr_bit])
            await ctx.tick()
            ctx.set(self.interrupt_enable, 0)
            ctx.set(self.register.ren, 0)
            assert not ctx.get(self.register.interrupt)
            await ctx.tick()
            assert ctx.get(self.register.interrupt)
            await ctx.tick()
            assert ctx.get(self.register.interrupt)
            ctx.set(self.interrupt_enable, 1)
            ctx.set(self.register.ren, 1)
            await ctx.tick()
            assert ctx.get(self.register.interrupt)
            ctx.set(self.interrupt_enable, 0)
            assert ctx.get(self.register.rdata[intr_bit])
            ctx.set(self.register.ren, 0)
            await ctx.tick()
            assert ctx.get(self.register.interrupt)
            ctx.set(self.register.ren, 1)
            assert ctx.get(self.register.interrupt)
            assert ctx.get(self.register.rdata[intr_bit])
            await ctx.tick()
            ctx.set(self.register.ren, 0)
            assert ctx.get(self.register.interrupt)
            await ctx.tick()
            assert not ctx.get(self.register.interrupt)

        self.simulate(bench)


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
        async def bench(ctx):
            await ctx.tick()
            ctx.set(self.dut.address, 0b10)
            ctx.set(self.dut.ren, 1)
            await ctx.tick()
            ctx.set(self.dut.ren, 0)
            assert ctx.get(self.dut.rdone) == 1
            assert ctx.get(self.dut.rdata) == 6789
            assert ctx.get(self.dut.wdone) == 0
            await ctx.tick()
            assert ctx.get(self.dut.rdone) == 0
            assert ctx.get(self.dut.rdata) == 0
            assert ctx.get(self.dut.wdone) == 0
            await ctx.tick()
            ctx.set(self.dut.address, 0b00)
            ctx.set(self.dut.ren, 1)
            await ctx.tick()
            ctx.set(self.dut.ren, 0)
            assert ctx.get(self.dut.rdone) == 1
            assert ctx.get(self.dut.rdata) == 12345
            assert ctx.get(self.dut.wdone) == 0
            await ctx.tick()
            ctx.set(self.dut.address, 0b10)
            ctx.set(self.dut.wstrobe, 0x8)
            ctx.set(self.dut.wdata, 0xffffffff)
            await ctx.tick()
            ctx.set(self.dut.wstrobe, 0)
            assert ctx.get(self.dut.rdone) == 0
            assert ctx.get(self.dut.rdata) == 0
            assert ctx.get(self.dut.wdone) == 1
            await ctx.tick()
            ctx.set(self.dut.ren, 1)
            await ctx.tick()
            ctx.set(self.dut.ren, 0)
            assert ctx.get(self.dut.rdone) == 1
            assert ctx.get(self.dut.rdata) == (0xff << 24) | 6789
            assert ctx.get(self.dut.wdone) == 0
            await ctx.tick()
            ctx.set(self.dut.address, 0b01)
            ctx.set(self.dut.ren, 1)
            await ctx.tick()
            ctx.set(self.dut.ren, 0)
            assert ctx.get(self.dut.rdone) == 1
            assert ctx.get(self.dut.rdata) == 0
            assert ctx.get(self.dut.wdone) == 0
            await ctx.tick()
            ctx.set(self.dut.wstrobe, 0xf)
            await ctx.tick()
            ctx.set(self.dut.wstrobe, 0)
            assert ctx.get(self.dut.rdone) == 0
            assert ctx.get(self.dut.rdata) == 0
            assert ctx.get(self.dut.wdone) == 1

        self.simulate(bench)


if __name__ == '__main__':
    unittest.main()
