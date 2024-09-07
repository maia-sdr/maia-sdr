#
# Copyright (C) 2023-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import numpy as np

import unittest

from maia_hdl.mult2x import Mult2x
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


class TestMult2x(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.domain_2x = 'clk2x'
        self.mult = Mult2x(
            self.domain_2x, c_width=self.width, r_width=self.width)
        self.dut = CommonEdgeTb(
            self.mult, [(self.domain_2x, 2, 'common_edge')])

    def test_random_inputs(self):
        num_inputs = 1000
        re = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                               size=num_inputs)
        im = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                               size=num_inputs)
        real = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                                 size=num_inputs)

        async def bench(ctx):
            for j in range(num_inputs):
                await ctx.tick()
                ctx.set(self.mult.clken, 1)
                ctx.set(self.mult.re_in, int(re[j]))
                ctx.set(self.mult.im_in, int(im[j]))
                ctx.set(self.mult.real_in, int(real[j]))
                if j >= self.mult.delay:
                    out = (
                        ctx.get(self.mult.re_out)
                        + 1j * ctx.get(self.mult.im_out))
                    expected = (
                        (re[j-self.mult.delay]
                         + 1j * im[j-self.mult.delay])
                        * real[j-self.mult.delay])
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'
        self.simulate(bench, named_clocks={self.domain_2x: 6e-9})


if __name__ == '__main__':
    unittest.main()
