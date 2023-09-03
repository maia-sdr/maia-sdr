#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import array
import random
import math
import struct

import cocotb
from cocotb_bus.drivers import BitDriver

from axi import AXI4Slave
from backpressure import RandomReady
from memory import Memory

from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge
from cocotb.regression import TestFactory

NUM_WRITES = 3  # write 3 buffers per test
MEMORY_START = 0x0000f000
MEMORY_END = 0x00011000
MEMORY_BYTES = MEMORY_END - MEMORY_START


class DmaStreamWriteTB:
    def __init__(self, dut):
        self.memory = Memory(MEMORY_BYTES)
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
            if dut.finished.value:
                break
            await rising


async def check_address(dut):
    rising = RisingEdge(dut.clk)
    await rising
    while True:
        if dut.AWVALID.value:
            address = int(dut.AWADDR.value)
            assert MEMORY_START <= address < MEMORY_END
        await rising


async def stream_data(dut):
    n = 0
    rising = RisingEdge(dut.clk)
    while True:
        await rising
        if dut.stream_valid.value and dut.stream_ready.value:
            n += 1
        dut.stream_data.value = n
        # TODO: add valid generation
        dut.stream_valid.value = 1


async def run_test(dut, backpressure_inserter=None):
    cocotb.start_soon(Clock(dut.clk, 10, units='ns').start())
    cocotb.start_soon(check_address(dut))
    dut.rst.value = 1
    dut.start.value = 0
    dut.stop.value = 0
    await ClockCycles(dut.clk, 2)
    tb = DmaStreamWriteTB(dut)
    dut.rst.value = 0

    if backpressure_inserter:
        tb.backpressure.start(backpressure_inserter())

    bytes_written = 0
    bytes_per_word = 8

    cocotb.start_soon(stream_data(dut))
    starts_task = cocotb.start_soon(starts(dut))

    risign = RisingEdge(dut.clk)
    for _ in range(10000000):
        if dut.WREADY.value and dut.WVALID.value:
            bytes_written += bytes_per_word
        await risign
        if starts_task.done():
            break

    assert bytes_written == NUM_WRITES * MEMORY_BYTES
    expected = array.array('B', [0] * MEMORY_BYTES)
    for j, word in enumerate(
            range((NUM_WRITES - 1) * MEMORY_BYTES // bytes_per_word,
                  NUM_WRITES * MEMORY_BYTES // bytes_per_word)):
        address = (bytes_per_word * j + MEMORY_START) % MEMORY_BYTES
        expected[address:address+bytes_per_word] = (
            array.array('B', struct.pack('<Q', word)))

    assert tb.memory._data == expected, \
        'memory contents do not match'


factory = TestFactory(run_test)
factory.add_option('backpressure_inserter',
                   [None, RandomReady(), RandomReady(2, 2)])
factory.generate_tests()
