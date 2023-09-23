#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.back.verilog
import numpy as np

from .dma import DmaBRAMWrite
from .fft import FFT
from .spectrum_integrator import SpectrumIntegrator


class Spectrometer(Elaboratable):
    """Spectrometer

    This elaboratable uses an FFT and a spectrum integrator to compute
    waterfall data. The data is written to an AXI bus using a DMA
    (DMABramWrite).

    Parameters
    ----------
    dma_base_address : int
        Base address for the DMABramWrite.
    dma_buffers_log2 : int
        Log2 of the number of DMA buffers, used as a parameter for
        the DMABramWrite.
    dma_name : Optional[str]
        DMA name. Used as the name for the DMABramWrite.
    domain_2x : str
        Name of the clock domain of the 2x clock.
    domain_3x : str
        Name of the clock domain of the 2x clock.

    Attributes
    ----------
    strobe_in : Signal(), in
        Strobe in for the input IQ samples.
    common_edge_2x : Signal(), in
        A signal that toggles with the 2x clock and is high immediately
        after the rising edge of the 1x clock.
    common_edge_3x : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock. This is only
        present when cmult3x is enabled.
    re_in : Signal(12), in
        Input samples real part.
    im_in : Signal(12), in
        Input samples imaginary part.
    number_integrations : Signal(10), in
        Sets the number of integrations to use in the integrator.
    peak_detect : Signal(), in
            Enables peak detect mode (instead of average power mode).
    last_buffer : Signal(dma_buffers_log2), out
        Indicates the last buffer to which the DMA has written to.
    interrupt_out : Signal(), out
        Pulsed each time that a DMA transfer finishes.
    """
    def __init__(self, dma_base_address, dma_buffers_log2, dma_name=None,
                 domain_2x='clk2x', domain_3x='clk3x'):
        self._domain_2x = domain_2x
        self._domain_3x = domain_3x
        self.fft_order_log2 = 12
        self.width_in = 12

        self.nint_width = 10
        self.width_integrator_out = 45

        self.dma = DmaBRAMWrite(
            dma_base_address, dma_buffers_log2,
            self.fft_order_log2, name=dma_name)

        self.strobe_in = Signal()
        self.common_edge_2x = Signal()
        self.common_edge_3x = Signal()
        self.re_in = Signal(self.width_in)
        self.im_in = Signal(self.width_in)

        self.number_integrations = Signal(self.nint_width)
        self.peak_detect = Signal()
        self.last_buffer = Signal(dma_buffers_log2)

        self.interrupt_out = Signal()

    def ports(self):
        return self.dma.axi.ports() + [
            self.strobe_in,
            self.common_edge,
            self.re_in,
            self.im_in,
            self.number_integrations,
            self.last_buffer,
            self.interrupt_out,
        ]

    def elaborate(self, platform):
        m = Module()

        truncates = (
            [[0, 0]] * (self.fft_order_log2 // 4)
            + [[1, 1]] * (self.fft_order_log2 // 4))
        m.submodules.fft = fft = FFT(
            self.width_in, self.fft_order_log2, 'R22',
            width_twiddle=16, truncates=truncates,
            use_bram_reg=True, window='blackmanharris',
            cmult3x=True,
            domain_2x=self._domain_2x, domain_3x=self._domain_3x)
        width_fft_out = len(fft.re_out)

        cpwr_truncate = (
            2 * width_fft_out + self.nint_width + 1
            - self.width_integrator_out)
        m.submodules.integrator = integrator = SpectrumIntegrator(
            self._domain_3x, width_fft_out, self.nint_width,
            self.fft_order_log2, cpwr_truncate=cpwr_truncate)

        m.submodules.dma = dma = self.dma

        dma_busy_q = Signal()
        m.d.sync += dma_busy_q.eq(dma.busy)

        m.d.comb += [
            fft.clken.eq(self.strobe_in),
            fft.common_edge_2x.eq(self.common_edge_2x),
            fft.common_edge_3x.eq(self.common_edge_3x),
            fft.re_in.eq(self.re_in),
            fft.im_in.eq(self.im_in),

            integrator.nint.eq(self.number_integrations),
            integrator.peak_detect.eq(self.peak_detect),
            integrator.clken.eq(self.strobe_in),
            integrator.common_edge.eq(self.common_edge_3x),
            integrator.input_last.eq(fft.out_last),
            integrator.re_in.eq(fft.re_out),
            integrator.im_in.eq(fft.im_out),
            integrator.rdaddr.eq(dma.raddr),
            integrator.rden.eq(dma.ren),

            dma.rdata.eq(integrator.rdata),
            dma.start.eq(integrator.done),
            self.last_buffer.eq(dma.last_buffer),

            self.interrupt_out.eq(~dma.busy & dma_busy_q),
        ]
        return m


if __name__ == '__main__':
    spectrometer = Spectrometer(0x1000_0000, 5)
    amaranth.cli.main(
        spectrometer, ports=spectrometer.ports())
