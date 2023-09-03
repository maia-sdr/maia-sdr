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

from maia_hdl.cpwr import Cpwr
from .amaranth_sim import AmaranthSim


class TestCpwr(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.add_width = 24

    def test_random_inputs(self):
        for add_latency in [0, 1]:
            for truncate in [0, 4]:
                for add_shift in [8, 16]:
                    self.add_latency = add_latency
                    with self.subTest(
                            add_latency=add_latency, truncate=truncate,
                            add_shift=add_shift):
                        self.dut = Cpwr(
                            width=self.width, add_width=self.add_width,
                            add_shift=add_shift, add_latency=add_latency,
                            truncate=truncate)
                        self.common_random_inputs()

    def common_random_inputs(self, vcd=None):
        num_inputs = 1000
        re = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                               size=num_inputs)
        im = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                               size=num_inputs)
        add = np.random.randint(-2**(self.add_width-1), 2**(self.add_width-1),
                                size=num_inputs)

        def bench():
            for j in range(num_inputs):
                yield self.dut.clken.eq(1)
                yield self.dut.re_in.eq(int(re[j]))
                yield self.dut.im_in.eq(int(im[j]))
                yield self.dut.add_in.eq(int(add[j]))
                yield
                if j >= self.dut.delay:
                    out = yield self.dut.out
                    k = j - self.dut.delay
                    expected = self.dut.model(
                        re[k], im[k], add[k + self.add_latency])
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'
        self.simulate(bench, vcd)


if __name__ == '__main__':
    unittest.main()
