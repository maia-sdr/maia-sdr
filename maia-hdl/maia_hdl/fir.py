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

from .util import clamp_nbits


class Macc(Elaboratable):
    """Multiply-accumulate

    This module is a multiply-accumulate that uses a single DSP48. When valid
    inputs are fed, the ``strobe_in`` input must be asserted. The output
    accumulator is updated 4 cycles afterwards. The ``first_acc`` input
    indicates that the current input is the first in a new accumulation, so the
    accumulator should not carry over the additions of the products
    corresponding to previous inputs.

    Parameters
    ----------
    a_width : int
        The width of the ``a`` input.
    b_width : int
        The width of the ``b`` input.
    acc_width : int
        The width of the accumulator.
    truncate_round : Optional[int]
        If this parameter is set to an integer, then an additional summand
        equal to ``2**(truncate_round-1)`` is added to the accumulator. When
        ``truncate_round`` LSBs of the MACC output are truncated, the effect
        achieved by this extra summand is that of round half-up instead of
        floor.

    Attributes
    ----------
    delay : int
        Delay (in clock cycles) introduced by this module.
    strobe_in : Signal(), in
        Indicates if the current inputs are valid.
    first_acc : Signal(), in
        Indicates that the current inputs are the first in a new accumulation.
    a : Signal(signed(a_width)), in
        Input ``a``.
    b : Signal(signed(b_width)), in
        Input ``b``.
    acc : Signal(signed(acc_width)), out
        Accumulator. Contains the sum of the products ``a * b`` for all valid
        inputs since the last time that ``first_acc`` was asserted.

    """
    def __init__(self, a_width, b_width, *,
                 acc_width=48, truncate_round=None):
        if truncate_round is not None and truncate_round < 1:
            raise ValueError('truncate_round must be greater or equal than 1')
        self.aw = a_width
        self.bw = b_width
        self.truncate_round = truncate_round

        self.strobe_in = Signal()
        self.first_acc = Signal()
        self.a = Signal(signed(self.aw))
        self.b = Signal(signed(self.bw))
        self.acc = Signal(signed(acc_width), reset_less=True)

    @property
    def delay(self):
        return 4

    def elaborate(self, platform):
        m = Module()

        a_q = Signal(signed(self.aw), reset_less=True)
        a_q2 = Signal(signed(self.aw), reset_less=True)
        b_q = Signal(signed(self.bw), reset_less=True)
        b_q2 = Signal(signed(self.bw), reset_less=True)
        mult = Signal(signed(self.aw + self.bw), reset_less=True)
        strobe_in_q = Signal(3, reset_less=True)
        first_acc_q = Signal(3, reset_less=True)

        m.d.sync += [
            strobe_in_q.eq(Cat(self.strobe_in, strobe_in_q[:-1])),
            first_acc_q.eq(Cat(self.first_acc, first_acc_q[:-1])),
        ]
        with m.If(self.strobe_in):
            m.d.sync += [a_q.eq(self.a), b_q.eq(self.b)]
        m.d.sync += [
            a_q2.eq(a_q), b_q2.eq(b_q),
            mult.eq(a_q2 * b_q2),
        ]
        initial_acc = (2**(self.truncate_round - 1)
                       if self.truncate_round is not None
                       else 0)
        with m.If(strobe_in_q[2]):
            m.d.sync += self.acc.eq(mult + Mux(
                first_acc_q[2], initial_acc, self.acc).as_signed())

        return m


