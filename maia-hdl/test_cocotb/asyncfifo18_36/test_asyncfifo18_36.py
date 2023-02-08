#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import cocotb

from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge, FallingEdge


async def counter(dut, count, max_iter=1000):
    rising = RisingEdge(dut.write_clk)
    for n in range(count):
        for _ in range(max_iter):
            await rising
            assert not dut.wrerr.value
            dut.data_in.value = n
            dut.wren.value = wren = not dut.full.value
            if wren:
                break
        else:
            raise Exception('exceded maximum iterations')


@cocotb.test()
async def test_asyncfifo18_36(dut):
    dut.fifo_rst.value = 1
    dut.read_rst.value = 1
    dut.write_rst.value = 1
    dut.wren.value = 0
    dut.rden.value = 0
    cocotb.start_soon(Clock(dut.write_clk, 11, units='ns').start())
    cocotb.start_soon(Clock(dut.read_clk, 10, units='ns').start())
    await ClockCycles(dut.write_clk, 10)
    dut.fifo_rst.value = 0
    dut.read_rst.value = 0
    dut.write_rst.value = 0
    await ClockCycles(dut.write_clk, 5)

    count = 1024
    cocotb.start_soon(counter(dut, count))

    falling = FallingEdge(dut.read_clk)
    for n in range(count):
        for _ in range(1000):
            await falling
            assert not dut.rderr.value
            has_read = dut.rden.value
            dut.rden.value = rden = not dut.empty.value
            if has_read:
                assert dut.data_out.value == n
                break
