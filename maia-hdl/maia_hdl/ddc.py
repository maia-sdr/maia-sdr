#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli

from .buffer import SkidBuffer
from .fir import FIRDecimator3Stage
from .mixer import Mixer


class DDC(Elaboratable):
    """Decimator with 3 FIR stages.

    This module implements a DDC that uses a Mixer and a FIRDecimator3Stage.
    The FIR decimator runs with a 3x clock, the Mixer uses both a 1x clock and
    a 3x clock, and the input and output use a 1x clock.

    Parameters
    ----------
    domain_3x : str
        Name of the clock domain of the 3x clock.
    in_width : int
        Width of input IQ samples.
    out_width : List[int]
        Output width of each FIR stage.
    nco_width : int
        Width of the mixer NCO.
    coeff_width : int
        FIR coefficients width (for all stages).
    decim_width : List[int]
        Width of ``decimation`` input for each stage.
    oper_width : List[int]
        Width of ``operations_minus_one`` input for each stage.
    macc_trunc : List[int]
        Truncation length for the output of each stage.

    Attributes
    ----------
    common_edge : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock.
    enable_input : Signal(), in
        If this signal is high, the DDC reads input whenever the strobe_in
        is pulsed. If this signal is low, the DDC ignores strobe_in, drops
        input and does not perform any work, so it is effectively disabled.
    frequency : Signal(signed(nco_width)), in
        Mixing frequency. The frequency in cycles per sample can be computed
        as ``frequency / 2**nco_width``. This is the frequency that is shifted
        to baseband by the mixer (the local oscillator frequency is the
        opposite of this frequency).
    coeff_waddr : Signal(10), in
        Coefficient write address. The 2 MSBs of the address are used to
        select the address space of each stage.
    coeff_wren : Signal(), in
        Coefficient write enable.
    coeff_wdata : Signal(coeff_width), in
        Coefficient write data.
    decimation1 : Signal(decim_width[0]), in
        Decimation factor for stage 1.
    decimation2 : Signal(decim_width[1]), in
        Decimation factor for stage 2.
    decimation3 : Signal(decim_width[2]), in
        Decimation factor for stage 3.
    bypass2 : Signal(), in
        Enables bypass of stage 2.
    bypass3 : Signal(), in
        Enables bypass of stage 3.
    operations_minus_one1 : Signal(oper_width[0]), in
        Number of operations to perform minus one for stage 1. See ``FIR4DSP``.
    operations_minus_one2 : Signal(oper_width[1]), in
        Number of operations to perform minus one for stage 2. See ``FIR2DSP``.
    operations_minus_one3 : Signal(oper_width[1]), in
        Number of operations to perform minus one for stage 3. See ``FIR4DSP``.
    odd_operations1 : Signal(), in
        Disable the MACC1 in the last operation of stage 1 in order to achieve
        an odd number of multiplies. See ``FIR4DSP``.
    odd_operations3 : Signal(), in
        Disable the MACC1 in the last operation of stage 1 in order to achieve
        an odd number of multiplies. See ``FIR4DSP``.
    re_in : Signal(signed(in_width)), in
        Input real part.
    im_in : Signal(signed(in_width)), in
        Input imaginary part.
    strobe_in : Signal(), in
        Input strobe.
    in_ready : Signal(), out
        Input ready (uses AXI-Stream handshaking).
    re_out : Signal(signed(out_width[-1])), out
        Output real part.
    im_out : Signal(signed(out_width[-1])), out
        Output imaginary part.
    strobe_out : Signal(), out
        Output strobe. It is asserted in the clock cycle when the output
        changes. The output is kept constant until the next time that
        ``strobe_out`` is asserted.

    """
    def __init__(self, domain_3x: str, *,
                 in_width: int = 12,
                 out_width: list[int] = [16]*3,
                 nco_width: int = 28,
                 coeff_width: int = 18,
                 decim_width: list[int] = [7, 6, 7],
                 oper_width: list[int] = [7, 6, 7],
                 macc_trunc: list[int] = [17, 18, 18]):
        self._3x = domain_3x
        self.iw = in_width
        self.ow = out_width
        self.nco_width = nco_width
        self.coeff_width = coeff_width
        self.decim_width = decim_width
        self.oper_width = oper_width
        self.macc_trunc = macc_trunc

        self.common_edge = Signal()
        self.enable_input = Signal()

        self.frequency = Signal(nco_width)

        self.coeff_waddr = Signal(10)
        self.coeff_wren = Signal()
        self.coeff_wdata = Signal(coeff_width)
        self.decimation1 = Signal(decim_width[0])
        self.decimation2 = Signal(decim_width[1])
        self.decimation3 = Signal(decim_width[2])
        self.bypass2 = Signal()
        self.bypass3 = Signal()
        self.operations_minus_one1 = Signal(oper_width[0])
        self.operations_minus_one2 = Signal(oper_width[1])
        self.operations_minus_one3 = Signal(oper_width[2])
        self.odd_operations1 = Signal()
        self.odd_operations3 = Signal()

        self.re_in = Signal(signed(self.iw))
        self.im_in = Signal(signed(self.iw))
        self.strobe_in = Signal()

        self.re_out = Signal(signed(self.ow[-1]), reset_less=True)
        self.im_out = Signal(signed(self.ow[-1]), reset_less=True)
        self.strobe_out = Signal()

    def elaborate(self, platform):
        m = Module()

        m.submodules.mixer = mixer = Mixer(
            self._3x, self.iw, nco_width=self.nco_width)

        m.d.comb += [
            mixer.common_edge.eq(self.common_edge),
            mixer.clken.eq(self.enable_input & self.strobe_in),
            mixer.frequency.eq(self.frequency),
            mixer.re_in.eq(self.re_in),
            mixer.im_in.eq(self.im_in),
        ]

        clk3x_renamer = DomainRenamer({'sync': self._3x})
        m.submodules.decimator = decimator = clk3x_renamer(
            FIRDecimator3Stage(
                in_width=self.iw, out_width=self.ow,
                coeff_width=self.coeff_width, decim_width=self.decim_width,
                oper_width=self.oper_width, macc_trunc=self.macc_trunc))
        for port in ['coeff_waddr', 'coeff_wren', 'coeff_wdata', 'decimation1',
                     'decimation2', 'decimation3', 'bypass2', 'bypass3',
                     'operations_minus_one1', 'operations_minus_one2',
                     'operations_minus_one3', 'odd_operations1',
                     'odd_operations3']:
            m.d.comb += getattr(decimator, port).eq(getattr(self, port))

        # input CDC: sync -> 3x
        m.submodules.inbuff = inbuff = clk3x_renamer(SkidBuffer(2 * self.iw))
        m.d.comb += [
            inbuff.in_data.eq(Cat(mixer.re_out, mixer.im_out)),
            inbuff.in_valid.eq(
                self.enable_input & self.common_edge & self.strobe_in),
            decimator.re_in.eq(inbuff.out_data[:self.iw]),
            decimator.im_in.eq(inbuff.out_data[-self.iw:]),
            decimator.in_valid.eq(inbuff.out_valid),
            inbuff.out_ready.eq(decimator.in_ready),
        ]

        # output CDC: 3x -> sync
        out_toggle = Signal()
        with m.If(decimator.strobe_out):
            m.d[self._3x] += out_toggle.eq(~out_toggle)

        out_toggle_q = Signal()
        m.d.sync += [
            out_toggle_q.eq(out_toggle),
            self.strobe_out.eq(out_toggle ^ out_toggle_q),
        ]
        with m.If(out_toggle ^ out_toggle_q):
            m.d.sync += [
                self.re_out.eq(decimator.re_out),
                self.im_out.eq(decimator.im_out),
            ]

        return m


if __name__ == '__main__':
    ddc = DDC('clk3x')
    amaranth.cli.main(
        ddc, ports=[
            ddc.common_edge,
            ddc.coeff_waddr, ddc.coeff_wren, ddc.coeff_wdata,
            ddc.decimation1, ddc.decimation2, ddc.decimation3,
            ddc.bypass2, ddc.bypass3,
            ddc.operations_minus_one1, ddc.operations_minus_one2,
            ddc.operations_minus_one3,
            ddc.odd_operations1, ddc.odd_operations3,
            ddc.re_in, ddc.im_in, ddc.strobe_in,
            ddc.re_out, ddc.im_out, ddc.strobe_out])
