#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

# This module contains utilities for "floating-point" representation. Unlike
# IEEE 754, this representation uses an unsigned exponent, which means that the
# mantissa is not necessarily of the form 1.xxxx. It can start by 0. The way to
# understand this kind of floating point representation is to think of
# converting an integer from width w1 to a narrower width w2 by right shifting
# it the minimum number of places needed to avoid overflow. The number of
# places that have been shifted is stored as the exponent. It has a maximum
# value of w1 - w2 and a minimum value of 0, which happens whenever the initial
# value already fits in w2 bits.

from amaranth import *
import amaranth.cli
import numpy as np


class ShiftRight(Elaboratable):
    """Shift right with a maximum shift

    This module performs a shift right combinationally, but it assumes a
    maximum shift that can be smaller than the largest integer representable
    by the  width of the shift value, so it uses less logic than the ``>>``
    operator.

    Parameters
    ----------
    width : int
        Width of input and output values.
    shift_width : int
        Width of shift value.
    max_shift : int
        Maximum acceptable shift value.
    is_signed : bool
        Selects whether the input and output are signed.
    is_power : bool
        If this parameter is set to True, then the shift is done by
        twice as many places as indicated in ``shift``. This is used
        to shift powers (modulus squared) values, which have been
        computed by computing the modulus squared of the mantissa and
        keeping the same exponent. In this case, the exponent represents
        powers of 4 instead of powers of 2.

    Attributes
    ----------
    in_data : Signal(width), in
        Input value (signed or unsigned depending on ``is_signed``).
    shift : Signal(shift_width), in
        Shift value.
    out_data : Signal(width), out
        Output value (signed or unsigned depending on ``is_signed``).
    """
    def __init__(self, width, shift_width, max_shift, *, is_signed=True,
                 is_power=False):
        self.w = width
        self.sw = shift_width
        self.max_shift = max_shift
        assert max_shift < 2**shift_width
        self.signed = signed if is_signed else unsigned
        self.is_power = is_power

        self.in_data = Signal(self.signed(width))
        self.shift = Signal(shift_width)
        self.out_data = Signal(self.signed(width))

    def elaborate(self, platform):
        m = Module()
        for j in range(self.max_shift + 1):
            with m.If(self.shift == j):
                s = 2 * j if self.is_power else j
                m.d.comb += self.out_data.eq(self.in_data >> s)
        return m


