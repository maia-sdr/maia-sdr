#
# Copyright (C) 2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli


class Mult2x(Elaboratable):
    """Complex by real multiplier at 2x clock

    A real by complex multiplier that uses a clock at 2x the sample rate in
    order to re-use the same multiplier for the two real multiplications.
    This module is intended to be implemented on a DSP48E1.

    Parameters
    ----------
    domain_2x : str
        Clock domain for the 2x clock. The 1x clock is the 'sync' domain.
    c_width : int
        Width of the complex operand.
    r_width : int
        Width of the real operand.
    truncate : int
        Determines how many bits to truncate in the output.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    common_edge : Signal(), in
        A signal that toggles with the 2x clock and is high immediately
        after the rising edge of the 1x clock.
    clken : Signal(), in
        Clock enable (uses 1x clock).
    re_in : Signal(signed(c_width)), in
        Real part of complex operand.
    im_in : Signal(signed(c_width)), in
        Imaginary part of complex operand.
    real_in : Signal(signed(r_width)), in
        Real operand.
    re_out : Signal(signed(c_width + r_width + 1 - truncate)), out
        Real part of result.
    im_out : Signal(signed(c_width + r_width + 1 - truncate)), out
        Imaginary part of result.
    """
    def __init__(self, domain_2x: str, c_width: int, r_width: int,
                 truncate: int = 0):
        self._2x = domain_2x
        self.cw = c_width
        self.rw = r_width
        self.outw = self.cw + self.rw - truncate
        self.truncate = truncate

        self.common_edge = Signal()
        self.clken = Signal()
        self.re_in = Signal(signed(self.cw))
        self.im_in = Signal(signed(self.cw))
        self.real_in = Signal(signed(self.rw))
        self.re_out = Signal(signed(self.outw),
                             reset_less=True)
        self.im_out = Signal(signed(self.outw),
                             reset_less=True)

    @property
    def delay(self):
        return 3

    def elaborate(self, platform):
        m = Module()
        a1 = Signal(signed(self.cw), reset_less=True)
        a2 = Signal(signed(self.cw), reset_less=True)
        b1 = Signal(signed(self.rw), reset_less=True)
        b2 = Signal(signed(self.rw), reset_less=True)
        mreg = Signal(signed(self.cw + self.rw + 1), reset_less=True)
        p = Signal(signed(len(mreg)), reset_less=True)
        p_trunc = p >> self.truncate
        re_out_reg = Signal(self.outw, reset_less=True)
        with m.If(self.clken):
            m.d[self._2x] += [
                a1.eq(Mux(self.common_edge, self.re_in, self.im_in)),
                a2.eq(a1),
                b1.eq(self.real_in),
                b2.eq(b1),
                mreg.eq(a2 * b2),
                p.eq(mreg),
                re_out_reg.eq(p_trunc)
            ]
            m.d.sync += [
                self.re_out.eq(re_out_reg),
                self.im_out.eq(p_trunc),
            ]
        return m


if __name__ == '__main__':
    mult = Mult2x('clk2x', c_width=12, r_width=18, truncate=19)
    amaranth.cli.main(
        mult, ports=[
            mult.common_edge, mult.clken, mult.re_in, mult.im_in,
            mult.real_in, mult.re_out, mult.im_out])
