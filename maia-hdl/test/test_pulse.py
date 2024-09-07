#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *

import unittest

from maia_hdl.pulse import PulseStretcher
from .amaranth_sim import AmaranthSim


class TestPulseStretcher(AmaranthSim):
    def test_pulse_stretcher(self):
        pulse_len_log2 = 2
        pulse_len = 2**pulse_len_log2
        self.dut = PulseStretcher(pulse_len_log2)

        async def bench(ctx):
            for _ in range(10):
                await ctx.tick()
                assert not ctx.get(self.dut.pulse_out)
            ctx.set(self.dut.pulse_in, 1)
            assert not ctx.get(self.dut.pulse_out)
            await ctx.tick()
            ctx.set(self.dut.pulse_in, 0)
            for _ in range(pulse_len):
                assert ctx.get(self.dut.pulse_out)
                await ctx.tick()
            for _ in range(15):
                assert not ctx.get(self.dut.pulse_out)
                await ctx.tick()
            ctx.set(self.dut.pulse_in, 1)
            assert not ctx.get(self.dut.pulse_out)
            await ctx.tick()
            ctx.set(self.dut.pulse_in, 0)
            assert ctx.get(self.dut.pulse_out)
            await ctx.tick()
            assert ctx.get(self.dut.pulse_out)
            await ctx.tick()
            ctx.set(self.dut.pulse_in, 1)
            assert ctx.get(self.dut.pulse_out)
            await ctx.tick()
            ctx.set(self.dut.pulse_in, 0)
            for _ in range(pulse_len):
                assert ctx.get(self.dut.pulse_out)
                await ctx.tick()
            for _ in range(8):
                assert not ctx.get(self.dut.pulse_out)
                await ctx.tick()

        self.simulate(bench)


if __name__ == '__main__':
    unittest.main()
