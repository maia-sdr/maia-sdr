#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *

from enum import Enum
import enum
from math import log2
from typing import List, Optional


class AxiDevice(Enum):
    MANAGER = enum.auto()
    SUBORDINATE = enum.auto()


class AxiDirection(Enum):
    READ = enum.auto()
    WRITE = enum.auto()


class AxiChannel:
    def __init__(self, direction: AxiDirection, address_bits: int,
                 data_bits: int, user_req_width: int = 0,
                 user_data_width: int = 0, user_resp_width: int = 0,
                 id_bits: int = 0):
        if data_bits % 8 != 0 or data_bits < 8 or data_bits > 1024:
            raise ValueError(f'invalid data_bits: {data_bits}')
        self.direction = direction
        self.address_bits = address_bits
        self.data_bits = data_bits
        self.id_bits = id_bits
        self.user_req_width = user_req_width
        self.user_data_width = user_data_width
        self.user_resp_width = user_resp_width

    def has_user_signals(self):
        return (self.user_req_width
                or self.user_data_width
                or self.user_resp_width)


class AxiVersion(Enum):
    AXI3 = enum.auto()
    AXI4 = enum.auto()
    AXI4LITE = enum.auto()


class AxiBurst(Enum):
    FIXED = 0b00
    INCR = 0b01
    WRAP = 0b10


class AxiResp(Enum):
    OKAY = 0b00
    EXOKAY = 0b01
    SLVERR = 0b10
    DECERR = 0b11


