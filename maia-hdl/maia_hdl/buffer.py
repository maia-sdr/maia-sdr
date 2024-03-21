#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli


class SkidBuffer(Elaboratable):
    """Skid buffer.

    This module implements a Skid buffer (FIFO with depth 2). The input and
    output hanshaking is as in AXI-Stream.

    Parameters
    ----------
    width : int
        Width of the buffer.

    Attributes
    ----------
    in_data : Signal(width), in
        Input signal.
    in_valid : Signal(), in
        Input valid.
    in_ready : Signal(), out
        Input ready.
    out_data : Signal(width), out
        Output signal.
    out_valid : Signal(), out
        Output valid.
    out_ready : Signal(), in
        Output ready.
    """
    def __init__(self, width=1):
        self.w = width

        self.in_data = Signal(width)
        self.in_valid = Signal()
        self.in_ready = Signal()
        self.out_data = Signal(width, reset_less=True)
        self.out_valid = Signal()
        self.out_ready = Signal()

    def elaborate(self, platform):
        m = Module()

        skid_data = Signal(self.w, reset_less=True)
        skid_valid = Signal()

        m.d.comb += self.in_ready.eq(~skid_valid)

        with m.If(self.out_valid & self.out_ready):
            m.d.sync += [
                skid_data.eq(self.in_data),
                skid_valid.eq(self.in_valid & skid_valid),
                self.out_data.eq(Mux(skid_valid, skid_data, self.in_data)),
                self.out_valid.eq(self.in_valid | skid_valid),
            ]
        with m.Elif(self.in_valid & self.in_ready):
            m.d.sync += [
                skid_data.eq(self.in_data),
                skid_valid.eq(self.out_valid),
                self.out_valid.eq(1),
            ]
            with m.If(~self.out_valid):
                m.d.sync += [
                    self.out_data.eq(self.in_data),
                ]

        return m
