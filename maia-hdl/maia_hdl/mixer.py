#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli

import numpy as np

from .cmult import Cmult3x
from .pluto_platform import PlutoPlatform
from .util import clamp_nbits


class Mixer(Elaboratable):
    """Complex mixer

    This module implements a complex mixer by using a lookup table
    for the complex exponential function and a ``Cmult3x`` for the
    complex multiplication.

    It is assumed that the input amplitude is not greater than
    ``2**(width-1)-1``. Otherwise the output overflows due to the
    rotation.

    Parameters
    ----------
    domain_3x : str
        Name of the clock domain of the 3x clock.
    width : int
        Width of input and output.
    nco_width : int
        Width of the NCO register.
    exp_width : int
        With of the complex exponential function.
    phase_bits : int
        Number of MSBs of the NCO register to consider for
        the lookup table.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    common_edge : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock.
    clken : Signal(), in
        Clock enable.
    frequency : Signal(signed(nco_width)), in
        Mixing frequency. The frequency in cycles per sample can be computed
        as ``frequency / 2**nco_width``. This is the frequency that is shifted
        to baseband by the mixer (the local oscillator frequency is the
        opposite of this frequency).
    re_in : Signal(signed(width)), in
        Real part of the input.
    im_in : Signal(signed(width)), in
        Imaginary part of the input.
    re_out : Signal(signed(width)), in
        Real part of the output.
    im_out : Signal(signed(width)), in
        Imaginary part of the output.
    """
    def __init__(self, domain_3x: str, width: int, *,
                 nco_width: int = 28, exp_width: int = 18,
                 phase_bits: int = 10):
        self._3x = domain_3x
        self.w = width
        self.nco_width = nco_width
        self.exp_width = exp_width
        self.phase_bits = phase_bits

        self.common_edge = Signal()
        self.clken = Signal()
        self.frequency = Signal(signed(self.nco_width))
        self.re_in = Signal(signed(self.w))
        self.im_in = Signal(signed(self.w))
        self.re_out = Signal(signed(self.w), reset_less=True)
        self.im_out = Signal(signed(self.w), reset_less=True)

        # Truncate by exp_width - 2 instead of exp_width - 1
        # because the LSB will be used to round half-up
        self.cmult = Cmult3x(
            self._3x, self.w, self.exp_width, self.exp_width - 2)

    @property
    def delay(self):
        return self.cmult.delay + 1

    def model(self, freq, re_in, im_in):
        assert len(re_in) == len(im_in)
        phase = (np.arange(len(re_in)) * freq) % 2**self.nco_width
        phase = phase // 2**(self.nco_width-self.phase_bits)
        cexp_re, cexp_im = [np.array(a, 'int')[phase] for a in self.cexp()]
        re_in, im_in = [np.array(a, 'int') for a in [re_in, im_in]]
        trunc = self.exp_width - 1
        round_up = 2**(trunc - 1)
        re = clamp_nbits(
            (re_in * cexp_re - im_in * cexp_im + round_up) >> trunc,
            self.w)
        im = clamp_nbits(
            (re_in * cexp_im + im_in * cexp_re + round_up) >> trunc,
            self.w)
        return re, im

    def cexp(self):
        n = 2**self.phase_bits
        z = np.exp(-1j*2*np.pi*np.arange(n)/n)
        scale = 2**(self.exp_width-1) - 1
        re = [int(a) for a in np.round(z.real * scale)]
        im = [int(a) for a in np.round(z.imag * scale)]
        return re, im

    def elaborate(self, platform):
        m = Module()

        m.submodules.cmult = cmult = self.cmult

        # Pack cexp re and im together in the same memory
        mask = 2**self.exp_width - 1
        cexp_packed = [((re & mask) << self.exp_width) | (im & mask)
                       for re, im in zip(*self.cexp())]
        cexp_mem = Memory(
            width=2*self.exp_width,
            depth=len(cexp_packed),
            init=cexp_packed,
            attrs={'ram_style': 'block'},
        )
        # Use transparent=False because read enable is not supported with
        # transparent=True (which is the default).
        m.submodules.rdport = rdport = (
            cexp_mem.read_port(domain='sync', transparent=False))
        # Use BRAM output register. For some reason this isn't working
        # as expected. Vivado retimes the BRAM and feeds its address
        # with the combinational phase + freq instead of with the
        # freq register, so this output registers gets synthesized
        # as flip-flops.
        cexp_mem_out = Signal(2 * self.exp_width, reset_less=True)

        phase = Signal(self.nco_width)

        with m.If(self.clken):
            m.d.sync += [
                cexp_mem_out.eq(rdport.data),
                phase.eq(phase + self.frequency),
                # round half-up
                self.re_out.eq(cmult.re_out[0] + cmult.re_out[1:]),
                self.im_out.eq(cmult.im_out[0] + cmult.im_out[1:]),
            ]
        m.d.comb += [
            rdport.en.eq(self.clken),
            rdport.addr.eq(phase[-self.phase_bits:]),
            cmult.common_edge.eq(self.common_edge),
            cmult.clken.eq(self.clken),
            cmult.re_a.eq(self.re_in),
            cmult.im_a.eq(self.im_in),
            cmult.re_b.eq(cexp_mem_out[-self.exp_width:]),
            cmult.im_b.eq(cexp_mem_out[:self.exp_width]),
        ]
        return m


if __name__ == '__main__':
    mixer = Mixer('clk3x', 16)
    amaranth.cli.main(mixer, ports=[
        mixer.common_edge, mixer.clken,
        mixer.frequency,
        mixer.re_in, mixer.im_in,
        mixer.re_out, mixer.im_out,
    ], platform=PlutoPlatform())