class SampleBuffer(Elaboratable):
    """Sample buffer

    This module is a RAM used to store the input IQ samples used by the FIR
    filter. Each IQ sample is stored in a word of this RAM. The RAM has either
    one or two read ports (so that it can feed 2 MACCs if needed), and one
    write port. The RAM has a read latency of 2 cycles.

    Parameters
    ----------
    width : int
        Word with of the RAM.
    awidth : int
        Address width of the RAM. The number of samples stored is
        ``2**awidth``.
    two_read_ports: bool
        Use two read ports instead of one.

    Attributes
    ----------
    raddr0 : Signal(awidth), in
        Address for read port 0.
    rdata0 : Signal(width), out
        Data for read port 0.
    raddr1 : Signal(awidth), in
        Address for read port 1 (only present if ``two_read_ports == True``).
    rdata1 : Signal(width), out
        Data for read port 1 (only present if ``two_read_ports == True``).
    waddr : Signal(awidth), in
        Address for write port.
    wren : Signal(), in
        Write enable.
    wdata : Signal(width), in
        Data for write port.

    """
    def __init__(self, width, *, awidth=8, two_read_ports=True):
        self.w = width
        self.aw = awidth
        self.two_read_ports = two_read_ports

        self.raddr0 = Signal(awidth)
        self.rdata0 = Signal(width, reset_less=True)
        if two_read_ports:
            self.raddr1 = Signal(awidth)
            self.rdata1 = Signal(width, reset_less=True)
        self.waddr = Signal(awidth)
        self.wren = Signal()
        self.wdata = Signal(width)

    def elaborate(self, platform):
        m = Module()

        mem = Memory(width=self.w, depth=2**self.aw)
        m.submodules.rdport0 = rdport0 = mem.read_port(
            transparent=False)
        if self.two_read_ports:
            m.submodules.rdport1 = rdport1 = mem.read_port(
                transparent=False)
        m.submodules.wrport = wrport = mem.write_port()
        m.d.sync += self.rdata0.eq(rdport0.data)
        m.d.comb += [
            rdport0.en.eq(1),
            rdport0.addr.eq(self.raddr0),
            wrport.en.eq(self.wren),
            wrport.addr.eq(self.waddr),
            wrport.data.eq(self.wdata),
        ]
        if self.two_read_ports:
            m.d.sync += self.rdata1.eq(rdport1.data)
            m.d.comb += [
                rdport1.en.eq(1),
                rdport1.addr.eq(self.raddr1),
            ]

        return m


class Coefficients(Elaboratable):
    """Coefficient RAM

    This module is a RAM used to store the FIR filter coefficients for one
    MACC. The RAM has one read port and one write port, and a read latency of 2
    cycles.

    Parameters
    ----------
    width : int
        Word with of the RAM.
    awidth : int
        Address width of the RAM. The number of coefficients stored is
        ``2**awidth``.

    Attributes
    ----------
    raddr : Signal(awidth), in
        Read address.
    rdata : Signal(width), out
        Read data.
    waddr : Signal(awidth), in
        Write address.
    wren : Signal(), in
        Write enable.
    wdata : Signal(width), in
        Write data.
    """
    def __init__(self, *, width=18, awidth=7):
        self.w = width
        self.aw = awidth

        self.raddr = Signal(awidth)
        self.rdata = Signal(signed(width), reset_less=True)
        self.waddr = Signal(awidth)
        self.wren = Signal()
        self.wdata = Signal(signed(width))

    def elaborate(self, platform):
        m = Module()

        mem = Memory(width=self.w, depth=2**self.aw)
        m.submodules.rdport = rdport = mem.read_port(
            transparent=False)
        m.submodules.wrport = wrport = mem.write_port()
        m.d.sync += self.rdata.eq(rdport.data)
        m.d.comb += [
            rdport.en.eq(1),
            rdport.addr.eq(self.raddr),
            wrport.en.eq(self.wren),
            wrport.addr.eq(self.waddr),
            wrport.data.eq(self.wdata),
        ]

        return m