class IQToFloatingPoint(Elaboratable):
    """Convert an IQ value to a floating point representation

    This converts an IQ value of a larger width into an IQ value of a narrower
    width and an exponent, which represents by how many places the value has
    been right shifted to perform the narrowing without overflowing.

    Parameters
    ----------
    in_width : int
        Width of the input.
    out_width : int
        Width of the output.

    Attributes
    ----------
    delay : int
        Delay (in samples) from input to output.
    clken : Signal(), in
        Clock enable
    re_in : Signal(signed(in_width)), in
        Input real part.
    im_in : Signal(signed(in_width)), in
        Input imaginary part.
    re_out : Signal(signed(out_width)), out
        Output real part.
    im_out : Signal(signed(out_width)), out
        Output imaginary part.
    exponent_out : Signal(exponent_width), out
        Exponent. This indicates by how many places the input has been right
        shifted to obtain the output value. The width of this value is
        determined automatically according to the number of bits needed to
        represent ``in_width - out_width``, since that is the maximum shift
        that can happen.
    """
    def __init__(self, in_width, out_width):
        assert out_width < in_width
        self.iw = in_width
        self.ow = out_width
        # exponent width
        self.ew = (in_width - out_width).bit_length()

        self.clken = Signal()
        self.re_in = Signal(signed(self.iw))
        self.im_in = Signal(signed(self.iw))
        self.re_out = Signal(signed(self.ow), reset_less=True)
        self.im_out = Signal(signed(self.ow), reset_less=True)
        self.exponent_out = Signal(self.ew, reset_less=True)

    @property
    def delay(self):
        return 2

    def model(self, re_in, im_in):
        def calc_len(a):
            return (int(a).bit_length() + 1 if a >= 0
                    else (-(int(a)+1)).bit_length() + 1)

        cl = np.vectorize(calc_len)
        re_lens = cl(re_in)
        im_lens = cl(im_in)
        re_exp, im_exp = (np.maximum(lens - self.ow, 0)
                          for lens in (re_lens, im_lens))
        exp = np.maximum(re_exp, im_exp)
        return re_in >> exp, im_in >> exp, exp

    def elaborate(self, platform):
        m = Module()

        exponent_re = Signal(self.ew)
        exponent_im = Signal(self.ew)
        m.d.comb += [exponent_re.eq(0), exponent_im.eq(0)]
        for j in reversed(range(self.iw - self.ow)):
            with m.If(self.re_in[-1] != self.re_in[-2-j]):
                m.d.comb += exponent_re.eq(self.iw - self.ow - j)
            with m.If(self.im_in[-1] != self.im_in[-2-j]):
                m.d.comb += exponent_im.eq(self.iw - self.ow - j)

        exponent = Signal(self.ew, reset_less=True)
        re_q = Signal(signed(self.iw), reset_less=True)
        im_q = Signal(signed(self.iw), reset_less=True)

        m.submodules.shift_re = ShiftRight(self.iw, self.ew, self.iw - self.ow)
        m.submodules.shift_im = ShiftRight(self.iw, self.ew, self.iw - self.ow)
        m.d.comb += [
            m.submodules.shift_re.in_data.eq(re_q),
            m.submodules.shift_re.shift.eq(exponent),
            m.submodules.shift_im.in_data.eq(im_q),
            m.submodules.shift_im.shift.eq(exponent),
        ]
        with m.If(self.clken):
            m.d.sync += [
                exponent.eq(
                    Mux(exponent_re >= exponent_im, exponent_re, exponent_im)),
                re_q.eq(self.re_in),
                im_q.eq(self.im_in),

                self.re_out.eq(m.submodules.shift_re.out_data),
                self.im_out.eq(m.submodules.shift_im.out_data),
                self.exponent_out.eq(exponent),
            ]

        return m


