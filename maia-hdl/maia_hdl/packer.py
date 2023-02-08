#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli


class Pack12IQto32(Elaboratable):
    """Pack 12-bit IQ to 32-bit words.

    The data is packed such that if the 32-bit words are written to memory as
    little-endian, then the samples can be found in memory as interleaved
    big-endian 12-bit I and Q samples.

    Attributes
    ----------
    enable : Signal(), in
        Asserted to enable the packer. When this is low, the packer is in its
        reset state.
    re_in : Signal(12), in
        Input real part.
    im_in : Signal(12), in
        Input imaginary part.
    strobe_in : Signal(), in
        Asserted to indicate that a valid sample is presented at the input.
    data_out : Signal(32), out
        Output 32-bit word.
    strobe_out : Signal(32), out
        Asserted when a valid word is presented at the output.
    """
    def __init__(self):
        self.enable = Signal(1)
        self.re_in = Signal(12)
        self.im_in = Signal(12)
        self.strobe_in = Signal()
        self.out = Signal(32)
        self.strobe_out = Signal()

    def elaborate(self, platform):
        m = Module()
        re_q = Signal(12, reset_less=True)
        im_q = Signal(12, reset_less=True)
        byte0_q = re_q[4:]
        byte1_q = Cat(im_q[-4:], re_q[:4])
        byte2_q = im_q[:8]
        byte0 = self.re_in[4:]
        byte1 = Cat(self.im_in[-4:], self.re_in[:4])
        byte2 = self.im_in[:8]
        for x in [byte0_q, byte1_q, byte2_q,
                  byte0, byte1, byte2]:
            assert len(x) == 8
        with m.If(self.strobe_in):
            m.d.sync += [re_q.eq(self.re_in),
                         im_q.eq(self.im_in)]
        with m.FSM(reset='A'):
            with m.State('A'):
                with m.If(self.strobe_in):
                    m.next = 'B'
                with m.If(~self.enable):
                    m.next = 'A'
            with m.State('B'):
                m.d.comb += self.out.eq(Cat(byte0_q, byte1_q, byte2_q, byte0))
                with m.If(self.strobe_in):
                    m.next = 'C'
                    m.d.comb += self.strobe_out.eq(1)
                with m.If(~self.enable):
                    m.next = 'A'
            with m.State('C'):
                m.d.comb += self.out.eq(Cat(byte1_q, byte2_q, byte0, byte1))
                with m.If(self.strobe_in):
                    m.next = 'D'
                    m.d.comb += self.strobe_out.eq(1)
                with m.If(~self.enable):
                    m.next = 'A'
            with m.State('D'):
                m.d.comb += self.out.eq(Cat(byte2_q, byte0, byte1, byte2))
                with m.If(self.strobe_in):
                    m.next = 'A'
                    m.d.comb += self.strobe_out.eq(1)
                with m.If(~self.enable):
                    m.next = 'A'
        return m


class Pack8IQto32(Elaboratable):
    """Pack 8-bit IQ to 32-bit words.

    The data is packed such that if the 32-bit words are written to memory as
    little-endian, then the samples can be found in memory as interleaved
    8-bit I and Q samples.

    Attributes
    ----------
    enable : Signal(), in
        Asserted to enable the packer. When this is low, the packer is in its
        reset state.
    re_in : Signal(8), in
        Input real part.
    im_in : Signal(8), in
        Input imaginary part.
    strobe_in : Signal(), in
        Asserted to indicate that a valid sample is presented at the input.
    data_out : Signal(32), out
        Output 32-bit word.
    strobe_out : Signal(32), out
        Asserted when a valid word is presented at the output.
    """
    def __init__(self):
        self.enable = Signal()
        self.re_in = Signal(8)
        self.im_in = Signal(8)
        self.strobe_in = Signal()
        self.out = Signal(32)
        self.strobe_out = Signal()

    def elaborate(self, platform):
        m = Module()
        re_q = Signal(8, reset_less=True)
        im_q = Signal(8, reset_less=True)
        full = Signal()
        with m.If(self.strobe_in):
            m.d.sync += [
                full.eq(~full),
                re_q.eq(self.re_in),
                im_q.eq(self.im_in),
            ]
        with m.If(~self.enable):
            m.d.sync += full.eq(0)
        m.d.comb += [
            self.out.eq(Cat(re_q, im_q, self.re_in, self.im_in)),
            self.strobe_out.eq(self.strobe_in & full),
        ]
        return m


class PackFifoTwice(Elaboratable):
    """Pack FIFO of width w to stream of width 2w.

    This packer reads two elements from the FIFO and packs them into
    a word of width 2w in an output stream. The earlier element of
    each pair is placed in the LSB part of the 2w word.

    Parameters
    ----------
    width_in : int
        Width of the FIFO.

    Attributes
    ----------
    enable : Signal(), in
        Asserted to enable the packer. When this is low, the packer is in its
        reset state.
    fifo_data : Signal(width_in), in
        FIFO data output.
    rden : Signal(), out
        Read enable for the FIFO. It is assumed that the FIFO has one cycle of
        read latency.
    empty : Signal(), in
        Empty indicator of the FIFO. FIFO reads are only performed when the
        FIFO is not empty.
    out_data : Signal(2 * width_in), out
        Stream output data.
    out_valid : Signal(), out
        Stream output valid. The semantics are as in AXI4-Stream.
    out_ready : Signal(), in
        Stream output ready. The semantics are as in AXI4-Stream.
    """
    def __init__(self, width_in=32):
        self.w = width_in
        self.enable = Signal()
        # FIFO interface
        self.fifo_data = Signal(self.w)
        self.rden = Signal()
        self.empty = Signal()
        # stream interface output
        self.out_data = Signal(2 * self.w)
        self.out_valid = Signal()
        self.out_ready = Signal()

    def elaborate(self, platform):
        m = Module()
        fifo_data_q = Signal(self.w, reset_less=True)
        has_one = Signal()
        has_two = Signal()
        with m.If(self.rden):
            m.d.sync += fifo_data_q.eq(self.fifo_data)
        out_tx = Signal()
        m.d.comb += [
            out_tx.eq(self.out_ready & self.out_valid),
            self.out_data.eq(Cat(fifo_data_q, self.fifo_data)),
            self.out_valid.eq(has_two),
            # We never read from the FIFO when disabled because the FIFO rden
            # should be low for a few cycles before the FIFO reset is asserted.
            self.rden.eq(~self.empty & self.enable & (out_tx | ~has_two)),
        ]
        with m.If(self.rden):
            # input but no output
            m.d.sync += [
                has_one.eq(~has_one),
                has_two.eq(has_one),
            ]
        with m.If(out_tx):
            # output (either with simultaneous input or not)
            m.d.sync += [
                has_one.eq(self.rden),
                has_two.eq(0),
            ]
        with m.If(~self.enable):
            # clear
            m.d.sync += [
                has_one.eq(0),
                has_two.eq(0),
            ]
        return m
