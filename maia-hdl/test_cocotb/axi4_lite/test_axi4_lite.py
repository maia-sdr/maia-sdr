#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import cocotb
from cocotb_bus.drivers.amba import AXI4LiteMaster

from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge, Timer
from cocotb.regression import TestFactory


class Axi4LiteTB:
    def __init__(self, dut):
        self.dut = dut
        self.axi = AXI4LiteMaster(dut, '', dut.clk)


async def run_test(dut):
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
    dut.rst.value = 1
    dut.clk.value = 0
    cocotb.start_soon(Clock(dut.clk, 10, units='ns').start())
    await ClockCycles(dut.clk, 2)
    tb = Axi4LiteTB(dut)
    dut.rst.value = 0

    value = await tb.axi.read(0x0)
    print('read register 0x0')
    assert value == 0xf001baa2

    value = await tb.axi.read(0x4)
    assert value == 0x1234

    value = await tb.axi.read(0x8)
    assert value == 0

    value = await tb.axi.read(0xc)
    assert value == 0

    # write to read-only register
    await tb.axi.write(0x0, 0xdeadbeef)
    value = await tb.axi.read(0x0)
    assert value == 0xf001baa2

    await tb.axi.write(0x4, 0xdeadbeef)
    value = await tb.axi.read(0x4)
    assert value == 0xdeadbeef

    await tb.axi.write(0x8, 0xfecba987)
    value = await tb.axi.read(0x8)
    assert value == 0xfecba987

    # write to non-existent register
    await tb.axi.write(0xc, 0xffff1111)
    value = await tb.axi.read(0xc)
    assert value == 0


factory = TestFactory(run_test)
factory.generate_tests()