class FIR4DSP(Elaboratable):
    """Polyphase FIR decimator with 4 DSP48.

    This module is a polyphase FIR decimator that uses 4 DSP48. The length of
    the polyphase branches is defined by a runtime parameter called
    "operations".  The FIR performs 2 multiplies per operation, except in the
    last operation, where it performs one or two operations depending on
    whether ``odd_operations`` is asserted. The length of the polyphase branch
    is equal to this number of multiplications. The module needs at least
    "operations" clock cycles per input sample.

    The FIR length must be smaller or equal than 256, with the additional
    constraint that it must be a multiple of the decimation factor.

    Parameters
    ----------
    in_width : int
        Width of input IQ samples
    out_width : int
        Width of output IQ samples
    coeff_width : int
        FIR coefficients width.
    decim_width : int
        Width of ``decimation`` input.
    oper_width : int
        Width of ``operations_minus_one`` input.
    macc_trunc : int
        Truncation length for output of each MACC. Round half up is used for
        truncation.
    len_log2 : int
        Maximum FIR length given as a log2 (by default, the maximum FIR length
        is 256).

    Attributes
    ----------
    coeff_waddr : Signal(len_log2), in
        Coefficient write address. The lower half of the address space is used
        for macc0_re and macc0_im, and the upper half is used for macc1_re and
        macc1_im.
    coeff_wren : Signal(), in
        Coefficient write enable.
    coeff_wdata : Signal(coeff_width), in
        Coefficient write data.
    decimation : Signal(decim_width), in
        Decimation factor.
    operations_minus_one : Signal(oper_width), in
        Number of operations to perform minus one. This determines the length
        of the polyphase branches. The length is equal to two times the number
        of operations, or two times the number of operations minus one if
        ``odd_operations`` is asserted.
    odd_operations : Signal(), in
        Disable the MACC1 in the last operation in order to achieve an odd
        number of multiplies.
    re_in : Signal(signed(in_width)), in
        Input real part.
    im_in : Signal(signed(in_width)), in
        Input imaginary part.
    in_valid : Signal(), in
        Input valid (uses AXI-Stream handshaking).
    in_ready : Signal(), out
        Input ready (uses AXI-Stream handshaking).
    re_out : Signal(signed(out_width)), out
        Output real part.
    im_out : Signal(signed(out_width)), out
        Output imaginary part.
    strobe_out : Signal(), out
        Output strobe. It is asserted in the clock cycle when the output
        changes. The output is kept constant until the next time that
        ``strobe_out`` is asserted.
    """
    def __init__(self, *, in_width=16, out_width=16,
                 coeff_width=18, decim_width=7, oper_width=7,
                 macc_trunc=19, len_log2=8):
        self.iw = in_width
        self.ow = out_width
        self.coeff_width = coeff_width
        self.decim_width = decim_width
        self.oper_width = oper_width
        self.macc_trunc = macc_trunc
        self.len_log2 = len_log2

        self.coeff_waddr = Signal(len_log2)
        self.coeff_wren = Signal()
        self.coeff_wdata = Signal(coeff_width)
        self.decimation = Signal(decim_width)
        self.operations_minus_one = Signal(oper_width)
        self.odd_operations = Signal()

        self.re_in = Signal(signed(self.iw))
        self.im_in = Signal(signed(self.iw))
        self.in_valid = Signal()
        self.in_ready = Signal()

        self.re_out = Signal(signed(self.ow), reset_less=True)
        self.im_out = Signal(signed(self.ow), reset_less=True)
        self.strobe_out = Signal()

    def model(self, taps, decimation, re_in, im_in):
        assert len(taps) % decimation == 0
        taps = np.array(taps).reshape(-1, decimation)
        history = np.zeros(taps.size - 1, 'int')
        re_out = np.zeros(len(re_in) // decimation, 'int')
        im_out = np.zeros(len(re_in) // decimation, 'int')
        re_in = np.concatenate((history, re_in))
        im_in = np.concatenate((history, im_in))
        for j in range(re_out.size):
            # initial values for rounding
            acc_init = (2**(self.macc_trunc - 1)
                        if self.macc_trunc >= 1
                        else 0)
            re0, im0, re1, im1 = [acc_init] * 4
            for k in range(taps.shape[0]):
                wr = re_in[(j + taps.shape[0] - k - 1)
                           * decimation:][:decimation]
                wi = im_in[(j + taps.shape[0] - k - 1)
                           * decimation:][:decimation]
                sr = np.sum(wr[::-1] * taps[k])
                si = np.sum(wi[::-1] * taps[k])
                if k % 2 == 0:
                    re0 += sr
                    im0 += si
                else:
                    re1 += sr
                    im1 += si
            re0 = clamp_nbits(re0 >> self.macc_trunc, self.ow)
            im0 = clamp_nbits(im0 >> self.macc_trunc, self.ow)
            re1 = clamp_nbits(re1 >> self.macc_trunc, self.ow)
            im1 = clamp_nbits(im1 >> self.macc_trunc, self.ow)
            re_out[j] = clamp_nbits(re0 + re1, self.ow)
            im_out[j] = clamp_nbits(im0 + im1, self.ow)
        return re_out, im_out

    def elaborate(self, platform):
        m = Module()

        truncate_round = (
            self.macc_trunc if self.macc_trunc >= 1
            else None)
        m.submodules.macc0_re = macc0_re = Macc(
            self.iw, self.coeff_width, truncate_round=truncate_round)
        m.submodules.macc0_im = macc0_im = Macc(
            self.iw, self.coeff_width, truncate_round=truncate_round)
        m.submodules.macc1_re = macc1_re = Macc(
            self.iw, self.coeff_width, truncate_round=truncate_round)
        m.submodules.macc1_im = macc1_im = Macc(
            self.iw, self.coeff_width, truncate_round=truncate_round)
        m.submodules.samples = samples = SampleBuffer(
            2*self.iw, awidth=self.len_log2)
        m.submodules.coeffs0 = coeffs0 = Coefficients(
            width=self.coeff_width, awidth=self.len_log2-1)
        m.submodules.coeffs1 = coeffs1 = Coefficients(
            width=self.coeff_width, awidth=self.len_log2-1)

        m.d.comb += [
            macc0_re.a.eq(samples.rdata0[:self.iw]),
            macc0_im.a.eq(samples.rdata0[self.iw:]),
            macc1_re.a.eq(samples.rdata1[:self.iw]),
            macc1_im.a.eq(samples.rdata1[self.iw:]),
            macc0_re.b.eq(coeffs0.rdata),
            macc0_im.b.eq(coeffs0.rdata),
            macc1_re.b.eq(coeffs1.rdata),
            macc1_im.b.eq(coeffs1.rdata),
            samples.wdata.eq(Cat(self.re_in, self.im_in)),
            coeffs0.waddr.eq(self.coeff_waddr[:-1]),
            coeffs1.waddr.eq(self.coeff_waddr[:-1]),
            coeffs0.wren.eq(self.coeff_wren & ~self.coeff_waddr[-1]),
            coeffs1.wren.eq(self.coeff_wren & self.coeff_waddr[-1]),
            coeffs0.wdata.eq(self.coeff_wdata),
            coeffs1.wdata.eq(self.coeff_wdata),
        ]

        write_pointer = Signal(self.len_log2, reset_less=True)
        decimation_counter = Signal(self.decim_width)
        operation_counter = Signal(self.oper_width, reset=1)
        coeff_counter = Signal(self.len_log2 - 1, reset_less=True)
        sample_addr0 = Signal(self.len_log2, reset_less=True)
        sample_addr1 = Signal(self.len_log2, reset_less=True)
        work = Signal(reset=1)
        decimation_end_of_count = Signal(reset_less=True)
        last_operation = Signal(reset_less=True)
        last_acc = Signal(reset_less=True)
        m.d.comb += last_acc.eq(decimation_end_of_count & last_operation)
        two_decim = Cat(Const(0, 1), self.decimation)
        first_acc = Signal()
        first_acc_q = Signal(2, reset_less=True)
        enable_macc0_q = Signal(2, reset_less=True)
        enable_macc1_q = Signal(2, reset_less=True)
        enable_macc0 = Signal(reset_less=True)
        enable_macc1 = Signal(reset_less=True)
        m.d.comb += [
            enable_macc0.eq(work),
            enable_macc1.eq(work & (~last_operation | ~self.odd_operations)),
        ]
        m.d.sync += [
            first_acc_q.eq(Cat(first_acc, first_acc_q[:-1])),
            enable_macc0_q.eq(Cat(enable_macc0, enable_macc0_q[:-1])),
            enable_macc1_q.eq(Cat(enable_macc1, enable_macc1_q[:-1])),
        ]

        with m.If(work):
            m.d.sync += [
                sample_addr0.eq(
                    Mux(last_operation, write_pointer,
                        sample_addr0 - two_decim)),
                sample_addr1.eq(
                    Mux(last_operation, write_pointer - self.decimation,
                        sample_addr1 - two_decim)),
                first_acc.eq(last_acc),
                coeff_counter.eq(Mux(last_acc, 0, coeff_counter + 1)),
                operation_counter.eq(Mux(last_operation,
                                         self.operations_minus_one,
                                         operation_counter - 1)),
                last_operation.eq((operation_counter == 1)
                                  | (self.operations_minus_one == 0)),
            ]
            with m.If(last_operation):
                m.d.sync += [
                    decimation_counter.eq(Mux(
                        last_acc, self.decimation, decimation_counter - 1)),
                    # decimation_counter should never reach 0 during normal
                    # operation, but it can when coming from reset. Using
                    # ~decimation_end_of_count here prevents
                    # decimation_end_of_count to be continuously stuck at 1
                    # when self.decimation is 2.
                    decimation_end_of_count.eq(
                        ~decimation_end_of_count
                        & ((decimation_counter == 2)
                           | (decimation_counter == 0))),
                ]

        with m.If(last_operation):
            m.d.sync += self.in_ready.eq(1)
        with m.If((last_operation & (~self.in_ready
                                     | (self.in_ready & self.in_valid)))
                  | (~work & self.in_valid)):
            m.d.sync += write_pointer.eq(write_pointer + 1)
        with m.If(self.in_valid & self.in_ready):
            m.d.sync += [
                self.in_ready.eq(~work | last_operation),
                work.eq(1),
            ]
        with m.If(~self.in_valid & self.in_ready & last_operation):
            m.d.sync += work.eq(0)

        m.d.comb += [
            macc0_re.strobe_in.eq(enable_macc0_q[1]),
            macc0_im.strobe_in.eq(enable_macc0_q[1]),
            macc1_re.strobe_in.eq(enable_macc1_q[1]),
            macc1_im.strobe_in.eq(enable_macc1_q[1]),
            macc0_re.first_acc.eq(first_acc_q[1]),
            macc0_im.first_acc.eq(first_acc_q[1]),
            macc1_re.first_acc.eq(first_acc_q[1]),
            macc1_im.first_acc.eq(first_acc_q[1]),
            samples.raddr0.eq(sample_addr0),
            samples.raddr1.eq(sample_addr1),
            samples.wren.eq(self.in_valid & self.in_ready),
            samples.waddr.eq(write_pointer),
            coeffs0.raddr.eq(coeff_counter),
            coeffs1.raddr.eq(coeff_counter),
        ]

        ram_delay = 2
        result_delay = macc0_re.delay + ram_delay
        macc_done_q = Signal(result_delay)

        m.d.sync += [
            macc_done_q.eq(Cat(last_acc & work, macc_done_q[:-1])),
            self.strobe_out.eq(macc_done_q[-1]),
        ]
        re0 = Signal(signed(self.ow))
        im0 = Signal(signed(self.ow))
        re1 = Signal(signed(self.ow))
        im1 = Signal(signed(self.ow))
        m.d.comb += [
            re0.eq(macc0_re.acc >> self.macc_trunc),
            im0.eq(macc0_im.acc >> self.macc_trunc),
            re1.eq(macc1_re.acc >> self.macc_trunc),
            im1.eq(macc1_im.acc >> self.macc_trunc),
        ]
        with m.If(macc_done_q[-1]):
            m.d.sync += [
                self.re_out.eq(re0 + re1),
                self.im_out.eq(im0 + im1),
            ]

        return m


# TODO: FIR2DSP and FIR4DSP have a lot of common control code.  Try to remove
# code duplication.
class FIR2DSP(Elaboratable):
    """Polyphase FIR decimator with 2 DSP48.

    This module is a polyphase FIR decimator that uses 2 DSP48. The length of
    the polyphase branches is defined by a runtime parameter called
    "operations". The FIR performs one multiply per operation, so the length of
    the polyphase branch is equal to the number of operations. The module needs
    at least "operations" clock cycles per input sample.

    The FIR length must be smaller or equal than 128, with the additional
    constraint that it must be a multiple of the decimation factor.

    Parameters
    ----------
    in_width : int
        Width of input IQ samples
    out_width : int
        Width of output IQ samples
    coeff_width : int
        FIR coefficients width.
    decim_width : int
        Width of ``decimation`` input.
    oper_width : int
        Width of ``operations_minus_one`` input.
    macc_trunc : int
        Truncation length for output of each MACC.
    len_log2 : int
        Maximum FIR length given as a log2 (by default, the maximum FIR length
        is 128).

    Attributes
    ----------
    coeff_waddr : Signal(len_log2), in
        Coefficient write address.
    coeff_wren : Signal(), in
        Coefficient write enable.
    coeff_wdata : Signal(coeff_width), in
        Coefficient write data.
    decimation : Signal(decim_width), in
        Decimation factor.
    operations_minus_one : Signal(oper_width), in
        Number of operations to perform minus one. This determines the length
        of the polyphase branches. The length is equal to the number of
        operations.
    re_in : Signal(signed(in_width)), in
        Input real part.
    im_in : Signal(signed(in_width)), in
        Input imaginary part.
    in_valid : Signal(), in
        Input valid (uses AXI-Stream handshaking).
    in_ready : Signal(), out
        Input ready (uses AXI-Stream handshaking).
    re_out : Signal(signed(out_width)), out
        Output real part.
    im_out : Signal(signed(out_width)), out
        Output imaginary part.
    strobe_out : Signal(), out
        Output strobe. It is asserted in the clock cycle when the output
        changes. The output is kept constant until the next time that
        ``strobe_out`` is asserted.
    """
    def __init__(self, *, in_width=16, out_width=16,
                 coeff_width=18, decim_width=6, oper_width=6,
                 macc_trunc=19, len_log2=7):
        self.iw = in_width
        self.ow = out_width
        self.coeff_width = coeff_width
        self.decim_width = decim_width
        self.oper_width = oper_width
        self.macc_trunc = macc_trunc
        self.len_log2 = len_log2

        self.coeff_waddr = Signal(len_log2)
        self.coeff_wren = Signal()
        self.coeff_wdata = Signal(coeff_width)
        self.decimation = Signal(decim_width)
        self.operations_minus_one = Signal(oper_width)

        self.re_in = Signal(signed(self.iw))
        self.im_in = Signal(signed(self.iw))
        self.in_valid = Signal()
        self.in_ready = Signal()

        self.re_out = Signal(signed(self.ow), reset_less=True)
        self.im_out = Signal(signed(self.ow), reset_less=True)
        self.strobe_out = Signal()

    def model(self, taps, decimation, re_in, im_in):
        assert len(taps) % decimation == 0
        taps = np.array(taps).reshape(-1, decimation)
        history = np.zeros(taps.size - 1, 'int')
        re_out = np.zeros(len(re_in) // decimation, 'int')
        im_out = np.zeros(len(re_in) // decimation, 'int')
        re_in = np.concatenate((history, re_in))
        im_in = np.concatenate((history, im_in))
        for j in range(re_out.size):
            # initial values for rounding
            acc_init = (2**(self.macc_trunc - 1)
                        if self.macc_trunc >= 1
                        else 0)
            re, im = [acc_init] * 2
            for k in range(taps.shape[0]):
                wr = re_in[(j + taps.shape[0] - k - 1)
                           * decimation:][:decimation]
                wi = im_in[(j + taps.shape[0] - k - 1)
                           * decimation:][:decimation]
                re += np.sum(wr[::-1] * taps[k])
                im += np.sum(wi[::-1] * taps[k])
            re_out[j] = clamp_nbits(re >> self.macc_trunc, self.ow)
            im_out[j] = clamp_nbits(im >> self.macc_trunc, self.ow)
        return re_out, im_out

    def elaborate(self, platform):
        m = Module()

        truncate_round = (
            self.macc_trunc if self.macc_trunc >= 1
            else None)
        m.submodules.macc_re = macc_re = Macc(
            self.iw, self.coeff_width, truncate_round=truncate_round)
        m.submodules.macc_im = macc_im = Macc(
            self.iw, self.coeff_width, truncate_round=truncate_round)
        m.submodules.samples = samples = SampleBuffer(
            2*self.iw, awidth=self.len_log2, two_read_ports=False)
        m.submodules.coeffs = coeffs = Coefficients(
            width=self.coeff_width, awidth=self.len_log2)

        m.d.comb += [
            macc_re.a.eq(samples.rdata0[:self.iw]),
            macc_im.a.eq(samples.rdata0[self.iw:]),
            macc_re.b.eq(coeffs.rdata),
            macc_im.b.eq(coeffs.rdata),
            samples.wdata.eq(Cat(self.re_in, self.im_in)),
            coeffs.waddr.eq(self.coeff_waddr),
            coeffs.wren.eq(self.coeff_wren),
            coeffs.wdata.eq(self.coeff_wdata),
        ]

        write_pointer = Signal(self.len_log2, reset_less=True)
        decimation_counter = Signal(self.decim_width)
        operation_counter = Signal(self.oper_width, reset=1)
        coeff_counter = Signal(self.len_log2, reset_less=True)
        sample_addr = Signal(self.len_log2, reset_less=True)
        work = Signal(reset=1)
        decimation_end_of_count = Signal(reset_less=True)
        last_operation = Signal(reset_less=True)
        last_acc = Signal(reset_less=True)
        m.d.comb += last_acc.eq(decimation_end_of_count & last_operation)
        first_acc = Signal()
        first_acc_q = Signal(2, reset_less=True)
        m.d.sync += first_acc_q.eq(Cat(first_acc, first_acc_q[:-1]))
        enable_macc_q = Signal(2, reset_less=True)
        enable_macc = Signal(reset_less=True)
        m.d.comb += enable_macc.eq(work)
        m.d.sync += [
            first_acc_q.eq(Cat(first_acc, first_acc_q[:-1])),
            enable_macc_q.eq(Cat(enable_macc, enable_macc_q[:-1])),
        ]

        with m.If(work):
            m.d.sync += [
                sample_addr.eq(
                    Mux(last_operation, write_pointer,
                        sample_addr - self.decimation)),
                first_acc.eq(last_acc),
                coeff_counter.eq(
                    Mux(last_acc, 0, coeff_counter + 1)),
                operation_counter.eq(Mux(last_operation,
                                         self.operations_minus_one,
                                         operation_counter - 1)),
                last_operation.eq((operation_counter == 1)
                                  | (self.operations_minus_one == 0)),
            ]
            with m.If(last_operation):
                m.d.sync += [
                    decimation_counter.eq(Mux(
                        last_acc, self.decimation, decimation_counter - 1)),
                    # decimation_counter should never reach 0 during normal
                    # operation, but it can when coming from reset. Using
                    # ~decimation_end_of_count here prevents
                    # decimation_end_of_count to be continuously stuck at 1
                    # when self.decimation is 2.
                    decimation_end_of_count.eq(
                        ~decimation_end_of_count
                        & ((decimation_counter == 2)
                           | (decimation_counter == 0))),
                ]

        with m.If(last_operation):
            m.d.sync += self.in_ready.eq(1)
        with m.If((last_operation & (~self.in_ready
                                     | (self.in_ready & self.in_valid)))
                  | (~work & self.in_valid)):
            m.d.sync += write_pointer.eq(write_pointer + 1)
        with m.If(self.in_valid & self.in_ready):
            m.d.sync += [
                self.in_ready.eq(~work | last_operation),
                work.eq(1),
            ]
        with m.If(~self.in_valid & self.in_ready & last_operation):
            m.d.sync += work.eq(0)

        m.d.comb += [
            macc_re.strobe_in.eq(enable_macc_q[1]),
            macc_im.strobe_in.eq(enable_macc_q[1]),
            macc_re.first_acc.eq(first_acc_q[1]),
            macc_im.first_acc.eq(first_acc_q[1]),
            samples.raddr0.eq(sample_addr),
            samples.wren.eq(self.in_valid & self.in_ready),
            samples.waddr.eq(write_pointer),
            coeffs.raddr.eq(coeff_counter),
        ]

        ram_delay = 2
        result_delay = macc_re.delay + ram_delay
        macc_done_q = Signal(result_delay)

        m.d.sync += [
            macc_done_q.eq(Cat(last_acc & work, macc_done_q[:-1])),
            self.strobe_out.eq(macc_done_q[-1]),
        ]
        re = Signal(signed(self.ow))
        im = Signal(signed(self.ow))
        m.d.comb += [
            re.eq(macc_re.acc >> self.macc_trunc),
            im.eq(macc_im.acc >> self.macc_trunc),
        ]
        with m.If(macc_done_q[-1]):
            m.d.sync += [
                self.re_out.eq(re),
                self.im_out.eq(im),
            ]

        return m


class FIRDecimator3Stage(Elaboratable):
    """Decimator with 3 FIR stages.

    This module is a decimator that is formed by chaining a FIR4DSP, a FIR2DSP,
    and a FIR4DSP. The design is inspired by the paper "Optimum FIR Digital
    Filter Implementations for Decimation, Interpolation, and Narrow-Band
    Filtering" by Crochiere and Rabiner. The second and third stages can be
    bypassed if desired.

    Parameters
    ----------
    in_width : int
        Width of input IQ samples.
    out_width : List[int]
        Output width of each FIR stage.
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
    in_valid : Signal(), in
        Input valid (uses AXI-Stream handshaking).
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
    def __init__(self, *, in_width=12, out_width=[16]*3,
                 coeff_width=18, decim_width=[7, 6, 7],
                 oper_width=[7, 6, 7], macc_trunc=[17, 18, 18]):
        self.iw = in_width
        self.ow = out_width
        self.coeff_width = coeff_width
        self.decim_width = decim_width
        self.oper_width = oper_width
        self.macc_trunc = macc_trunc

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
        self.in_valid = Signal()
        self.in_ready = Signal()

        self.re_out = Signal(signed(self.ow[-1]), reset_less=True)
        self.im_out = Signal(signed(self.ow[-1]), reset_less=True)
        self.strobe_out = Signal()

    def elaborate(self, platform):
        m = Module()

        m.submodules.stage1 = stage1 = FIR4DSP(
            in_width=self.iw, out_width=self.ow[0],
            coeff_width=self.coeff_width, decim_width=self.decim_width[0],
            oper_width=self.oper_width[0], macc_trunc=self.macc_trunc[0],
            len_log2=8)
        m.submodules.stage2 = stage2 = FIR2DSP(
            in_width=self.ow[0], out_width=self.ow[1],
            coeff_width=self.coeff_width, decim_width=self.decim_width[1],
            oper_width=self.oper_width[1], macc_trunc=self.macc_trunc[1],
            len_log2=7)
        m.submodules.stage3 = stage3 = FIR4DSP(
            in_width=self.ow[1], out_width=self.ow[2],
            coeff_width=self.coeff_width, decim_width=self.decim_width[2],
            oper_width=self.oper_width[2], macc_trunc=self.macc_trunc[2],
            len_log2=8)
        stages = [stage1, stage2, stage3]

        for j, stage in enumerate(stages):
            m.d.comb += [
                stage.coeff_waddr.eq(self.coeff_waddr),
                stage.coeff_wdata.eq(self.coeff_wdata),
                stage.coeff_wren.eq(self.coeff_wren
                                    & (self.coeff_waddr[-2:] == j)),
                ]
        m.d.comb += [
            stage1.decimation.eq(self.decimation1),
            stage2.decimation.eq(self.decimation2),
            stage3.decimation.eq(self.decimation3),
            stage1.operations_minus_one.eq(self.operations_minus_one1),
            stage2.operations_minus_one.eq(self.operations_minus_one2),
            stage3.operations_minus_one.eq(self.operations_minus_one3),
            stage1.odd_operations.eq(self.odd_operations1),
            stage3.odd_operations.eq(self.odd_operations3),
            stage1.re_in.eq(self.re_in),
            stage1.im_in.eq(self.im_in),
            stage1.in_valid.eq(self.in_valid),
            self.in_ready.eq(stage1.in_ready),
        ]

        stage2_re_in = Signal(signed(self.ow[0]), reset_less=True)
        stage2_im_in = Signal(signed(self.ow[0]), reset_less=True)
        stage2_in_valid = Signal(reset_less=True)
        m.d.comb += [
            stage2.re_in.eq(stage2_re_in),
            stage2.im_in.eq(stage2_im_in),
            stage2.in_valid.eq(stage2_in_valid),
        ]
        with m.If(stage2.in_ready):
            m.d.sync += stage2_in_valid.eq(0)
        with m.If(stage1.strobe_out):
            m.d.sync += [
                stage2_in_valid.eq(~self.bypass2),
                stage2_re_in.eq(stage1.re_out),
                stage2_im_in.eq(stage1.im_out),
            ]

        stage3_re_in = Signal(signed(self.ow[1]), reset_less=True)
        stage3_im_in = Signal(signed(self.ow[1]), reset_less=True)
        stage3_in_valid = Signal(reset_less=True)
        m.d.comb += [
            stage3.re_in.eq(stage3_re_in),
            stage3.im_in.eq(stage3_im_in),
            stage3.in_valid.eq(stage3_in_valid),
        ]
        with m.If(stage3.in_ready):
            m.d.sync += stage3_in_valid.eq(0)
        with m.If(self.bypass2):
            with m.If(stage1.strobe_out):
                m.d.sync += [
                    stage3_re_in.eq(stage1.re_out),
                    stage3_im_in.eq(stage1.im_out),
                    stage3_in_valid.eq(~self.bypass3),
                ]
        with m.Else():
            with m.If(stage2.strobe_out):
                m.d.sync += [
                    stage3_re_in.eq(stage2.re_out),
                    stage3_im_in.eq(stage2.im_out),
                    stage3_in_valid.eq(~self.bypass3),
                ]

        with m.If(self.bypass3):
            with m.If(self.bypass2):
                m.d.sync += [
                    self.re_out.eq(stage1.re_out),
                    self.im_out.eq(stage1.im_out),
                    self.strobe_out.eq(stage1.strobe_out),
                ]
            with m.Else():
                m.d.sync += [
                    self.re_out.eq(stage2.re_out),
                    self.im_out.eq(stage2.im_out),
                    self.strobe_out.eq(stage2.strobe_out),
                ]
        with m.Else():
            m.d.sync += [
                self.re_out.eq(stage3.re_out),
                self.im_out.eq(stage3.im_out),
                self.strobe_out.eq(stage3.strobe_out),
            ]

        return m


if __name__ == '__main__':
    fir = FIRDecimator3Stage()
    amaranth.cli.main(
        fir, ports=[
            fir.coeff_waddr, fir.coeff_wren, fir.coeff_wdata,
            fir.decimation1, fir.decimation2, fir.decimation3,
            fir.bypass2, fir.bypass3,
            fir.operations_minus_one1, fir.operations_minus_one2,
            fir.operations_minus_one3,
            fir.odd_operations1, fir.odd_operations3,
            fir.re_in, fir.im_in, fir.in_valid, fir.in_ready,
            fir.re_out, fir.im_out, fir.strobe_out])
