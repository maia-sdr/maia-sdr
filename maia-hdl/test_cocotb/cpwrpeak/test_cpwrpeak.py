#
# Copyright (C) 2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import cocotb
from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge, FallingEdge
from cocotb.regression import TestFactory

import random


async def run_test(dut, peak_detect=False):
    dut.rst.value = 1
    dut.clk3x_rst.value = 1
    dut.clken.value = 1
    dut.re_in.value = 0
    dut.im_in.value = 0
    dut.real_in.value = 0
    dut.peak_detect.value = peak_detect
    cocotb.start_soon(Clock(dut.clk, 12, units='ns').start())
    cocotb.start_soon(Clock(dut.clk3x_clk, 4, units='ns').start())
    # We need to wait for 100 ns for GSR to go low
    await ClockCycles(dut.clk, 20)
    dut.rst.value = 0
    dut.clk3x_rst.value = 0

    rising = RisingEdge(dut.clk)
    num_inputs = 1000
    dut_delay = 4  # needs to be one more than the DUT delay @property

    re_in, im_in = (
        [random.randrange(-2**15, 2**15) for _ in range(num_inputs)]
        for _ in range(2))
    real_in = [random.randrange(-2**23, 2**23) for _ in range(num_inputs)]

    for j in range(num_inputs):
        await rising
        dut.re_in.value = re_in[j]
        dut.im_in.value = im_in[j]
        dut.real_in.value = real_in[j]
        if j >= dut_delay:
            a = re_in[j - dut_delay]
            b = im_in[j - dut_delay]
            c = real_in[j - dut_delay]
            out = dut.out.value.signed_integer
            if peak_detect:
                assert out == (a**2 + b**2) >> 16
                is_greater = dut.is_greater.value
                assert is_greater == ((a**2 + b**2) >= (c << 16))
            else:
                assert out == (a**2 + b**2 + (c << 16)) >> 16


factory = TestFactory(run_test)
factory.add_option('peak_detect', [False, True])
factory.generate_tests()
