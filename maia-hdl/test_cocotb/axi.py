# Copyright (c) 2022,2026 Daniel Estevez <daniel@destevez.net>
# Copyright cocotb contributors
# Copyright (c) 2014 Potential Ventures Ltd
# Licensed under the Revised BSD License, see LICENSE for details.
# SPDX-License-Identifier: BSD-3-Clause

# Modified AXI4Slave from cocotb_bus

import array
import collections.abc
import enum
import itertools
from typing import Any, List, Optional, Sequence, Tuple, Union

import cocotb
from cocotb.handle import Immediate, SimHandleBase
from cocotb.triggers import ClockCycles, Combine, Lock, ReadOnly, RisingEdge
from cocotb.types import LogicArray

from cocotb_bus.drivers import BusDriver


class AXI4Slave(BusDriver):
    '''
    AXI4 Slave
    Monitors an internal memory and handles read and write requests.
    '''
    _signals = [
        "ARREADY", "ARVALID", "ARADDR",             # Read address channel
        "ARLEN",   "ARSIZE",  "ARBURST", "ARPROT",

        "RREADY",  "RVALID",  "RDATA",   "RLAST",   # Read response channel

        "AWREADY", "AWADDR",  "AWVALID",            # Write address channel
        "AWPROT",  "AWSIZE",  "AWBURST", "AWLEN",

        "WREADY",  "WVALID",  "WDATA",

    ]

    # Not currently supported by this driver
    _optional_signals = [
        "WLAST",   "WSTRB",
        "BVALID",  "BREADY",  "BRESP",   "RRESP",
        "RCOUNT",  "WCOUNT",  "RACOUNT", "WACOUNT",
        "ARLOCK",  "AWLOCK",  "ARCACHE", "AWCACHE",
        "ARQOS",   "AWQOS",   "ARID",    "AWID",
        "BID",     "RID",     "WID"
    ]

    def __init__(self, entity, name, clock, memory, callback=None, event=None,
                 big_endian=False, backpressure_inserter=None, **kwargs):

        BusDriver.__init__(self, entity, name, clock, **kwargs)
        self.clock = clock

        self.big_endian = big_endian
        self.backpressure = (backpressure_inserter()
                             if backpressure_inserter is not None
                             else None)
        self._wready_cycles = None
        self.bus.ARREADY.set(Immediate(1))
        self.bus.RVALID.set(Immediate(0))
        self.bus.RLAST.set(Immediate(0))
        self.bus.AWREADY.set(Immediate(1))
        self.bus.WREADY.set(Immediate(0))
        if hasattr(self.bus, "BVALID"):
            self.bus.BVALID.set(Immediate(0))
        self._memory = memory

        self.write_address_busy = Lock()
        self.read_address_busy = Lock()
        self.write_data_busy = Lock()

        self._aw = []
        cocotb.start_soon(self._aw_data())
        cocotb.start_soon(self._read_data())
        cocotb.start_soon(self._write_data())

    def _size_to_bytes_in_beat(self, AxSIZE):
        if AxSIZE < 7:
            return 2 ** AxSIZE
        return None

    async def _aw_data(self):
        max_awaddr_queue = 32
        clock_re = RisingEdge(self.clock)

        while True:
            if len(self._aw) < max_awaddr_queue:
                self.bus.AWREADY.value = 1
            else:
                self.bus.AWREADY.value = 0
            if self.bus.AWREADY.value and self.bus.AWVALID.value:
                _awaddr = self.bus.AWADDR.value.to_unsigned()
                _awlen = self.bus.AWLEN.value.to_unsigned()
                _awsize = self.bus.AWSIZE.value.to_unsigned()
                _awburst = self.bus.AWBURST.value.to_unsigned()
                _awprot = self.bus.AWPROT.value.to_unsigned()
                burst_length = _awlen + 1
                bytes_in_beat = self._size_to_bytes_in_beat(_awsize)

                self._aw.append({
                    '_awaddr': _awaddr,
                    '_awlen': _awlen,
                    '_awsize': _awsize,
                    '_awburst': _awburst,
                    '_awprot': _awprot,
                    'burst_length': burst_length,
                    'bytes_in_beat': bytes_in_beat,
                })

                if __debug__:
                    self.log.debug(
                        "AWADDR  %d\n" % _awaddr +
                        "AWLEN   %d\n" % _awlen +
                        "AWSIZE  %d\n" % _awsize +
                        "AWBURST %d\n" % _awburst +
                        "AWPROT %d\n" % _awprot +
                        "BURST_LENGTH %d\n" % burst_length +
                        "Bytes in beat %d\n" % bytes_in_beat)
            await clock_re

    def _update_wready(self):
        if self.backpressure is None:
            self.bus.WREADY.value = 1
            return
        if self._wready_cycles is None or (self._wready_cycles[0] == 0
                                           and self._wready_cycles[1] == 0):
            self._wready_cycles = list(next(self.backpressure))
        if self._wready_cycles[0] > 0:
            self.bus.WREADY.value = 1
            self._wready_cycles[0] -= 1
        else:
            self.bus.WREADY.value = 0
            self._wready_cycles[1] -= 1

    async def _write_data(self):
        clock_re = RisingEdge(self.clock)

        while True:
            while True:
                self.bus.WREADY.value = 0
                if self._aw:
                    self._update_wready()
                    break
                await clock_re

            aw = self._aw[0]
            burst_count = aw['burst_length']

            await clock_re

            while True:
                if self.bus.WREADY.value and self.bus.WVALID.value:
                    _burst_diff = aw['burst_length'] - burst_count
                    _st = (aw['_awaddr']
                           + (_burst_diff * aw['bytes_in_beat']))  # start
                    _end = (aw['_awaddr']
                            + ((_burst_diff + 1) * aw['bytes_in_beat']))  # end
                    self._memory[_st:_end] = array.array(
                        'B', self.bus.WDATA.value.to_bytes(
                            byteorder='big' if self.big_endian else 'little'))
                    burst_count -= 1
                    if burst_count == 0:
                        break
                self._update_wready()
                await clock_re

            if hasattr(self.bus, "BREADY") and hasattr(self.bus, "BVALID"):
                self.bus.WREADY.value = 0
                self.bus.BVALID.value = 1
                await clock_re
                while True:
                    if self.bus.BREADY.value:
                        break
                    await clock_re
                self.bus.BVALID.value = 0

            del self._aw[0]

    async def _read_data(self):
        clock_re = RisingEdge(self.clock)

        while True:
            while True:
                await ReadOnly()
                if self.bus.ARVALID.value:
                    break
                await clock_re

            await ReadOnly()
            _araddr = self.bus.ARADDR.value.to_unsigned()
            _arlen = self.bus.ARLEN.value.to_unsigned()
            _arsize = self.bus.ARSIZE.value.to_unsigned()
            _arburst = self.bus.ARBURST.value.to_unsigned()
            _arprot = self.bus.ARPROT.value.to_unsigned()

            burst_length = _arlen + 1
            bytes_in_beat = self._size_to_bytes_in_beat(_arsize)

            if __debug__:
                self.log.debug(
                    "ARADDR  %d\n" % _araddr +
                    "ARLEN   %d\n" % _arlen +
                    "ARSIZE  %d\n" % _arsize +
                    "ARBURST %d\n" % _arburst +
                    "ARPROT %d\n" % _arprot +
                    "BURST_LENGTH %d\n" % burst_length +
                    "Bytes in beat %d\n" % bytes_in_beat)

            burst_count = burst_length

            await clock_re

            while True:
                self.bus.RVALID.value = 1
                await ReadOnly()
                if self.bus.RREADY.value:
                    _burst_diff = burst_length - burst_count
                    _st = _araddr + (_burst_diff * bytes_in_beat)
                    _end = _araddr + ((_burst_diff + 1) * bytes_in_beat)
                    self.bus.RDATA.value = LogicArray.from_bytes(
                        self._memory[_st:_end].tobytes(),
                        byteorder='big' if self.big_endian else 'little')
                    if burst_count == 1:
                        self.bus.RLAST.value = 1
                await clock_re
                burst_count -= 1
                self.bus.RLAST.value = 0
                if burst_count == 0:
                    break