class MakeCommonExponent(Elaboratable):
    """Convert two floating point numbers to a common exponent

    Given two input values in floating-point representation, this converts them
    to a representation using the same exponent by taking the maximum exponent
    of both and right-shifting each of the inputs by the difference between the
    maximum exponent and the exponent that the input had.

    The inputs can be either IQ or real, and either signed or
    unsigned. Moreover, this module also supports a "power" representation in
    which exponent units represent a shift by 2 places instead of a shift by 1
    place as usual. This representation is used with numbers which are a power
    (instead of amplitude), which has been computed by computing the modulus
    squared of the mantissa and keeping the exponent value unchanged.

    Parameters
    ----------
    a_width : int
        Width of the a input.
    b_width : int
        Width of the a input.
    exponent_width : int
        Width of the exponent fields.
    max_exponent : int
        Maximum exponent that can appear.
    a_complex : bool
        Determines if a is IQ or real.
    b_complex : bool
        Determines if b is IQ or real.
    a_power : bool
        Determines if a uses a power representation or not.
    b_power : bool
        Determines if b uses a power representation or not.
    a_signed : bool
        Determines if a is signed.
    b_signed : bool
        Determines if b is signed.

    Attributes
    ----------
    delay : int
        Delay (in samples) from input to output.
    clken : Signal(), in
        Clock enable
    re_a_in : Signal(a_width), in
        Input real part of a (only present when a_complex is True). It can be
        signed or unsigned according to a_signed.
    im_a_in : Signal(a_width), in
        Input imaginary part of a (only present when a_complex is True). It can
        be signed or unsigned according to a_signed.
    re_a_out : Signal(a_width), out
        Output real part of a (only present when a_complex is True). It can be
        signed or unsigned according to a_signed.
    im_a_out : Signal(a_width), out
        Input imaginary part of a (only present when a_complex is True). It can
        be signed or unsigned according to a_signed.
    a_in : Signal(a_width), in
        Input for a (only present when a_complex is False). It can be signed
        or unsigned according to a_signed.
    a_out : Signal(a_width), in
        Output for a (only present when a_complex is False). It can be signed
        or unsigned according to a_signed.
    exponent_a_in : Signal(exponent_width), in
        Input exponent for a.
    re_b_in : Signal(b_width), in
        Input real part of b (only present when b_complex is True). It can be
        signed or unsigned according to b_signed.
    im_b_in : Signal(b_width), in
        Input imaginary part of b (only present when b_complex is True). It can
        be signed or unsigned according to b_signed.
    re_b_out : Signal(b_width), out
        Output real part of b (only present when b_complex is True). It can be
        signed or unsigned according to b_signed.
    im_b_out : Signal(b_width), out
        Output imaginary part of b (only present when b_complex is True). It
        can be signed or unsigned according to b_signed.
    b_in : Signal(b_width), in
        Input for b (only present when b_complex is False). It can be signed
        or unsigned according to b_signed.
    b_out : Signal(b_width), out
        Output for b (only present when b_complex is False). It can be signed
        or unsigned according to b_signed.
    exponent_b_in : Signal(exponent_width), in
        Input exponent for b.
    re_out : Signal(signed(out_width)), out
        Output real part.
    im_out : Signal(signed(out_width)), out
        Output imaginary part.
    exponent_out : Signal(exponent_width), out
        Common exponent for the outputs.
    """
    def __init__(self, a_width, b_width, exponent_width, max_exponent, *,
                 a_complex=False, b_complex=False,
                 a_power=False, b_power=False,
                 a_signed=True, b_signed=True):
        self.aw = a_width
        self.bw = b_width
        self.ew = exponent_width
        self.max_exp = max_exponent
        self.a_complex = a_complex
        self.b_complex = b_complex
        self.a_power = a_power
        self.b_power = b_power
        self.a_signed = signed if a_signed else unsigned
        self.b_signed = signed if b_signed else unsigned
        self.is_a_signed = a_signed
        self.is_b_signed = b_signed

        self.clken = Signal()
        if a_complex:
            self.re_a_in = Signal(self.a_signed(self.aw))
            self.im_a_in = Signal(self.a_signed(self.aw))
            self.re_a_out = Signal(self.a_signed(self.aw), reset_less=True)
            self.im_a_out = Signal(self.a_signed(self.aw), reset_less=True)
        else:
            self.a_in = Signal(self.a_signed(self.aw))
            self.a_out = Signal(self.a_signed(self.aw), reset_less=True)
        self.exponent_a_in = Signal(self.ew)
        if b_complex:
            self.re_b_in = Signal(self.b_signed(self.bw))
            self.im_b_in = Signal(self.b_signed(self.bw))
            self.re_b_out = Signal(self.b_signed(self.bw), reset_less=True)
            self.im_b_out = Signal(self.b_signed(self.bw), reset_less=True)
        else:
            self.b_in = Signal(self.b_signed(self.bw))
            self.b_out = Signal(self.b_signed(self.bw), reset_less=True)
        self.exponent_b_in = Signal(self.ew)
        self.exponent_out = Signal(self.ew, reset_less=True)

    @property
    def delay(self):
        return 2

    def model(self, re_a, im_a, exponent_a, re_b, im_b, exponent_b):
        max_exponent = np.maximum(exponent_a, exponent_b)
        diff_a = max_exponent - exponent_a
        diff_b = max_exponent - exponent_b
        if self.a_power:
            diff_a *= 2
        if self.b_power:
            diff_b *= 2
        return (re_a >> diff_a, im_a >> diff_a,
                re_b >> diff_b, im_b >> diff_b,
                max_exponent)

    def elaborate(self, platform):
        m = Module()

        max_exponent = Mux(self.exponent_a_in >= self.exponent_b_in,
                           self.exponent_a_in, self.exponent_b_in)
        exponent = Signal(self.ew, reset_less=True)
        diff_a = Signal(self.ew, reset_less=True)
        diff_b = Signal(self.ew, reset_less=True)
        with m.If(self.clken):
            m.d.sync += [
                exponent.eq(max_exponent),
                diff_a.eq(max_exponent - self.exponent_a_in),
                diff_b.eq(max_exponent - self.exponent_b_in),
                self.exponent_out.eq(exponent),
            ]

        if self.a_complex:
            re_a_q = Signal(self.a_signed(self.aw), reset_less=True)
            im_a_q = Signal(self.a_signed(self.aw), reset_less=True)
            m.submodules.shift_re_a = ShiftRight(
                self.aw, self.ew, self.max_exp,
                is_signed=self.is_a_signed, is_power=self.a_power)
            m.submodules.shift_im_a = ShiftRight(
                self.aw, self.ew, self.max_exp,
                is_signed=self.is_a_signed, is_power=self.a_power)
            m.d.comb += [
                m.submodules.shift_re_a.in_data.eq(re_a_q),
                m.submodules.shift_re_a.shift.eq(diff_a),
                m.submodules.shift_im_a.in_data.eq(im_a_q),
                m.submodules.shift_im_a.shift.eq(diff_a),
            ]
            with m.If(self.clken):
                m.d.sync += [
                    re_a_q.eq(self.re_a_in),
                    im_a_q.eq(self.im_a_in),
                    self.re_a_out.eq(
                        m.submodules.shift_re_a.out_data),
                    self.im_a_out.eq(
                        m.submodules.shift_im_a.out_data),
                ]
        else:
            a_q = Signal(self.a_signed(self.aw), reset_less=True)
            m.submodules.shift_a = ShiftRight(
                self.aw, self.ew, self.max_exp,
                is_signed=self.is_a_signed, is_power=self.a_power)
            m.d.comb += [
                m.submodules.shift_a.in_data.eq(re_a_q),
                m.submodules.shift_a.shift.eq(diff_a),
            ]
            with m.If(self.clken):
                m.d.sync += [
                    a_q.eq(self.a_in),
                    self.a_out.eq(m.d.submodules.shift_a.out_data),
                ]

        if self.b_complex:
            re_b_q = Signal(self.b_signed(self.bw), reset_less=True)
            im_b_q = Signal(self.b_signed(self.bw), reset_less=True)
            m.submodules.shift_re_b = ShiftRight(
                self.bw, self.ew, self.max_exp,
                is_signed=self.is_b_signed, is_power=self.b_power)
            m.submodules.shift_im_b = ShiftRight(
                self.bw, self.ew, self.max_exp,
                is_signed=self.is_b_signed, is_power=self.b_power)
            m.d.comb += [
                m.submodules.shift_re_b.in_data.eq(re_b_q),
                m.submodules.shift_re_b.shift.eq(diff_b),
                m.submodules.shift_im_b.in_data.eq(im_b_q),
                m.submodules.shift_im_b.shift.eq(diff_b),
            ]
            with m.If(self.clken):
                m.d.sync += [
                    re_b_q.eq(self.re_b_in),
                    im_b_q.eq(self.im_b_in),
                    self.re_b_out.eq(
                        m.submodules.shift_re_b.out_data),
                    self.im_b_out.eq(
                        m.submodules.shift_im_b.out_data),
                ]
        else:
            b_q = Signal(self.b_signed(self.bw), reset_less=True)
            m.submodules.shift_b = ShiftRight(
                self.bw, self.ew, self.max_exp,
                is_signed=self.is_b_signed, is_power=self.b_power)
            m.d.comb += [
                m.submodules.shift_b.in_data.eq(b_q),
                m.submodules.shift_b.shift.eq(diff_b),
            ]
            with m.If(self.clken):
                m.d.sync += [
                    b_q.eq(self.b_in),
                    self.b_out.eq(m.submodules.shift_b.out_data),
                ]

        return m


if __name__ == '__main__':
    dut = MakeCommonExponent(18, 47, 3, 4, a_complex=True, b_power=True)
    amaranth.cli.main(
        dut, ports=[
            dut.clken, dut.re_a_in, dut.im_a_in, dut.exponent_a_in,
            dut.re_a_out, dut.im_a_out,
            dut.b_in, dut.exponent_b_in, dut.b_out, dut.exponent_out])
