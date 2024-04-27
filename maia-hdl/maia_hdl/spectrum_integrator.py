#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli

import numpy as np

from .cpwr import CpwrPeak
from .floating_point import IQToFloatingPoint, MakeCommonExponent
from .pluto_platform import PlutoPlatform
from .util import bit_invert


class SpectrumIntegrator(Elaboratable):
    """Spectrum integrator

    This module uses a CpwrPeak and a BRAM to compute the integrated power or
    the peak power detect at the output of an FFT. The number of integrations
    (or samples to cover in the maximum, in peak detect mode) can be changed at
    runtime.

    A ping-pong approach with the BRAMs is used so that a reader module can
    read the previous integration while the current integration is being
    computed.

    Floating-point representation as defined in the floating_point module is
    used to increase the dynamic range. For instance, for a 22-bit input, the
    input input is converted to a 18-bit mantissa and 3-bit exponent (with a
    maximum value of 4). The modulus squared of the mantissa is calculated,
    yielding 37 bits. A 47-bit mantissa accumulator is used to allow up to 1024
    integrations. Together with each 47-mantissa, the 3-bit exponent for each
    accumulator is stored in the BRAM.

    Parameters
    ----------
    domain_3x : str
        Name of the clock domain of the 3x clock.
    input_width : int
        Width of the input samples.
    input_fp_width : int
        Mantissa width for the input converted to floating point.
    nint_width : int
        Width of the input that indicates the number of integrations.
    fft_order_log2 : int
        Determines the FFT size, as ``2**fft_order_log2``.

    Attributes
    ----------
    nint : Signal(nint_width), in
        Number of integrations to perform. This signal is only latched
        after the current integration has finished.
    abort : Signal(), in
        Abort the current integration before reaching the number of
        integrations. This mechanism is intended to be used when changing
        the input sampling rate of the spectrum integrator. If an integration
        with a large nint is underway and the sampling rate is reduced by
        a large factor, the integration might take a long time to complete.
        When the abort signal is pulsed for at least one cycle, the current
        integration will finish when the end of the current FFT is reached.
    peak_detect : Signal(), in
        Enables peak detect mode (instead of average power mode).
    clken : Signal(), in
        Clock enable.
    common_edge : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock.
    input_last : Signal(), in
        This signal should be asserted when the last sample of the FFT
        vector is presented at the input.
    re_in : Signal(signed(input_width)), in
        Real part of the input sample.
    im_in : Signal(signed(input_width)), in
        Imaginary part of the input sample.
    done : Signal(), out
        This signal is pulsed for one clock cycle whenever an integration
        is finished.
    rdaddr : Signal(fft_order_log2), in
        Read address for the BRAM that contains the previous integration.
    rdata_value : Signal(sum_width), out
        Value (mantissa) part of the read data for the BRAM that contains the
        previous integration.
    rdata_exponent : Signal(exponent_width), out
        Exponent part of the read data for the BRAM that contains the previous
        integration.
    rden : Signal(), in
        Read enable for the BRAM that contains the previous integration.
    """
    def __init__(self, domain_3x, input_width, input_fp_width,
                 nint_width, fft_order_log2):
        self.w = input_width
        self.fw = input_fp_width
        self.nw = nint_width
        # Here + 1 accounts for the addition of the real and imaginary parts.
        self.sumw = 2*self.fw + 1 + nint_width
        self.order_log2 = fft_order_log2

        self.to_fp = IQToFloatingPoint(self.w, self.fw)
        self.ew = len(self.to_fp.exponent_out)
        self.common_exp = MakeCommonExponent(
            self.fw, self.sumw, self.ew, self.w - self.fw,
            a_complex=True, b_power=True, b_signed=False)
        self.cpwr = CpwrPeak(domain_3x, self.fw, self.sumw)

        self.nint = Signal(nint_width)
        self.abort = Signal()
        self.peak_detect = Signal()
        self.clken = Signal()
        self.common_edge = Signal()
        self.input_last = Signal()
        self.re_in = Signal(signed(input_width))
        self.im_in = Signal(signed(input_width))
        self.done = Signal()
        self.rdaddr = Signal(fft_order_log2)
        self.rdata_value = Signal(self.sumw)
        self.rdata_exponent = Signal(self.ew)
        self.rden = Signal()

    @property
    def model_vlen(self, nint):
        return 2**self.order_log2 * nint

    def model(self, nint, re_in, im_in, peak_detect):
        re_in, im_in = (
            np.array(x, 'int').reshape(-1, nint, 2**self.order_log2)
            for x in [re_in, im_in])
        re_in, im_in, exp_in = self.to_fp.model(re_in, im_in)
        acc, acc_exp = (np.zeros((re_in.shape[0], 2**self.order_log2), 'int')
                        for _ in range(2))
        for j in range(nint):
            re_in_c, im_in_c, acc_c, _, exp_c = self.common_exp.model(
                re_in[:, j], im_in[:, j], exp_in[:, j],
                acc, np.zeros_like(acc), acc_exp)
            cpwr_result = self.cpwr.model(
                re_in_c, im_in_c, acc_c, peak_detect)
            if peak_detect:
                pwr, is_greater = cpwr_result
                acc[is_greater] = pwr[is_greater]
                acc_exp[is_greater] = exp_c[is_greater]
            else:
                acc[:] = cpwr_result
                acc_exp[:] = exp_c
        # Bit reverse accumulator order
        acc = acc[:, [bit_invert(n, self.order_log2, 1)
                      for n in range(2**self.order_log2)]]
        acc_exp = acc_exp[:, [bit_invert(n, self.order_log2, 1)
                              for n in range(2**self.order_log2)]]
        # Perform fftshift
        acc = np.fft.fftshift(acc, axes=-1)
        acc_exp = np.fft.fftshift(acc_exp, axes=-1)
        return acc.ravel(), acc_exp.ravel()

    def elaborate(self, platform):
        m = Module()
        m.submodules.to_fp = to_fp = self.to_fp
        m.submodules.common_exp = common_exp = self.common_exp
        m.submodules.cpwr = cpwr = self.cpwr

        mems = [Memory(width=self.sumw+self.ew, depth=2**self.order_log2)
                for _ in range(2)]
        rdports = [mem.read_port(transparent=False) for mem in mems]
        # BRAM output register
        rdports_reg = [Signal(self.sumw+self.ew, name=f'rdport{j}_reg',
                              reset_less=True)
                       for j in range(2)]
        for j in range(2):
            with m.If(rdports[j].en):
                m.d.sync += rdports_reg[j].eq(rdports[j].data)
        m.submodules.rdport0 = rdports[0]
        m.submodules.rdport1 = rdports[1]
        wrports = [mem.write_port() for mem in mems]
        m.submodules.wrport0 = wrports[0]
        m.submodules.wrport1 = wrports[1]

        # We use the output register on the BRAM.
        mem_delay = 2
        # Accumulator data is subject to the BRAM delay. Input data is subject
        # to floating-point conversion. Since these operations have the same
        # delay, we do not need to delay any of them to align them.
        assert mem_delay == to_fp.delay
        processing_delay = mem_delay + common_exp.delay + cpwr.delay

        read_counter_rst = 0
        read_counter = Signal(self.order_log2, reset=read_counter_rst)
        write_counter_rst = (
            (read_counter_rst - processing_delay) % 2**self.order_log2)
        write_counter = Signal(self.order_log2, reset=write_counter_rst)
        sum_counter = Signal(self.nw)
        not_first_sum = Signal()
        not_first_sum_delay = Signal(mem_delay)
        pingpong = Signal()
        pingpong_delay = Signal(processing_delay, reset_less=False)
        pingpong_q = Signal(reset_less=False)
        do_abort = Signal()

        with m.If(self.clken):
            m.d.sync += [
                read_counter.eq(read_counter + 1),
                write_counter.eq(write_counter + 1),
                pingpong_delay.eq(Cat(pingpong, pingpong_delay[:-1])),
                not_first_sum_delay.eq(
                    Cat(not_first_sum, not_first_sum_delay[:-1])),
            ]

            with m.If(self.input_last):
                # An FFT vector ends
                m.d.sync += [
                    read_counter.eq(read_counter_rst),
                    write_counter.eq(write_counter_rst),
                    not_first_sum.eq(1),
                    sum_counter.eq(sum_counter - 1),
                ]
                with m.If((sum_counter == 1) | (sum_counter == 0) | do_abort):
                    # A new sum starts
                    m.d.sync += [
                        sum_counter.eq(self.nint),
                        not_first_sum.eq(0),
                        pingpong.eq(~pingpong),
                        do_abort.eq(0),
                    ]

        with m.If(self.abort):
            m.d.sync += do_abort.eq(1)

        m.d.sync += pingpong_q.eq(pingpong_delay[-1])

        # The read and write counters are reversed to perform bit order
        # inversion in the FFT indices. Moreover, the MSB is negated to perform
        # fftshift.
        read_counter_rev = read_counter[::-1]
        write_counter_rev = write_counter[::-1]
        read_counter_shift = Cat(read_counter_rev[:-1],
                                 ~read_counter_rev[-1])
        write_counter_shift = Cat(write_counter_rev[:-1],
                                  ~write_counter_rev[-1])

        exp_delay = [Signal(self.ew, name=f'exp_q_{j}', reset_less=True)
                     for j in range(cpwr.delay)]
        with m.If(self.clken):
            m.d.sync += exp_delay[0].eq(common_exp.exponent_out)
            m.d.sync += [exp_delay[j].eq(exp_delay[j - 1])
                         for j in range(1, len(exp_delay))]

        writeback = Signal()
        read_data = Signal(self.sumw + self.ew)
        rdata = Mux(pingpong, rdports_reg[0], rdports_reg[1])
        m.d.comb += [
            to_fp.clken.eq(self.clken),
            to_fp.re_in.eq(self.re_in),
            to_fp.im_in.eq(self.im_in),

            common_exp.clken.eq(self.clken),
            common_exp.re_a_in.eq(to_fp.re_out),
            common_exp.im_a_in.eq(to_fp.im_out),
            common_exp.exponent_a_in.eq(to_fp.exponent_out),
            read_data.eq(
                Mux(not_first_sum_delay[-1],
                    Mux(pingpong_delay[mem_delay - 1],
                        rdports_reg[1], rdports_reg[0]),
                    0)),
            common_exp.b_in.eq(read_data[:self.sumw]),
            common_exp.exponent_b_in.eq(read_data[-self.ew:]),

            cpwr.clken.eq(self.clken),
            cpwr.common_edge.eq(self.common_edge),
            cpwr.peak_detect.eq(self.peak_detect),
            cpwr.re_in.eq(common_exp.re_a_out),
            cpwr.im_in.eq(common_exp.im_a_out),
            cpwr.real_in.eq(common_exp.b_out),

            self.done.eq(pingpong_delay[-1] ^ pingpong_q),
            # We need to include pingpong_delay[1] here because otherwise the
            # rden would be active immediately after toggling pingpong, and we
            # would lose the contents of the ram output register.
            rdports[0].en.eq(Mux(pingpong & pingpong_delay[mem_delay - 1],
                                 self.rden, self.clken)),
            rdports[1].en.eq(Mux(pingpong | pingpong_delay[mem_delay - 1],
                                 self.clken, self.rden)),
            rdports[0].addr.eq(Mux(pingpong, self.rdaddr, read_counter_shift)),
            rdports[1].addr.eq(Mux(pingpong, read_counter_shift, self.rdaddr)),
            # In average mode, always write back to the BRAM. In peak detect
            # mode, only write back when the cpwr says that the new power is
            # greater than the one in the BRAM.
            writeback.eq(~self.peak_detect | cpwr.is_greater),
            wrports[0].en.eq((~pingpong_delay[-1]) & self.clken & writeback),
            wrports[1].en.eq(pingpong_delay[-1] & self.clken & writeback),
            self.rdata_value.eq(rdata[:self.sumw]),
            self.rdata_exponent.eq(rdata[-self.ew:]),
        ]
        for wr in wrports:
            m.d.comb += [
                wr.addr.eq(write_counter_shift),
                wr.data.eq(Cat(cpwr.out[:self.sumw], exp_delay[-1])),
                ]
        return m


if __name__ == '__main__':
    integrator = SpectrumIntegrator('clk_3x', 22, 18, 10, 12)
    amaranth.cli.main(
        integrator, ports=[
            integrator.clken, integrator.nint, integrator.peak_detect,
            integrator.common_edge,
            integrator.input_last, integrator.re_in, integrator.im_in,
            integrator.done, integrator.rdaddr, integrator.rdata_value,
            integrator.rdata_exponent, integrator.rden],
        platform=PlutoPlatform())
