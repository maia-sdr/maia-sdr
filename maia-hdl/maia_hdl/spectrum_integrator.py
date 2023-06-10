#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli

import numpy as np

from .cpwr import Cpwr
from .util import bit_invert


class SpectrumIntegrator(Elaboratable):
    def __init__(self, input_width, nint_width, fft_order_log2,
                 cpwr_truncate=None):
        """Spectrum integrator

        This module uses a Cpwr and a BRAM to compute the integrated power
        at the output of an FFT. The number of integrations can be changed
        at runtime.

        A ping-pong approach with the BRAMs is used so that a reader module
        can read the previous integration while the current integration is
        being computed.

        Parameters
        ----------
        input_width : int
            Width of the input samples.
        nint_width : int
            Width of the input that indicates the number of integrations.
        fft_order_log2 : int
            Determines the FFT size, as ``2**fft_order_log2``.
        cpwr_truncate : Optional[int]
            Truncation to apply in the Cpwr module. By default, a truncation
            of ``input_width + 1`` is applied to compensate for the bit growth
            in the squaring and summation of the real and imaginary parts.

        Attributes
        ----------
        nint : Signal(nint_width), in
            Number of integrations to perform. This signal is only latched
            after the current integration has finished.
        clken : Signal(), in
            Clock enable.
        input_last : Signal(), in
            This signal should be asserted when the last sample of the FFT
            vector is presented at the input.
        re_in : Signal(input_width), in
            Real part of the input sample.
        im_in : Signal(input_width), in
            Imaginary part of the input sample.
        done : Signal(), out
            This signal is pulsed for one clock cycle whenever an integration
            is finished.
        rdaddr : Signal(fft_order_log2), in
            Read address for the BRAM that contains the previous integration.
        rdata : Signal(sum_width), out
            Read data for the BRAM that contains the previous integration.
        rden : Signal(), in
            Read enable for the BRAM that contains the previous integration.
        """
        self.w = input_width
        self.nw = nint_width
        self.cpwr_truncate = (
            self.w + 1 if cpwr_truncate is None else cpwr_truncate)
        # Here + 1 accounts for the addition of the real and imaginary parts.
        self.sumw = 2*self.w - self.cpwr_truncate + nint_width + 1
        self.order_log2 = fft_order_log2

        self.nint = Signal(nint_width)
        self.clken = Signal()
        self.input_last = Signal()
        self.re_in = Signal(input_width)
        self.im_in = Signal(input_width)
        self.done = Signal()
        self.rdaddr = Signal(fft_order_log2)
        self.rdata = Signal(self.sumw)
        self.rden = Signal()

        # Compensate one of the BRAM latency cycles with the Cpwr add_latency.
        self.cpwr_add_latency = 1
        self.cpwr = Cpwr(
            self.w, add_width=self.sumw, add_shift=self.cpwr_truncate,
            add_latency=self.cpwr_add_latency, truncate=self.cpwr_truncate)

    @property
    def model_vlen(self, nint):
        return 2**self.order_log2 * nint

    def model(self, nint, re_in, im_in):
        re_in, im_in = (
            np.array(x, 'int').reshape(-1, nint, 2**self.order_log2)
            for x in [re_in, im_in])
        acc = np.zeros((re_in.shape[0], 2**self.order_log2), 'int')
        for j in range(nint):
            acc = self.cpwr.model(re_in[:, j], im_in[:, j], acc)
        # Bit reverse accumulator order
        acc = acc[:, [bit_invert(n, self.order_log2, 1)
                      for n in range(2**self.order_log2)]]
        # Perform fftshift
        acc = np.fft.fftshift(acc, axes=-1)
        return acc.ravel()

    def elaborate(self, platform):
        m = Module()
        m.submodules.cpwr = cpwr = self.cpwr

        mems = [Memory(width=self.sumw, depth=2**self.order_log2)
                for _ in range(2)]
        rdports = [mem.read_port(transparent=False) for mem in mems]
        # BRAM output register
        rdports_reg = [Signal(self.sumw, name=f'rdport{j}_reg',
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

        read_counter_rst = 0
        read_counter = Signal(self.order_log2, reset=read_counter_rst)
        # Here 1 accounts for the extra delay (re_q, im_q) (see below).
        write_counter_rst = (
            (read_counter_rst - cpwr.delay - 1) % 2**self.order_log2)
        write_counter = Signal(self.order_log2, reset=write_counter_rst)
        sum_counter = Signal(self.nw)
        not_first_sum = Signal()
        not_first_sum_delay = Signal(mem_delay)
        pingpong = Signal()
        pingpong_delay = Signal(cpwr.delay + mem_delay - self.cpwr_add_latency,
                                reset_less=False)
        pingpong_q = Signal(reset_less=False)

        # These are used to compensate for mem_delay in the input signal. We
        # only need to register once because we are compensating one out of two
        # clock cycles with the Cpwr add_latency.
        re_q = Signal(self.w, reset_less=False)
        im_q = Signal(self.w, reset_less=False)

        with m.If(self.clken):
            m.d.sync += [
                re_q.eq(self.re_in),
                im_q.eq(self.im_in),
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
                with m.If((sum_counter == 1) | (sum_counter == 0)):
                    # A new sum starts
                    m.d.sync += [
                        sum_counter.eq(self.nint),
                        not_first_sum.eq(0),
                        pingpong.eq(~pingpong),
                    ]

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
        m.d.comb += [
            cpwr.clken.eq(self.clken),
            cpwr.re_in.eq(re_q),
            cpwr.im_in.eq(im_q),
            cpwr.add_in.eq(
                Mux(not_first_sum_delay[-1],
                    Mux(pingpong_delay[mem_delay],
                        rdports_reg[1], rdports_reg[0]),
                    0)),
            self.done.eq(pingpong_delay[-1] ^ pingpong_q),
            # We need to include pingpong_delay[1] here because otherwise the
            # rden would be active immediately after toggling pingpong, and we
            # would lose the contents of the ram output register.
            rdports[0].en.eq(Mux(pingpong & pingpong_delay[mem_delay],
                                 self.rden, self.clken)),
            rdports[1].en.eq(Mux(pingpong | pingpong_delay[mem_delay],
                                 self.clken, self.rden)),
            rdports[0].addr.eq(Mux(pingpong, self.rdaddr, read_counter_shift)),
            rdports[1].addr.eq(Mux(pingpong, read_counter_shift, self.rdaddr)),
            wrports[0].en.eq((~pingpong_delay[-1]) & self.clken),
            wrports[1].en.eq(pingpong_delay[-1] & self.clken),
            self.rdata.eq(Mux(pingpong, rdports_reg[0], rdports_reg[1]))
        ]
        for wr in wrports:
            m.d.comb += [
                wr.addr.eq(write_counter_shift),
                wr.data.eq(cpwr.out),
                ]
        return m


if __name__ == '__main__':
    integrator = SpectrumIntegrator(16, 8, 12)
    amaranth.cli.main(
        integrator, ports=[
            integrator.clken, integrator.nint, integrator.input_last,
            integrator.re_in, integrator.im_in, integrator.done,
            integrator.rdaddr, integrator.rdata])
