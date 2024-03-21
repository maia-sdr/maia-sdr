#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import numpy as np

import unittest

from maia_hdl.floating_point import IQToFloatingPoint, MakeCommonExponent
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


class TestIQToFloatingPoint(AmaranthSim):
    def test_random_inputs(self):
        in_width = 22
        out_width = 18
        self.dut = IQToFloatingPoint(in_width, out_width)

        num_inputs = 2048
        re, im = (np.random.randint(-2**(in_width-1), 2**(in_width-1),
                                    size=num_inputs)
                  for _ in range(2))

        re_expected, im_expected, exp_expected = self.dut.model(re, im)

        def bench():
            for j in range(num_inputs):
                yield self.dut.clken.eq(1)
                yield self.dut.re_in.eq(int(re[j]))
                yield self.dut.im_in.eq(int(im[j]))
                yield
                if j >= self.dut.delay:
                    re_out = yield self.dut.re_out
                    im_out = yield self.dut.im_out
                    exponent_out = yield self.dut.exponent_out
                    k = j - self.dut.delay
                    assert re_out == re_expected[k], \
                        (f're_out = {re_out}, expected = {re_expected[k]} '
                         f'@ cycle = {j}')
                    assert im_out == im_expected[k], \
                        (f'im_out = {im_out}, expected = {im_expected[k]} '
                         f'@ cycle = {j}')
                    assert exponent_out == exp_expected[k], \
                        (f'exponent_out = {exponent_out}, '
                         f'expected = {exp_expected[k]} @ cycle = {j}')

        self.simulate(bench)


class TestMakeCommonExponent(AmaranthSim):
    def test_random_inputs(self):
        a_width = 22
        b_width = 47
        exponent_width = 3
        max_exponent = 4
        assert max_exponent < 2**exponent_width
        self.dut = MakeCommonExponent(
            a_width, b_width, exponent_width, max_exponent,
            a_complex=True, b_signed=False, b_power=True)

        num_inputs = 2048
        re_a, im_a = (np.random.randint(-2**(a_width-1), 2**(a_width-1),
                                        size=num_inputs)
                      for _ in range(2))
        b = np.random.randint(0, 2**b_width, size=num_inputs)
        exp_a, exp_b = (np.random.randint(0, max_exponent + 1,
                                          size=num_inputs)
                        for _ in range(2))

        (expected_re_a, expected_im_a, expected_b, _, expected_exp) = (
            self.dut.model(re_a, im_a, exp_a,
                           b, np.zeros(num_inputs, 'int'), exp_b))

        def bench():
            for j in range(num_inputs):
                yield self.dut.clken.eq(1)
                yield self.dut.re_a_in.eq(int(re_a[j]))
                yield self.dut.im_a_in.eq(int(im_a[j]))
                yield self.dut.exponent_a_in.eq(int(exp_a[j]))
                yield self.dut.b_in.eq(int(b[j]))
                yield self.dut.exponent_b_in.eq(int(exp_b[j]))
                yield
                if j >= self.dut.delay:
                    re_a_out = yield self.dut.re_a_out
                    im_a_out = yield self.dut.im_a_out
                    b_out = yield self.dut.b_out
                    exponent_out = yield self.dut.exponent_out
                    k = j - self.dut.delay
                    assert re_a_out == expected_re_a[k], \
                        (f're_a_out = {re_a_out}, '
                         f'expected = {expected_re_a[k]} @ cycle = {j}')
                    assert im_a_out == expected_im_a[k], \
                        (f'im_a_out = {im_a_out}, '
                         f'expected = {expected_im_a[k]} @ cycle = {j}')
                    assert b_out == expected_b[k], \
                        (f'b_out = {b_out}, expected = {expected_b[k]} '
                         f'@ cycle = {j}')
                    assert exponent_out == expected_exp[k], \
                        (f'exponent_out = {exponent_out}, '
                         f'expected = {expected_exp[k]} @ cycle = {j}')

        self.simulate(bench, 'make_common_exp.vcd')


if __name__ == '__main__':
    unittest.main()
