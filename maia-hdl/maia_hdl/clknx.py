#
# Copyright (C) 2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *


class ClkNxCommonEdge(Elaboratable):
    """Common edge signal generator for an Nx clock setup.

    This module generates an output that is synchronous to the Nx clock and is
    asserted in those clock cycles in which the edges of the 1x and Nx clocks
    match.

    Parameters
    ----------
    domain_1x : str
        Domain of the 1x clock.
    domain_nx : str
        Domain of the Nx clock.
    n: int
        Frequency ratio between the Nx clock and the 1x clock.

    Attributes
    ----------
    common_edge : Signal(), out
        Output common edge signal.
    """
    def __init__(self, domain_1x: str, domain_nx: str, n: int):
        self._1x = domain_1x
        self._nx = domain_nx
        self._n = n

        self.common_edge = Signal(reset_less=True)

    def elaborate(self, platform):
        m = Module()
        toggle_1x = Signal(reset_less=True)
        m.d[self._1x] += toggle_1x.eq(~toggle_1x)
        toggle_1x_q = Signal(reset_less=True)
        m.d[self._nx] += toggle_1x_q.eq(toggle_1x)
        # This is the output we want, but it is combinational.
        pulse = toggle_1x ^ toggle_1x_q
        # To have a registered output, we delay it N cycles, getting the same
        # output, but coming out of a register.
        pulse_del = Signal(self._n, reset_less=True)
        m.d[self._nx] += pulse_del.eq(Cat(pulse, pulse_del[:-1]))
        m.d.comb += self.common_edge.eq(pulse_del[-1])
        return m
