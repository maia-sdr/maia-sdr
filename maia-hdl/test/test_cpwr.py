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

from maia_hdl.cpwr import Cpwr, CpwrPeak
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


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


class TestCpwrPeak(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.real_width = 24
        self.domain_3x = 'clk3x'

    def test_random_inputs(self):
        for peak_detect in [False, True]:
            for truncate in [0, 4]:
                for real_shift in [8, 16]:
                    self.peak_detect = peak_detect
                    with self.subTest(peak_detect=peak_detect,
                                      truncate=truncate,
                                      real_shift=real_shift):
                        self.cpwr = CpwrPeak(
                            self.domain_3x, width=self.width,
                            real_width=self.real_width, real_shift=real_shift,
                            truncate=truncate)
                        self.dut = CommonEdgeTb(
                            self.cpwr, [(self.domain_3x, 3, 'common_edge')])
                        self.common_random_inputs()

    def common_random_inputs(self, vcd=None):
        num_inputs = 1000
        re = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                               size=num_inputs)
        im = np.random.randint(-2**(self.width-1), 2**(self.width-1),
                               size=num_inputs)
        real = np.random.randint(
            -2**(self.real_width-1), 2**(self.real_width-1),
            size=num_inputs)

        def bench():
            for j in range(num_inputs):
                yield self.cpwr.clken.eq(1)
                yield self.cpwr.peak_detect.eq(self.peak_detect)
                yield self.cpwr.re_in.eq(int(re[j]))
                yield self.cpwr.im_in.eq(int(im[j]))
                yield self.cpwr.real_in.eq(int(real[j]))
                yield
                if j >= self.cpwr.delay:
                    out = yield self.cpwr.out
                    k = j - self.cpwr.delay
                    expected = self.cpwr.model(
                        re[k], im[k], real[k], self.peak_detect)
                    if self.peak_detect:
                        is_greater = yield self.cpwr.is_greater
                        is_greater_expected = expected[1]
                        assert is_greater == is_greater_expected, \
                            (f'is_greater = {is_greater}, '
                             f'expected = {expected} @ cycle = {j}')
                        expected = expected[0]
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'
        self.simulate(bench, vcd, named_clocks={self.domain_3x: 4e-9})


if __name__ == '__main__':
    unittest.main()
