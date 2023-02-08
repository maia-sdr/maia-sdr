#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.back.verilog
from amaranth.lib.cdc import FFSynchronizer, PulseSynchronizer

from .fifo import AsyncFifo18_36


class RegisterCDC(Elaboratable):
    """Clock domain crossing for a register bus

    See register.py for the definition of the custom register bus

    Parameters
    ----------
    i_domain : str
        Input clock domain.
    o_domain : str
        Output clock domain.
    address_width: int
        Address width of the register bus.
    width : int
        Data width of the register bus.
    stages : int
        Number of flip-flop synchronization stages used in the CDC.

    Attributes
    ----------
    i_ren : Signal(), in
        Read enable of the input register bus.
    i_rdone : Signal(), out
        Read done of the input register bus.
    i_wstrobe : Signal(width // 8), in
        Write strobe of the input register bus.
    i_address : Signal(address_width), in
        Address of the input register bus.
    i_rdata : Signal(width), out
        Read data of the input register bus.
    i_wdata : Signal(width), in
        Write data of the input register bus.
    o_ren : Signal(), out
        Read enable of the output register bus.
    o_rdone : Signal(), in
        Read done of the output register bus.
    o_wstrobe : Signal(width // 8), out
        Write strobe of the output register bus.
    o_address : Signal(address_width), out
        Address of the output register bus.
    o_rdata : Signal(width), in
        Read data of the output register bus.
    o_wdata : Signal(width), out
        Write data of the output register bus.
    """
    def __init__(self, i_domain: str, o_domain: str, address_width: int,
                 width: int = 32, stages: int = 2):
        self._i_domain = i_domain
        self._o_domain = o_domain
        self.w = width
        self.aw = address_width
        self.nstrobes = width // 8
        self._stages = stages

        self.i_ren = Signal()
        self.i_rdone = Signal()
        self.i_wstrobe = Signal(self.nstrobes)
        self.i_wdone = Signal()
        self.i_address = Signal(self.aw)
        self.i_rdata = Signal(self.w, reset_less=True)
        self.i_wdata = Signal(self.w)

        self.o_ren = Signal()
        self.o_rdone = Signal()
        self.o_wstrobe = Signal(self.nstrobes)
        self.o_wdone = Signal()
        self.o_address = Signal(self.aw, reset_less=True)
        self.o_rdata = Signal(self.w)
        self.o_wdata = Signal(self.w, reset_less=True)

    def ports(self):
        return [
            self.i_ren, self.i_rdone, self.i_wstrobe,
            self.i_wdone, self.i_address, self.i_rdata,
            self.i_wdata,
            self.o_ren, self.o_rdone, self.o_wstrobe,
            self.o_wdone, self.o_address, self.o_rdata,
            self.o_wdata,
        ]

    def elaborate(self, platform):
        m = Module()
        m.submodules.request_sync = request_sync = PulseSynchronizer(
            self._i_domain, self._o_domain, stages=self._stages)
        m.submodules.response_sync = response_sync = PulseSynchronizer(
            self._o_domain, self._i_domain, stages=self._stages)
        cdc_request_data_src = Signal(
            self.nstrobes + self.w + self.aw + 1,
            reset_less=True)
        cdc_request_data_dest = Signal(
            self.nstrobes + self.w + self.aw + 1,
            reset_less=True)
        cdc_response_data_src = Signal(self.w, reset_less=True)
        cdc_response_data_dest = Signal(self.w, reset_less=True)

        # Request
        m.d.comb += request_sync.i.eq(
            self.i_ren | self.i_wstrobe.any())
        with m.If(request_sync.i):
            m.d[self._i_domain] += cdc_request_data_src.eq(
                Cat(self.i_wstrobe, self.i_address, self.i_wdata,
                    self.i_ren)),
        with m.If(request_sync.o):
            m.d[self._o_domain] += cdc_request_data_dest.eq(
                cdc_request_data_src)
        wstrobe = Signal(self.nstrobes)
        ren = Signal()
        m.d.comb += Cat(
            wstrobe, self.o_address, self.o_wdata, ren,
        ).eq(cdc_request_data_dest)
        request_sync_o_q = Signal()
        m.d[self._o_domain] += request_sync_o_q.eq(request_sync.o)
        with m.If(request_sync_o_q):
            m.d.comb += [
                self.o_wstrobe.eq(wstrobe),
                self.o_ren.eq(ren),
            ]

        # Response
        m.d.comb += response_sync.i.eq(self.o_rdone | self.o_wdone)
        with m.If(self.o_rdone):
            m.d[self._o_domain] += cdc_response_data_src.eq(self.o_rdata)
        with m.If(response_sync.o):
            m.d[self._i_domain] += cdc_response_data_dest.eq(
                cdc_response_data_src)
        with m.If(self.i_rdone):
            m.d.comb += self.i_rdata.eq(cdc_response_data_dest)
        m.d[self._i_domain] += [
            self.i_rdone.eq(response_sync.o & cdc_request_data_src[-1]),
            self.i_wdone.eq(response_sync.o & ~cdc_request_data_src[-1]),
        ]

        return m


