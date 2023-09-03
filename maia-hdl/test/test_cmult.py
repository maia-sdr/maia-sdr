#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
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

        def bench():
            for j in range(num_inputs):
                yield self.dut.clken.eq(1)
                yield self.dut.re_a.eq(int(re_a[j]))
                yield self.dut.im_a.eq(int(im_a[j]))
                yield self.dut.re_b.eq(int(re_b[j]))
                yield self.dut.im_b.eq(int(im_b[j]))
                yield
                if j >= self.dut.delay:
                    out = (
                        (yield self.dut.re_out)
                        + 1j * (yield self.dut.im_out))
                    expected = (
                        (re_a[j-self.dut.delay]
                         + 1j * im_a[j-self.dut.delay])
                        * (re_b[j-self.dut.delay]
                           + 1j * im_b[j-self.dut.delay]))
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'

        self.simulate(bench)


class TestCmult3x(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.domain_3x = 'clk3x'
        self.cmult = Cmult3x(
            self.domain_3x, a_width=self.width, b_width=self.width)
        self.dut = CommonEdgeTb(
            self.cmult, [(self.domain_3x, 3, 'common_edge')])

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

        def bench():
            for j in range(num_inputs):
                yield self.cmult.clken.eq(1)
                yield self.cmult.re_a.eq(int(re_a[j]))
                yield self.cmult.im_a.eq(int(im_a[j]))
                yield self.cmult.re_b.eq(int(re_b[j]))
                yield self.cmult.im_b.eq(int(im_b[j]))
                yield
                if j >= self.cmult.delay:
                    out = (
                        (yield self.cmult.re_out)
                        + 1j * (yield self.cmult.im_out))
                    expected = (
                        (re_a[j-self.cmult.delay]
                         + 1j * im_a[j-self.cmult.delay])
                        * (re_b[j-self.cmult.delay]
                           + 1j * im_b[j-self.cmult.delay]))
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'

        self.simulate(bench, named_clocks={self.domain_3x: 4e-9})


if __name__ == '__main__':
    unittest.main()
