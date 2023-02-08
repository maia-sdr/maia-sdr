#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli


class Cpwr(Elaboratable):
    def __init__(self, width, add_width=0, add_shift=0, add_latency=0,
                 truncate=0):
        """Complex power

        This module uses 2 multipliers in pipeline to compute the power
        (amplitude squared) of a complex number. There is an additional
        input for a real number to be added to the result. This is useful
        for a power integrator, which adds the value of the accumulator
        to the power of the current sample.

        Parameters
        ----------
        width : int
            Width of the input sample.
        add_width : int
            Width of the real input 'add' to be added.
        add_shift : int
            Number of bits to shift the 'add' real number to the left.
            This is used because the power has a large bit growth and
            is often truncated after the addition.
        add_latency : int
            Latency with which the 'add' input is delivered relative to
            the complex input. A latency of 0 means that both inputs are
            delivered simultaneously. A latency of 1 means that the 'add'
            input corresponding to the current complex sample will be
            delivered together with the next sample. A latency greater
            than 1 absorbes some flip-flops that would delay the 'add'
            input for a latency of 0.
        truncate : int
            Number of bits to truncate in the output.

        Attributes
        ----------
        delay : in
            Delay (in samples) introduced by this module to the complex input
            data.
        clken : Signal(), in
            Clock enable.
        re_in : Signal(signed(width)), in
            Input sample real part.
        im_in : Signal(signed(width)), in
            Input sample imaginary part.
        add_in : Signal(signed(add_width)), in
            Real value to be added.
        out : Signal(signed(output_width)), out
            Output, formally ``re_in**2 + im_in**2 + add_in`` (with the
            appropriate shifts and truncations). The output width is computed
            according to ``width``, ``add_width`` and ``truncate``.
        """
        self.w = width
        self.outw = (
            2 * width + 2 - truncate
            if 2 * width >= add_width + add_shift
            else add_width + add_shift + 1 - truncate)
        self.add_width = add_width
        self.add_shift = add_shift
        self.add_latency = add_latency
        self.truncate = truncate

        self.re_delay = 1
        if self.re_delay + 1 < self.add_latency:
            raise ValueError('add_latency cannot be larger than re_delay + 1')

        self.clken = Signal()
        self.add_in = Signal(signed(add_width))
        self.re_in = Signal(signed(self.w))
        self.im_in = Signal(signed(self.w))
        self.out = Signal(signed(self.outw))

    @property
    def delay(self):
        return self.re_delay + 3

    def model(self, re_in, im_in, add_in):
        return (
            re_in**2 + im_in**2 + (add_in << self.add_shift)
            ) >> self.truncate

    def elaborate(self, platform):
        m = Module()

        # Note that im_q is delayed one cycle more than re_q
        re_q = [Signal(signed(self.w), name=f're_q{i+1}',
                       reset_less=True)
                for i in range(self.re_delay)]
        im_q = [Signal(signed(self.w), name=f'im_q{i+1}',
                       reset_less=True)
                for i in range(self.re_delay + 1)]
        add_delay = self.re_delay - self.add_latency + 1
        if add_delay:
            add_q = [Signal(signed(self.add_width + self.add_shift),
                            name=f'add_q{i+1}', reset_less=True)
                     for i in range(add_delay)]
        add_out = add_q[-1] if add_delay else self.add_in << self.add_shift

        re_square = Signal(signed(2 * self.w), reset_less=True)
        im_square = Signal(signed(2 * self.w), reset_less=True)
        re_sum = Signal(
            signed(max(2 * self.w, self.add_width + self.add_shift) + 1),
            reset_less=True)
        im_sum = Signal(signed(self.outw + self.truncate), reset_less=True)

        with m.If(self.clken):
            m.d.sync += re_q[0].eq(self.re_in)
            m.d.sync += [re_q[j].eq(re_q[j - 1])
                         for j in range(1, len(re_q))]
            m.d.sync += im_q[0].eq(self.im_in)
            m.d.sync += [im_q[j].eq(im_q[j - 1])
                         for j in range(1, len(im_q))]
            if add_delay:
                m.d.sync += add_q[0].eq(self.add_in << self.add_shift)
                m.d.sync += [add_q[j].eq(add_q[j - 1])
                             for j in range(1, len(add_q))]
            m.d.sync += [
                re_square.eq(re_q[-1] * re_q[-1]),
                im_square.eq(im_q[-1] * im_q[-1]),
                re_sum.eq(re_square + add_out),
                im_sum.eq(re_sum + im_square),
            ]
        m.d.comb += self.out.eq(im_sum >> self.truncate)
        return m


if __name__ == '__main__':
    cpwr = Cpwr(width=16, add_width=24, add_shift=16, truncate=16,
                add_latency=1)
    amaranth.cli.main(
        cpwr, ports=[
            cpwr.clken, cpwr.re_in, cpwr.im_in, cpwr.add_in, cpwr.out])
