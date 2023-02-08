#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli
import amaranth.back.verilog

from typing import Optional

from . import axi


class Axi4LiteRegisterBridge(Elaboratable):
    """AXI4-Lite to register bus bridge

    This elaboratable gives a bridge between AXI4-Lite and a custom bus used
    internally for register access. See register.py for the definition of the
    custom bus.

    Parameters
    ----------
    address_width : int
        Address width for the custom register bus, which uses 32-bit word
        addressing. The AXI bus uses byte addressing, so its address width
        is ``address_width + 2``.
    name : Optional[str]
        Used to set the pin names of the AXI4-Lite interface.

    Attributes
    ----------
    axi : AxiInterface
        The AXI4-Lite interface.
    ren : Signal(), out
        Read enable of the register bus.
    rdone : Signal(), in
        Read done of the register bus.
    wstrobe : Signal(4), out
        Write strobe of the register bus.
    address : Signal(address_width), out
        Address of the register bus.
    rdata : Signal(32), in
        Read data of the register bus.
    wdata : Signal(32), out
        Write data of the register bus.
    """
    def __init__(self, address_width: int, name: Optional[str] = None):
        self.aw = address_width

        self.axi = axi.AxiInterface(
            axi.AxiDevice.SUBORDINATE,
            [axi.AxiChannel(axi.AxiDirection.READ, self.aw + 2, 32),
             axi.AxiChannel(axi.AxiDirection.WRITE, self.aw + 2, 32)],
            axi.AxiVersion.AXI4LITE,
            name=name)

        self.ren = Signal()
        self.rdone = Signal()
        self.wstrobe = Signal(4)
        self.wdone = Signal()
        self.address = Signal(self.aw, reset_less=True)
        self.rdata = Signal(32)
        self.wdata = Signal(32)

    def ports(self):
        return self.axi.ports() + [
            self.ren,
            self.rdone,
            self.wstrobe,
            self.wdone,
            self.address,
            self.rdata,
            self.wdata,
        ]

    def elaborate(self, platform):
        m = Module()
        busy = Signal()
        write_transaction = Signal()
        write_preference = Signal()
        start_write = Signal()
        start_write_q = Signal()
        start_read = Signal()
        start_read_q = Signal()
        m.d.comb += [
            self.axi.awready.eq(start_write_q),
            self.axi.wready.eq(start_write_q),
            self.axi.arready.eq(start_read_q),
            self.ren.eq(start_read_q),
            self.wdata.eq(self.axi.wdata),
            start_write.eq(
                ~busy & self.axi.awvalid & self.axi.wvalid &
                (write_preference | ~self.axi.arvalid)),
            start_read.eq(
                ~busy & self.axi.arvalid &
                (~write_preference | ~self.axi.awvalid | ~self.axi.wvalid)),
        ]
        m.d.sync += [
            start_write_q.eq(start_write),
            start_read_q.eq(start_read),
            self.wstrobe.eq(Mux(start_write,
                                self.axi.wstrb,
                                0)),
            self.address.eq(Mux(start_write,
                                self.axi.awaddr >> 2,
                                self.axi.araddr >> 2)),
        ]

        with m.If(self.axi.b_handshake() | self.axi.r_handshake()):
            m.d.sync += busy.eq(0)
        with m.If(start_write | start_read):
            m.d.sync += [
                busy.eq(1),
                write_preference.eq(~write_preference),
            ]

        with m.If(self.wdone):
            m.d.sync += self.axi.bvalid.eq(1)
        with m.If(self.axi.b_handshake()):
            m.d.sync += self.axi.bvalid.eq(0)
        m.d.comb += self.axi.bresp.eq(axi.AxiResp.OKAY)

        with m.If(self.rdone):
            m.d.sync += [
                self.axi.rdata.eq(self.rdata),
                self.axi.rvalid.eq(1),
            ]
        with m.If(self.axi.r_handshake()):
            m.d.sync += self.axi.rvalid.eq(0)
        m.d.comb += self.axi.rresp.eq(axi.AxiResp.OKAY)

        return m


if __name__ == '__main__':
    axi4 = Axi4LiteRegisterBridge(4)
    amaranth.cli.main(
        axi4, ports=axi4.ports())
