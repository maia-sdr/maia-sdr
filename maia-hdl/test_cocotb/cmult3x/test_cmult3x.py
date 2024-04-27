#
# Copyright (C) 2023-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import cocotb
from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge, FallingEdge

import random


@cocotb.test()
async def test_cmult3x(dut):
    dut.rst.value = 1
    dut.clk3x_rst.value = 1
    dut.clken.value = 1
    dut.re_a.value = 0
    dut.im_a.value = 0
    dut.re_b.value = 0
    dut.im_b.value = 0
    dut.wide_re_a.value = 0
    dut.wide_im_a.value = 0
    dut.wide_re_b.value = 0
    dut.wide_im_b.value = 0
    cocotb.start_soon(Clock(dut.clk, 12, units='ns').start())
    cocotb.start_soon(Clock(dut.clk3x_clk, 4, units='ns').start())
    # We need to wait for 100 ns for GSR to go low
    await ClockCycles(dut.clk, 20)
    dut.rst.value = 0
    dut.clk3x_rst.value = 0

    rising = RisingEdge(dut.clk)
    num_inputs = 1000
    dut_delay = 3
    dut_wide_delay = 4

    re_a, im_a, re_b, im_b, wide_re_b, wide_im_b = (
        [random.randrange(-2**15, 2**15) for _ in range(num_inputs)]
        for _ in range(6))
    wide_re_a, wide_im_a = (
        [random.randrange(-2**15, 2**15) for _ in range(num_inputs)]
        for _ in range(2))

    for j in range(num_inputs):
        await rising
        dut.re_a.value = re_a[j]
        dut.im_a.value = im_a[j]
        dut.re_b.value = re_b[j]
        dut.im_b.value = im_b[j]
        dut.wide_re_a.value = wide_re_a[j]
        dut.wide_im_a.value = wide_im_a[j]
        dut.wide_re_b.value = wide_re_b[j]
        dut.wide_im_b.value = wide_im_b[j]
        if j >= dut_delay:
            a = re_a[j - dut_delay]
            b = im_a[j - dut_delay]
            c = re_b[j - dut_delay]
            d = im_b[j - dut_delay]
            re_out = dut.re_out.value.signed_integer
            im_out = dut.im_out.value.signed_integer
            assert re_out == a * c - b * d
            assert im_out == a * d + b * c
        if j >= dut_wide_delay:
            wa = wide_re_a[j - dut_wide_delay]
            wb = wide_im_a[j - dut_wide_delay]
            wc = wide_re_b[j - dut_wide_delay]
            wd = wide_im_b[j - dut_wide_delay]
            wide_re_out = dut.wide_re_out.value.signed_integer
            wide_im_out = dut.wide_im_out.value.signed_integer
            assert wide_re_out == wa * wc - wb * wd
            assert wide_im_out == wa * wd + wb * wc
