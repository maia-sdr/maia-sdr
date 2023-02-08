#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import cocotb
from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge
from cocotb_bus.drivers.amba import AXI4LiteMaster

import numpy as np


class TB:
    def __init__(self, dut):
        self.dut = dut
        self.axi = AXI4LiteMaster(dut, '', dut.s_axi_lite_clk)


@cocotb.test()
async def test_noise_input(dut):
    dut.s_axi_lite_rst.value = 1
    dut.clk.value = 0
    dut.s_axi_lite_clk.value = 0
    dut.AWVALID.value = 0
    dut.AWADDR.value = 0
    dut.AWPROT.value = 0
    dut.WVALID.value = 0
    dut.WDATA.value = 0
    dut.WSTRB.value = 0
    dut.BREADY.value = 0
    dut.ARVALID.value = 0
    dut.ARADDR.value = 0
    dut.ARPROT.value = 0
    dut.RREADY.value = 0
    cocotb.start_soon(Clock(dut.sampling_clk, 16, units='ns').start())
    cocotb.start_soon(Clock(dut.clk, 12, units='ns').start())
    cocotb.start_soon(Clock(dut.clk2x_clk, 6, units='ns').start())
    cocotb.start_soon(Clock(dut.clk3x_clk, 4, units='ns').start())
    cocotb.start_soon(Clock(dut.s_axi_lite_clk, 10, units='ns').start())
    await ClockCycles(dut.s_axi_lite_clk, 4)
    tb = TB(dut)
    dut.s_axi_lite_rst.value = 0

    # remove IP core reset
    assert dut.rst.value == 1
    await tb.axi.write(0x8, 0x0)
    await ClockCycles(dut.s_axi_lite_clk, 20)
    assert dut.rst.value == 0

    scale = 1024
    mask = 2**12 - 1
    rising = RisingEdge(dut.clk)
    for _ in range(20000):
        re, im = np.round(np.random.randn(2) * scale)
        dut.re_in.value = int(re) & mask
        dut.im_in.value = int(im) & mask
        await rising
