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

import numpy as np

import cocotb
from cocotb_bus.drivers import BitDriver

from axi import AXI4Slave
from backpressure import RandomReady
from memory import Memory

from cocotb.clock import Clock
from cocotb.triggers import ClockCycles, RisingEdge
from cocotb.regression import TestFactory

MEMORY_START = 0x00000000
MEMORY_END = 0x00001000
MEMORY_BYTES = MEMORY_END - MEMORY_START


class RecorderTB:
    def __init__(self, dut):
        self.dut = dut
        self.memory = Memory(MEMORY_BYTES)
        self.subordinate = AXI4Slave(dut, None, dut.clk, self.memory)
        self.backpressure = BitDriver(dut.WREADY, dut.clk)


async def iq_data(dut, sample_stream):
    n = 0
    rising = RisingEdge(dut.iq_clk)
    while True:
        await rising
        dut.strobe_in.value = 1
        dut.re_in.value = re = random.randrange(-2**11, 2**11)
        dut.im_in.value = im = random.randrange(-2**11, 2**11)
        sample_stream.append((re, im))
        await rising
        dut.strobe_in.value = 0
        await rising


async def start(dut):
    rising = RisingEdge(dut.clk)
    await rising
    dut.start.value = 1
    await rising
    dut.start.value = 0


async def stop(dut):
    rising = RisingEdge(dut.clk)
    await rising
    dut.stop.value = 1
    await rising
    dut.stop.value = 0


async def wait_finished(dut):
    rising = RisingEdge(dut.clk)
    while True:
        await rising
        if dut.finished.value:
            return


def check_output(tb, sample_stream):
    written = tb.memory._data[:tb.dut.next_address.value]
    sample_re = np.array([a[0] for a in sample_stream])
    sample_im = np.array([a[1] for a in sample_stream])
    if tb.dut.mode_8bit.value:
        re = np.array(written[::2], 'int8').astype('int16') << 4
        im = np.array(written[1::2], 'int8').astype('int16') << 4
        sample_re = sample_re >> 4 << 4
        sample_im = sample_im >> 4 << 4
    else:
        L = len(written) // 3 * 3
        re = ((np.array(written[:L:3], 'int8').astype('int16') << 4)
              | (np.array(written[1:L:3], 'uint8').astype('int16') >> 4))
        im = (((np.array(written[1:L:3], 'uint8') << 4).view('int8')
               .astype('int16') << 4)
              | np.array(written[2:L:3], 'uint8'))
    for j in range(sample_re.size - re.size):
        if sample_re[j] == re[0]:
            re_match = np.array_equal(sample_re[j:][:re.size], re)
            im_match = np.array_equal(sample_im[j:][:re.size], im)
            if re_match and im_match:
                break
    else:
        raise Exception('unable to find match in sample_stream')


async def run_test(dut, backpressure_inserter=None):
    sample_stream = []
    cocotb.start_soon(Clock(dut.clk, 10, units='ns').start())
    cocotb.start_soon(Clock(dut.iq_clk, 12, units='ns').start())
    dut.rst.value = 1
    dut.iq_rst.value = 1
    dut.start.value = 0
    dut.stop.value = 0
    dut.mode_8bit.value = 0
    dut.strobe_in.value = 0
    await ClockCycles(dut.clk, 10)
    tb = RecorderTB(dut)
    dut.rst.value = 0
    dut.iq_rst.value = 0

    if backpressure_inserter:
        tb.backpressure.start(backpressure_inserter())

    cocotb.start_soon(iq_data(dut, sample_stream))

    await ClockCycles(dut.clk, 10)
    del sample_stream[:]
    await start(dut)
    await wait_finished(dut)
    assert dut.next_address.value == MEMORY_END
    await ClockCycles(dut.clk, 20)
    assert dut.dropped_samples.value == 0
    check_output(tb, sample_stream)

    await ClockCycles(dut.clk, 100)
    dut.mode_8bit.value = 1
    await ClockCycles(dut.clk, 20)
    del sample_stream[:]
    await start(dut)
    await ClockCycles(dut.clk, 1000)
    await stop(dut)
    await wait_finished(dut)
    assert dut.next_address.value.integer < MEMORY_END
    await ClockCycles(dut.clk, 20)
    assert dut.dropped_samples.value == 0
    check_output(tb, sample_stream)


factory = TestFactory(run_test)
factory.add_option('backpressure_inserter',
                   [None, RandomReady(), RandomReady(2, 2)])
factory.generate_tests()
