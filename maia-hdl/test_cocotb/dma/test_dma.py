#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import array
import random
import struct

import cocotb
from cocotb_bus.drivers import BitDriver

from axi import AXI4Slave
from backpressure import RandomReady
from memory import Memory

from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge
from cocotb.regression import TestFactory


BRAM_SIZE = 4096
NUM_WRITES = 3  # write 3 transfers per test


class DmaBRAMWriteTB:
    def __init__(self, dut):
        self.memory = Memory(2 * 1024 * 1024)
        self.subordinate = AXI4Slave(dut, None, dut.clk, self.memory)
        self.backpressure = BitDriver(dut.WREADY, dut.clk)


async def starts(dut):
    for _ in range(NUM_WRITES):
        rising = RisingEdge(dut.clk)
        await ClockCycles(dut.clk, 4)
        dut.start.value = 1
        await rising
        dut.start.value = 0
        await rising
        while True:
            if not dut.busy.value:
                break
            await rising


async def check_address(dut):
    rising = RisingEdge(dut.clk)
    await rising
    while True:
        if dut.AWVALID.value:
            assert int(dut.AWADDR.value) >> 20 == 0x080
        await rising


async def run_test(dut, backpressure_inserter=None):
    cocotb.start_soon(Clock(dut.clk, 10, units='ns').start())
    cocotb.start_soon(check_address(dut))
    dut.rst.value = 1
    dut.start.value = 0
    await ClockCycles(dut.clk, 2)
    tb = DmaBRAMWriteTB(dut)
    dut.rst.value = 0

    if backpressure_inserter:
        tb.backpressure.start(backpressure_inserter())

    bytes_written = 0
    bytes_per_word = 8

    starts_task = cocotb.start_soon(starts(dut))

    risign = RisingEdge(dut.clk)
    for _ in range(100000):
        if dut.WREADY.value and dut.WVALID.value:
            bytes_written += bytes_per_word
        await risign
        if starts_task.done():
            break

    assert bytes_written == NUM_WRITES * BRAM_SIZE * 8  # 8 bytes/word
    expected = array.array('B', [0] * bytes_written)
    for word in range(bytes_written // bytes_per_word):
        expected[word*bytes_per_word:(word+1)*bytes_per_word] = (
            array.array('B', struct.pack('<Q', word % BRAM_SIZE)))

    assert tb.memory._data[:bytes_written] == expected, \
        'memory contents do not match'


factory = TestFactory(run_test)
factory.add_option('backpressure_inserter',
                   [None, RandomReady(), RandomReady(2, 2)])
factory.generate_tests()
