#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import numpy as np

import unittest

from maia_hdl.cmult import Cmult, Cmult3x
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


class TestCmult(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.dut = Cmult(a_width=self.width, b_width=self.width)

    def test_random_inputs(self):
        num_inputs = 1000
        re_a = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                                 size=num_inputs)
        im_a = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                                 size=num_inputs)
        re_b = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                                 size=num_inputs)
        im_b = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                                 size=num_inputs)

        async def bench(ctx):
            for j in range(num_inputs):
                await ctx.tick()
                ctx.set(self.dut.clken, 1)
                ctx.set(self.dut.re_a, int(re_a[j]))
                ctx.set(self.dut.im_a, int(im_a[j]))
                ctx.set(self.dut.re_b, int(re_b[j]))
                ctx.set(self.dut.im_b, int(im_b[j]))
                if j >= self.dut.delay:
                    out = (
                        ctx.get(self.dut.re_out)
                        + 1j * ctx.get(self.dut.im_out))
                    expected = (
                        (re_a[j-self.dut.delay]
                         + 1j * im_a[j-self.dut.delay])
                        * (re_b[j-self.dut.delay]
                           + 1j * im_b[j-self.dut.delay]))
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'

        self.simulate(bench)


class TestCmult3x(AmaranthSim):
    def test_random_inputs(self):
        self.random_inputs_common(16, 16)

    def test_random_inputs_width(self):
        self.random_inputs_common(22, 16)

    def random_inputs_common(self, a_width, b_width):
        domain_3x = 'clk3x'
        cmult = Cmult3x(
            domain_3x, a_width=a_width, b_width=b_width)
        self.dut = CommonEdgeTb(
            cmult, [(domain_3x, 3, 'common_edge')])
        num_inputs = 1000
        re_a = np.random.randint(-2**(a_width-1), 2**(a_width-1),
                                 size=num_inputs)
        im_a = np.random.randint(-2**(a_width-1), 2**(a_width-1),
                                 size=num_inputs)
        re_b = np.random.randint(-2**(b_width-1), 2**(b_width-1),
                                 size=num_inputs)
        im_b = np.random.randint(-2**(b_width-1), 2**(b_width-1),
                                 size=num_inputs)

        async def bench(ctx):
            for j in range(num_inputs):
                await ctx.tick()
                ctx.set(cmult.clken, 1)
                ctx.set(cmult.re_a, int(re_a[j]))
                ctx.set(cmult.im_a, int(im_a[j]))
                ctx.set(cmult.re_b, int(re_b[j]))
                ctx.set(cmult.im_b, int(im_b[j]))
                # wait until a clk3x tick, since im_out is set
                # a clk3x tick after re_out
                await ctx.tick('clk3x')
                if j >= cmult.delay:
                    out = (
                        ctx.get(cmult.re_out)
                        + 1j * ctx.get(cmult.im_out))
                    expected = (
                        (re_a[j-cmult.delay]
                         + 1j * im_a[j-cmult.delay])
                        * (re_b[j-cmult.delay]
                           + 1j * im_b[j-cmult.delay]))
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'

        self.simulate(bench, named_clocks={domain_3x: 4e-9})


if __name__ == '__main__':
    unittest.main()