class RxIQCDC(Elaboratable):
    """CDC for RX IQ data based around the Xilinx FIFO18_36 primitive.

    Parameters
    ----------
    i_domain : str
        Input clock domain.
    o_domain : str
        Output clock domain.
    width : int
        Data width.

    Attributes
    ----------
    re_in : Signal(width), in
        Input real part.
    im_in : Signal(width), in
        Input imaginary part.
    reset : Signal(), in
        FIFO reset. This signal is assumed to be asynchronous with respect to
        the i_domain clock, so it can be driven by the o_domain clock.
    strobe_out : Signal(), out
        Output strobe out. It is asserted when a new sample is presented in
        the output.
    re_out : Signal(width), out
        Output real part.
    im_out : Signal(width), out
        Output imaginary part.
    """
    def __init__(self, i_domain: str, o_domain: str, width: int):
        self._i_domain = i_domain
        self._o_domain = o_domain
        self.w = width
        if self.w > 18:
            raise ValueError('width > 18 not supported')

        # i_domain
        self.re_in = Signal(width)
        self.im_in = Signal(width)

        # o_domain
        self.reset = Signal()
        self.strobe_out = Signal()
        self.re_out = Signal(width)
        self.im_out = Signal(width)

    def elaborate(self, platform):
        m = Module()
        m.submodules.fifo = fifo = AsyncFifo18_36(
            r_domain=self._o_domain, w_domain=self._i_domain)

        # i_domain

        # o_domain -> i_domain reset
        #
        # This synchronizer already provides sufficient delay between the time
        # that the FIFO sees the deassertion of reset and the first time that
        # wren is asserted.
        reset_i = Signal()
        m.submodules.sync_reset = FFSynchronizer(
            self.reset, reset_i, o_domain=self._i_domain, reset=1)

        m.d.comb += [
            fifo.data_in.eq(Cat(self.re_in, self.im_in)),
            fifo.wren.eq(~reset_i),
        ]

        # o_domain
        m.d.comb += [
            self.re_out.eq(fifo.data_out[:self.w]),
            self.im_out.eq(fifo.data_out[self.w:]),
            fifo.rden.eq(~fifo.empty),
            fifo.reset.eq(self.reset),
        ]
        m.d[self._o_domain] += self.strobe_out.eq(fifo.rden)

        return m


def gen_verilog_register():
    m = Module()
    register = ClockDomain()
    m.domains += register
    m.submodules.cdc = cdc = RegisterCDC('sync', 'register', 4)
    with open('register_cdc.v', 'w') as f:
        f.write(amaranth.back.verilog.convert(
            m, ports=cdc.ports() + [register.clk, register.rst],
            emit_src=False))


def gen_verilog_rxiq():
    m = Module()
    internal = ClockDomain()
    m.domains += internal
    m.submodules.cdc = cdc = RxIQCDC('sync', 'internal', 18)
    with open('rxiq_cdc.v', 'w') as f:
        f.write(amaranth.back.verilog.convert(
            m, ports=[
                cdc.re_in, cdc.im_in, cdc.reset, cdc.strobe_out,
                cdc.re_out, cdc.im_out, internal.clk, internal.rst,
            ],
            emit_src=False))


if __name__ == '__main__':
    gen_verilog_register()
    gen_verilog_rxiq()