class AxiInterface:
    def __init__(self, device: AxiDevice, channels: List[AxiChannel],
                 version: AxiVersion, name: Optional[str] = None):
        self.name = name
        self.version = version
        self.device = device
        self.channels = self._check_channels(channels)
        self._ports = ports = []

        # Write channel
        try:
            ch = self.channels[AxiDirection.WRITE]
        except KeyError:
            pass
        else:
            if self.version != AxiVersion.AXI4LITE:
                self.awid = Signal(ch.id_bits, reset=0, reset_less=True,
                                   name=self._pin_name('awid'))
                ports.append(self.awid)
            self.awaddr = Signal(ch.address_bits, reset_less=True,
                                 name=self._pin_name('awaddr'))
            ports.append(self.awaddr)
            if self.version != AxiVersion.AXI4LITE:
                self.awlen = Signal(
                    4 if self.version == AxiVersion.AXI3 else 8, reset=0,
                    reset_less=True, name=self._pin_name('awlen'))
                ports.append(self.awlen)
                self.awsize = Signal(
                    3, reset=int(log2(ch.data_bits // 8)), reset_less=True,
                    name=self._pin_name('awsize'))
                ports.append(self.awsize)
                self.awburst = Signal(2, reset=0b01, reset_less=True,
                                      name=self._pin_name('awburst'))
                ports.append(self.awburst)
                self.awlock = Signal(
                    2 if self.version == AxiVersion.AXI3 else 1, reset=0,
                    reset_less=True, name=self._pin_name('awlock'))
                ports.append(self.awlock)
                self.awcache = Signal(4, reset=0b0000, reset_less=True,
                                      name=self._pin_name('awcache'))
                ports.append(self.awcache)
            self.awprot = Signal(3, reset_less=True,
                                 name=self._pin_name('awprot'))
            ports.append(self.awprot)
            if self.version not in [AxiVersion.AXI3, AxiVersion.AXI4LITE]:
                self.awqos = Signal(4, reset=0b0000, reset_less=True,
                                    name=self._pin_name('awqos'))
                ports.append(self.awqos)
                self.awregion = Signal(4, reset=0, reset_less=True,
                                       name=self._pin_name('awregion'))
                ports.append(self.awregion)
                self.awuser = Signal(ch.user_req_width, reset_less=True,
                                     name=self._pin_name('awuser'))
                ports.append(self.awuser)
            self.awvalid = Signal(name=self._pin_name('awvalid'))
            ports.append(self.awvalid)
            self.awready = Signal(name=self._pin_name('awready'))
            ports.append(self.awready)

            if self.version == AxiVersion.AXI3:
                self.wid = Signal(ch.id_bits, reset=0, reset_less=True,
                                  name=self._pin_name('wid'))
                ports.append(self.wid)
            self.wdata = Signal(ch.data_bits, reset_less=True,
                                name=self._pin_name('wdata'))
            ports.append(self.wdata)
            self.wstrb = Signal(ch.data_bits // 8, reset=-1, reset_less=True,
                                name=self._pin_name('wstrb'))
            ports.append(self.wstrb)
            if self.version != AxiVersion.AXI4LITE:
                self.wlast = Signal(reset_less=True,
                                    name=self._pin_name('wlast'))
                ports.append(self.wlast)
            if self.version not in [AxiVersion.AXI3, AxiVersion.AXI4LITE]:
                self.wuser = Signal(ch.user_data_width, reset_less=True,
                                    name=self._pin_name('wuser'))
                ports.append(self.wuser)
            self.wvalid = Signal(name=self._pin_name('wvalid'))
            ports.append(self.wvalid)
            self.wready = Signal(name=self._pin_name('wready'))
            ports.append(self.wready)

            if self.version != AxiVersion.AXI4LITE:
                self.bid = Signal(ch.id_bits, reset_less=True,
                                  name=self._pin_name('bid'))
                ports.append(self.bid)
            self.bresp = Signal(2, reset=0b00, reset_less=True,
                                name=self._pin_name('bresp'))
            ports.append(self.bresp)
            if self.version not in [AxiVersion.AXI3, AxiVersion.AXI4LITE]:
                self.buser = Signal(self.user_resp_width, reset_less=True,
                                    name=self._pin_name('buser'))
                ports.append(self.buser)
            self.bvalid = Signal(name=self._pin_name('bvalid'))
            ports.append(self.bvalid)
            self.bready = Signal(name=self._pin_name('bready'))
            ports.append(self.bready)

        # Read channel
        try:
            ch = self.channels[AxiDirection.READ]
        except KeyError:
            pass
        else:
            if self.version != AxiVersion.AXI4LITE:
                self.arid = Signal(ch.id_bits, reset=0, reset_less=True,
                                   name=self._pin_name('arid'))
                ports.append(self.arid)
            self.araddr = Signal(ch.address_bits, reset_less=True,
                                 name=self._pin_name('araddr'))
            ports.append(self.araddr)
            if self.version != AxiVersion.AXI4LITE:
                self.arlen = Signal(
                    4 if self.version == AxiVersion.AXI3 else 8, reset=0,
                    reset_less=True, name=self._pin_name('arlen'))
                ports.append(self.arlen)
                self.arsize = Signal(3, reset=int(log2(ch.data_bits // 8)),
                                     reset_less=True,
                                     name=self._pin_name('arsize'))
                ports.append(self.arsize)
                self.arburst = Signal(2, reset_less=True,
                                      name=self._pin_name('arburst'))
                ports.append(self.arburst)
                self.arlock = Signal(2 if self.version == AxiVersion.AXI3
                                     else 1,
                                     reset=0, reset_less=True,
                                     name=self._pin_name('arlock'))
                ports.append(self.arlock)
                self.arcache = Signal(4, reset=0b0000, reset_less=True,
                                      name=self._pin_name('arcache'))
                ports.append(self.arcache)
            self.arprot = Signal(3, reset_less=True,
                                 name=self._pin_name('arprot'))
            ports.append(self.arprot)
            if self.version not in [AxiVersion.AXI3, AxiVersion.AXI4LITE]:
                self.arqos = Signal(4, reset=0b0000, reset_less=True,
                                    name=self._pin_name('arqos'))
                ports.append(self.arqos)
                self.arregion = Signal(4, reset=0x0, reset_less=True,
                                       name=self._pin_name('arregion'))
                ports.append(self.arregion)
                self.aruser = Signal(ch.user_req_width, reset_less=True,
                                     name=self._pin_name('aruser'))
                ports.append(self.aruser)
            self.arvalid = Signal(name=self._pin_name('arvalid'))
            ports.append(self.arvalid)
            self.arready = Signal(name=self._pin_name('arready'))
            ports.append(self.arready)

            if self.version != AxiVersion.AXI4LITE:
                self.rid = Signal(ch.id_bits, reset_less=True,
                                  name=self._pin_name('rid'))
                ports.append(self.rid)
            self.rdata = Signal(ch.data_bits, reset_less=True,
                                name=self._pin_name('rdata'))
            ports.append(self.rdata)
            self.rresp = Signal(2, reset=0b00, reset_less=True,
                                name=self._pin_name('rresp'))
            ports.append(self.rresp)
            if self.version != AxiVersion.AXI4LITE:
                self.rlast = Signal(reset_less=True,
                                    name=self._pin_name('rlast'))
                ports.append(self.rlast)
                if self.version != AxiVersion.AXI3:
                    self.ruser = Signal(ch.user_data_width
                                        + ch.user_resp_width,
                                        reset_less=True,
                                        name=self._pin_name('ruser'))
                    ports.append(self.rlast)
            self.rvalid = Signal(name=self._pin_name('rvalid'))
            ports.append(self.rvalid)
            self.rready = Signal(name=self._pin_name('rready'))
            ports.append(self.rready)

    def _pin_name(self, pin):
        if self.name is None:
            return pin
        return f'{self.name}_{pin}'

    def _check_channels(self, channels):
        dupe_channels = len({ch.direction for ch in channels}) != len(channels)
        if not channels or len(channels) > 2 or dupe_channels:
            raise ValueError('invalid channel list')
        if self.version == AxiVersion.AXI3:
            for ch in channels:
                if ch.has_user_signals():
                    raise ValueError('user signals are not supported in AXI3')
        return {ch.direction: ch for ch in channels}

    def ports(self):
        return self._ports

    def aw_handshake(self):
        return self.awvalid & self.awready

    def w_handshake(self):
        return self.wvalid & self.wready

    def b_handshake(self):
        return self.bvalid & self.bready

    def ar_handshake(self):
        return self.arvalid & self.arready

    def r_handshake(self):
        return self.rvalid & self.rready
