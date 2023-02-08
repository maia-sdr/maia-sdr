#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *


class PulseStretcher(Elaboratable):
    """Pulse stretcher

    This module transforms single cycle pulses at its input in multiple-cycle
    pulses at its output. The output pulse length must be a power of two.

    Parameters
    ----------
    pulse_len_log2 : int
        The log2 of the output pulse length.

    Attributes
    ----------
    pulse_in : Signal(), in
        Pulse input.
    pulse_out : Signal(), out
        Pulse output.
    """
    def __init__(self, pulse_len_log2=3):
        self.pulse_len_log2 = pulse_len_log2

        self.pulse_in = Signal()
        self.pulse_out = Signal()

    def elaborate(self, platform):
        m = Module()
        counter = Signal(self.pulse_len_log2, reset_less=True)
        counter_next = Signal(self.pulse_len_log2 + 1)
        carry = counter_next[-1]
        m.d.comb += counter_next.eq(counter + 1)
        with m.If(self.pulse_out):
            m.d.sync += counter.eq(counter_next)
        with m.If(carry):
            m.d.sync += self.pulse_out.eq(0)
        with m.If(self.pulse_in):
            m.d.sync += [
                self.pulse_out.eq(1),
                counter.eq(0),
            ]
        return m
