#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
from amaranth.lib.memory import Memory
import amaranth.back.verilog
import numpy as np
import scipy.signal

import operator

from .cmult import Cmult, Cmult3x
from .mult2x import Mult2x
from .pluto_platform import PlutoPlatform
from .util import clamp_nbits


class R2SDF(Elaboratable):
    """Radix-2 Single-Delay-Feedback butterfly

    Parameters
    ----------
    order : int
        The order of the butterfly determines the how many consecutive input
        elements the butterfly needs to consume to be able to produce all of
        its corresponding outputs. This number is ``2**order``. For a radix-2
        DIF FFT of size ``2**n``, the orders of the butterflies are ``n``,
        ``n-1``, ..., ``2``, ``1`` .
    width_in : int
        Width of the input samples.
    truncate : int
        Number of bits to be truncated in the output. Since the butterfly
        adds/subtracts two elements, there is a bit growth of one bit in
        the output. By default, the output is not truncated, so its width
        is ``width_in + 1``. A positive value of ``truncation`` can be used
        to truncate the LSBs of the output and obtain an output width of
        ``width_in + 1 - truncation``.
    bf2ii : bool
        This should be set to True to implement the second part of an
        R2^2SDF butterfly. This part absorbes the sign change of the
        multiplication by -i in a TwiddleI by reversing the signs of
        the calculations for the imaginary parts.
    storage : str
        Selects the storage mode for the shift registers. There are three
        possible storage mode:
        * ``'distributed'`` uses distributed memory (flip-flops or LUTMs
          depending on synthesis)
        * ``'bram'`` uses BRAMs with 1 clock cycles of read latency
        * ``'auto'`` chooses 'distributed' or 'bram' depending on ``order``.
    use_bram_reg : bool
        This should be set to True if the output register for the BRAM
        is to be used, giving a read latency of 2 clock cycles. This
        parameter only applies when BRAM storage is used.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    mux_control : Signal(), in
        Controls the multiplexers of the R2SDF. The ``mux_control`` should
        be low for the first ``2**(order-1)`` input samples, and high for the
        next ``2**(order-1)`` input samples.
    i_control : Signal(), in
        Only present when ``bf2ii == True``. This should be high when the
        butterfly performs the sign change of the multiplication by -i.
    bram_raddr : Signal(order-1), in
        Only present for ``'bram'`` storage mode (even when selected with
        ``'auto'``). A counter that iterates through all the BRAM addresses.
        Its value must equal ``bram_waddr + 1`` if ``use_bram_reg`` is False,
        or ``bram_waddr + 2`` if ``use_bram_reg`` is True.
    bram_waddr : Signal(order-1), in
        Only present for ``'bram'`` storage mode (even when selected with
        ``'auto'``). A counter that iterates through all the BRAM addresses.
    re_in : Signal(signed(width_in)), in
        Real part of the input sample.
    im_in : Signal(signed(width_in)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(width_in+1-truncate)), out
        Real part of the output sample.
    im_out : Signal(signed(width_in+1-truncate)), out
        Imaginary part of the output sample.
    """
    def __init__(self, order, width_in, truncate=0, bf2ii=False,
                 storage='auto', use_bram_reg=False):
        self.radix_log2 = 1
        self.order = order
        self.storage = (
            storage if storage != 'auto' else self.auto_storage_rule())
        if self.storage == 'bram' and self.order == 1:
            raise ValueError('order 1 cannot be implemented with BRAM')
        self.use_bram_reg = self.storage == 'bram' and use_bram_reg
        self.w = width_in
        self.w_out = width_in + 1 - truncate
        self.trunc = truncate
        self.bf2ii = bf2ii

        self.clken = Signal()
        self.mux_control = Signal()
        if self.bf2ii:
            self.i_control = Signal()
        if self.storage == 'bram':
            self.bram_raddr = Signal(self.order - 1)
            self.bram_waddr = Signal(self.order - 1)
        self.re_in = Signal(signed(self.w))
        self.im_in = Signal(signed(self.w))
        self.re_out = Signal(signed(self.w_out), reset_less=True)
        self.im_out = Signal(signed(self.w_out), reset_less=True)

    @property
    def delay(self):
        return self.buff_len

    @property
    def buff_len(self):
        return 2**(self.order-1)

    @property
    def model_vlen(self):
        return 2**self.order

    def auto_storage_rule(self):
        return 'bram' if self.order >= 9 else 'distributed'

    def model(self, re_in, im_in):
        v = self.model_vlen
        re_in, im_in = (np.array(x, 'int').reshape(-1, 2, v // 2)
                        for x in [re_in, im_in])
        re_out, im_out = [
            clamp_nbits(
                np.concatenate(
                    (x[:, 0] + x[:, 1], x[:, 0] - x[:, 1]),
                    axis=-1).ravel() >> self.trunc,
                self.w_out)
            for x in [re_in, im_in]]
        return re_out, im_out

    def elaborate(self, platform):
        m = Module()

        w_buff = max(self.w, self.w_out)
        buff_re_in = Signal(w_buff)
        buff_im_in = Signal(w_buff)
        if self.storage == 'distributed':
            # Distributed (flip-flop / LUTM) storage
            buff_re = [Signal(signed(w_buff), name=f'buff_re_{i}',
                              reset_less=True)
                       for i in range(self.buff_len)]
            buff_im = [Signal(signed(w_buff), name=f'buff_im_{i}',
                              reset_less=True)
                       for i in range(self.buff_len)]
            buff_re_out = buff_re[-1]
            buff_im_out = buff_im[-1]
            with m.If(self.clken):
                m.d.sync += [buff[j].eq(buff[j - 1])
                             for buff in [buff_re, buff_im]
                             for j in range(1, self.buff_len)]
                m.d.sync += [buff_re[0].eq(buff_re_in),
                             buff_im[0].eq(buff_im_in)]
        else:
            # BRAM storage
            bram_w = 2 * w_buff
            m.submodules.buff_mem = buff_mem = (
                Memory(shape=bram_w, depth=self.buff_len,
                       init=[],
                       attrs={'ram_style': 'block'}))
            rdport = buff_mem.read_port()
            wrport = buff_mem.write_port()
            if self.use_bram_reg:
                # BRAM output register
                rdata = Signal(bram_w, reset_less=True)
                with m.If(self.clken):
                    m.d.sync += rdata.eq(rdport.data)
            else:
                rdata = rdport.data
            buff_re_out = rdata[:w_buff]
            buff_im_out = rdata[w_buff:]
            m.d.comb += [
                rdport.en.eq(self.clken),
                wrport.en.eq(self.clken),
                rdport.addr.eq(self.bram_raddr),
                wrport.addr.eq(self.bram_waddr),
                wrport.data.eq(Cat(buff_re_in, buff_im_in)),
            ]

        # Select operations for the imaginary part, depending on whether we
        # are doing a bf2ii butterfly or not. If we are not bf2ii, we act as
        # if self.i_control = 0 always.
        op_plus, op_minus = (
            op(buff_im_out[:self.w].as_signed(),
               self.im_in).as_signed() >> self.trunc
            for op in (operator.add, operator.sub))

        buff_im_next = (
            Mux(self.i_control, op_plus, op_minus)
            if self.bf2ii
            else op_minus)
        out_im = (
            Mux(self.i_control, op_minus, op_plus)
            if self.bf2ii
            else op_plus)

        m.d.comb += [
            buff_re_in.eq(
                Mux(self.mux_control,
                    (buff_re_out[:self.w].as_signed()
                     - self.re_in).as_signed()
                    >> self.trunc,
                    self.re_in)),
            buff_im_in.eq(
                Mux(self.mux_control,
                    buff_im_next,
                    self.im_in)),
            self.re_out.eq(Mux(self.mux_control,
                               (buff_re_out[:self.w].as_signed()
                                + self.re_in).as_signed()
                               >> self.trunc,
                               buff_re_out)),
            self.im_out.eq(Mux(self.mux_control,
                               out_im,
                               buff_im_out))
        ]

        return m


class R4SDF(Elaboratable):
    """Radix-4 Single-Delay-Feedback butterfly

    Parameters
    ----------
    order : int
        The order of the butterfly determines the how many consecutive input
        elements the butterfly needs to consume to be able to produce all of
        its corresponding outputs. This number is ``4**order``. For a DIF
        FFT of size ``4**n``, the orders of the butterflies are ``n``,
        ``n-2``, ..., ``2``, ``1``.
    width_in : int
        Width of the input samples.
    truncate : int
        Number of bits to be truncated in the output. Since the butterfly
        adds/subtracts 4 elements, there is a bit growth of 2 bits in
        the output. By default, the output is not truncated, so its width
        is ``width_in + 2``. A positive value of ``truncation`` can be used
        to truncate the LSBs of the output and obtain an output width of
        ``width_in + 2 - truncation``.
    storage : str
        Selects the storage mode for the shift registers. There are three
        possible storage mode:
        * ``'distributed'`` uses distributed memory (flip-flops or LUTMs
          depending on synthesis)
        * ``'bram'`` uses BRAMs with 1 clock cycles of read latency
        * ``'auto'`` chooses 'distributed' or 'bram' depending on ``order``.
    use_bram_reg : bool
        This should be set to True if the output register for the BRAM
        is to be used, giving a read latency of 2 clock cycles. This
        parameter only applies when BRAM storage is used.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    mux_control : Signal(), in
        Controls the multiplexers of the R4SDF. The ``mux_control`` should
        be low for the first ``3*4**(order-1)`` input samples, and high for the
        next ``4**(order-1)`` input samples.
    bram_raddr : Signal(order-1), in
        Only present for ``'bram'`` storage mode (even when selected with
        ``'auto'``). A counter that iterates through all the BRAM addresses.
        Its value must equal ``bram_waddr + 1`` if ``use_bram_reg`` is False,
        or ``bram_waddr + 2`` if ``use_bram_reg`` is True.
    bram_waddr : Signal(2*(order-1)), in
        Only present for ``'bram'`` storage mode (even when selected with
        ``'auto'``). A counter that iterates through all the BRAM addresses.
    re_in : Signal(signed(width_in)), in
        Real part of the input sample.
    im_in : Signal(signed(width_in)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(width_in+2-truncate)), out
        Real part of the output sample.
    im_out : Signal(signed(width_in+2-truncate)), out
        Imaginary part of the output sample.
    """
    def __init__(self, order, width_in, truncate=0, storage='auto',
                 use_bram_reg=False):
        self.radix_log2 = 2
        self.order = order
        self.storage = (
            storage if storage != 'auto' else self.auto_storage_rule())
        if self.storage == 'bram' and self.order == 1:
            raise ValueError('order 1 cannot be implemented with BRAM')
        self.use_bram_reg = self.storage == 'bram' and use_bram_reg
        self.w = width_in
        self.w_out = width_in + 2 - truncate
        self.trunc = truncate

        self.clken = Signal()
        self.mux_control = Signal()
        if self.storage == 'bram':
            self.bram_raddr = Signal(2*(self.order - 1))
            self.bram_waddr = Signal(2*(self.order - 1))
        self.re_in = Signal(signed(self.w))
        self.im_in = Signal(signed(self.w))
        self.re_out = Signal(signed(self.w_out), reset_less=True)
        self.im_out = Signal(signed(self.w_out), reset_less=True)

    @property
    def delay(self):
        return self.num_buffs * self.buff_len

    @property
    def buff_len(self):
        return 4**(self.order-1)

    @property
    def num_buffs(self):
        return 3

    @property
    def model_vlen(self):
        return 4**self.order

    def auto_storage_rule(self):
        return 'bram' if self.order >= 4 else 'distributed'

    def model(self, re_in, im_in):
        v = self.model_vlen
        re_in, im_in = (np.array(x, 'int').reshape(-1, 4, v // 4)
                        for x in [re_in, im_in])
        re_out = clamp_nbits(
            np.concatenate(
                (re_in[:, 0] + re_in[:, 1] + re_in[:, 2] + re_in[:, 3],
                 re_in[:, 0] + im_in[:, 1] - re_in[:, 2] - im_in[:, 3],
                 re_in[:, 0] - re_in[:, 1] + re_in[:, 2] - re_in[:, 3],
                 re_in[:, 0] - im_in[:, 1] - re_in[:, 2] + im_in[:, 3]),
                axis=-1).ravel() >> self.trunc,
            self.w_out)
        im_out = clamp_nbits(
            np.concatenate(
                (im_in[:, 0] + im_in[:, 1] + im_in[:, 2] + im_in[:, 3],
                 im_in[:, 0] - re_in[:, 1] - im_in[:, 2] + re_in[:, 3],
                 im_in[:, 0] - im_in[:, 1] + im_in[:, 2] - im_in[:, 3],
                 im_in[:, 0] + re_in[:, 1] - im_in[:, 2] - re_in[:, 3]),
                axis=-1).ravel() >> self.trunc,
            self.w_out)
        return re_out, im_out

    def elaborate(self, platform):
        m = Module()

        w_buff = max(self.w, self.w_out)
        buffs_re_in = [Signal(w_buff, name=f'buff{j}_re_in')
                       for j in range(self.num_buffs)]
        buffs_im_in = [Signal(w_buff, name=f'buff{j}_im_in')
                       for j in range(self.num_buffs)]
        if self.storage == 'distributed':
            # Distributed (flip-flop / LUTM) storage
            buffs_re = [
                [Signal(signed(w_buff), name=f'buff{k}_re_{j}',
                        reset_less=True)
                 for j in range(self.buff_len)]
                for k in range(self.num_buffs)]
            buffs_im = [
                [Signal(signed(w_buff), name=f'buff{k}_im_{j}',
                        reset_less=True)
                 for j in range(self.buff_len)]
                for k in range(self.num_buffs)]
            buffs = [buffs_re, buffs_im]
            buffs_re_out = [b[-1] for b in buffs_re]
            buffs_im_out = [b[-1] for b in buffs_im]
            with m.If(self.clken):
                # shift each of the buffers
                m.d.sync += [
                    b[j][k].eq(b[j][k - 1])
                    for b in buffs
                    for j in range(self.num_buffs)
                    for k in range(1, self.buff_len)
                ]
                # input for each buffer
                m.d.sync += [
                    buffs_re[j][0].eq(buffs_re_in[j])
                    for j in range(self.num_buffs)
                ]
                m.d.sync += [
                    buffs_im[j][0].eq(buffs_im_in[j])
                    for j in range(self.num_buffs)
                ]
        else:
            # BRAM storage
            bram_w = 2 * self.num_buffs * w_buff
            m.submodules.buff_mem = buff_mem = (
                Memory(shape=bram_w,
                       depth=self.buff_len,
                       init=[],
                       attrs={'ram_style': 'block'}))
            rdport = buff_mem.read_port()
            wrport = buff_mem.write_port()
            if self.use_bram_reg:
                # BRAM output register
                rdata = Signal(bram_w, reset_less=True)
                with m.If(self.clken):
                    m.d.sync += rdata.eq(rdport.data)
            else:
                rdata = rdport.data
            buffs_re_out = [rdata[2*j*w_buff:(2*j+1)*w_buff]
                            for j in range(self.num_buffs)]
            buffs_im_out = [rdata[(2*j+1)*w_buff:(2*j+2)*w_buff]
                            for j in range(self.num_buffs)]
            m.d.comb += [
                rdport.en.eq(self.clken),
                wrport.en.eq(self.clken),
                rdport.addr.eq(self.bram_raddr),
                wrport.addr.eq(self.bram_waddr),
                wrport.data.eq(Cat(
                    *[a for r, i in zip(buffs_re_in, buffs_im_in)
                      for a in [r, i]])),
            ]

        with m.If(self.mux_control):
            # compute 4 outputs and push into buffers
            x0r = buffs_re_out[2][:self.w].as_signed()
            x1r = buffs_re_out[1][:self.w].as_signed()
            x2r = buffs_re_out[0][:self.w].as_signed()
            x3r = self.re_in
            x0i = buffs_im_out[2][:self.w].as_signed()
            x1i = buffs_im_out[1][:self.w].as_signed()
            x2i = buffs_im_out[0][:self.w].as_signed()
            x3i = self.im_in
            m.d.comb += [
                # x0 + x1 + x2 + x3
                self.re_out.eq((x0r + x1r + x2r + x3r).as_signed()
                               >> self.trunc),
                self.im_out.eq((x0i + x1i + x2i + x3i).as_signed()
                               >> self.trunc),
                # x0 - i*x1 - x2 + i*x3
                buffs_re_in[2].eq((x0r + x1i - x2r - x3i).as_signed()
                                  >> self.trunc),
                buffs_im_in[2].eq((x0i - x1r - x2i + x3r).as_signed()
                                  >> self.trunc),
                # x0 - x1 + x2 - x3
                buffs_re_in[1].eq((x0r - x1r + x2r - x3r).as_signed()
                                  >> self.trunc),
                buffs_im_in[1].eq((x0i - x1i + x2i - x3i).as_signed()
                                  >> self.trunc),
                # x0 + i*x1 - x2 - i*x3
                buffs_re_in[0].eq((x0r - x1i - x2r + x3i).as_signed()
                                  >> self.trunc),
                buffs_im_in[0].eq((x0i + x1r - x2i - x3r).as_signed()
                                  >> self.trunc),
            ]
        with m.Else():
            # shift buffers around
            m.d.comb += [
                self.re_out.eq(buffs_re_out[-1]),
                self.im_out.eq(buffs_im_out[-1])]
            m.d.comb += [
                buffs_re_in[j].eq(buffs_re_out[j - 1])
                for j in range(1, self.num_buffs)]
            m.d.comb += [
                buffs_im_in[j].eq(buffs_im_out[j - 1])
                for j in range(1, self.num_buffs)]
            m.d.comb += [
                buffs_re_in[0].eq(self.re_in),
                buffs_im_in[0].eq(self.im_in)]

        return m


class R22SDF(Elaboratable):
    """Radix-2^2 Single-Delay-Feedback butterfly

    This implements a radix-4 butterfly by concatenating 2 R2SDF's. There
    is formally a :class: ``TwiddleI`` between both R2SDF's, but the sign
    change given by the multiplication by -i is absorbed in the operations
    of the second R2SDF using the ``bf2ii`` parameter, and the remaining
    part of the multiplication by -i is implemented as a conmutator that
    swaps the real and imaginary parts.

    Parameters
    ----------
    order : int
        The order of the butterfly determines the how many consecutive input
        elements the butterfly needs to consume to be able to produce all of
        its corresponding outputs. This number is ``4**order``. For a DIF
        FFT of size ``4**n``, the orders of the butterflies are ``n``,
        ``n-2``, ..., ``2``, ``1``.
    width_in : int
        Width of the input samples.
    truncate : [int, int]
        Number of bits to be truncated in the output of each of the
        two butterflies. Since each butterfly adds/subtracts 2 elements,
        there is a bit growth of 1 bit in each butterfly.
    storage : str
        Selects the storage mode for the shift registers. There are three
        possible storage mode:
        * ``'distributed'`` uses distributed memory (flip-flops or LUTMs
          depending on synthesis)
        * ``'bram'`` uses BRAMs with 1 clock cycles of read latency
        * ``'auto'`` chooses 'distributed' or 'bram' depending on ``order``.
    use_bram_reg : bool
        This should be set to True if the output register for the BRAM
        is to be used, giving a read latency of 2 clock cycles. This
        parameter only applies when BRAM storage is used.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    mux_count : Signal(2), in
        Controls the multiplexers of the R2^2SDF. The ``mux_count`` should be
        0 for the first ``4**(order-1)`` input samples, 1 for the next
        ``4**(order-1)`` input samples, etc.
    bram_raddr : Signal(order-1), in
        Only present for ``'bram'`` storage mode (even when selected with
        ``'auto'``). A counter that iterates through all the BRAM addresses.
        Its value must equal ``bram_waddr + 1`` if ``use_bram_reg`` is False,
        or ``bram_waddr + 2`` if ``use_bram_reg`` is True.
    bram_waddr : Signal(order-1), in
        Only present for ``'bram'`` storage mode (even when selected with
        ``'auto'``). A counter that iterates through all the BRAM addresses.
    re_in : Signal(signed(width_in)), in
        Real part of the input sample.
    im_in : Signal(signed(width_in)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(width_in+2-truncate[0]-truncate[1])), out
        Real part of the output sample.
    im_out : Signal(signed(width_in+2-truncate[0]-truncate[1])), out
        Imaginary part of the output sample.
    """
    def __init__(self, order, width_in, truncate=[0, 0], storage='auto',
                 use_bram_reg=False):
        self.radix_log2 = 2
        self.order = order
        self.w_in = width_in
        if len(truncate) != 2:
            raise ValueError("R22SDF needs a list of two int's as truncate")
        self.trunc0 = truncate[0]
        self.trunc1 = truncate[1]
        self.w_out = width_in + 2 - self.trunc0 - self.trunc1

        self.clken = Signal()
        self.mux_count = Signal(2)
        self.re_in = Signal(signed(self.w_in))
        self.im_in = Signal(signed(self.w_in))
        self.re_out = Signal(signed(self.w_out), reset_less=True)
        self.im_out = Signal(signed(self.w_out), reset_less=True)

        self.bfly0 = R2SDF(2 * self.order, self.w_in, truncate=self.trunc0,
                           bf2ii=False, storage=storage,
                           use_bram_reg=use_bram_reg)
        self.w_inter = self.w_in + 1 - self.trunc0
        self.bfly1 = R2SDF(2 * self.order - 1, self.w_inter,
                           truncate=self.trunc1, bf2ii=True, storage=storage,
                           use_bram_reg=use_bram_reg)

        if self.storage == 'bram':
            self.bram_raddr = Signal(2 * self.order - 1)
            self.bram_waddr = Signal(2 * self.order - 1)
        self.use_bram_reg = self.storage == 'bram' and use_bram_reg

    @property
    def storage(self):
        s0 = self.bfly0.storage
        s1 = self.bfly1.storage
        assert s0 == s1
        return s0

    @property
    def delay(self):
        # The + 1 accounts for a register between both SDF's
        return self.bfly0.delay + self.bfly1.delay + 1

    @property
    def model_vlen(self):
        return 4**self.order

    def model(self, re_in, im_in):
        v = self.model_vlen
        re_in, im_in = (np.array(x, 'int').reshape(-1, 4, v // 4)
                        for x in [re_in, im_in])
        re_inter = clamp_nbits(
            np.concatenate(
                (re_in[:, 0] + re_in[:, 2], re_in[:, 1] + re_in[:, 3],
                 re_in[:, 0] - re_in[:, 2], im_in[:, 1] - im_in[:, 3]),
                axis=-1).ravel() >> self.trunc0,
            self.w_inter)
        im_inter = clamp_nbits(
            np.concatenate(
                (im_in[:, 0] + im_in[:, 2], im_in[:, 1] + im_in[:, 3],
                 im_in[:, 0] - im_in[:, 2], re_in[:, 1] - re_in[:, 3]),
                axis=-1).ravel() >> self.trunc0,
            self.w_inter)
        re_inter, im_inter = (x.reshape(-1, 4, v // 4)
                              for x in [re_inter, im_inter])
        re_out = clamp_nbits(
            np.concatenate(
                (re_inter[:, 0] + re_inter[:, 1],
                 re_inter[:, 0] - re_inter[:, 1],
                 re_inter[:, 2] + re_inter[:, 3],
                 re_inter[:, 2] - re_inter[:, 3]),
                axis=-1).ravel() >> self.trunc1,
            self.w_out)
        im_out = clamp_nbits(
            np.concatenate(
                (im_inter[:, 0] + im_inter[:, 1],
                 im_inter[:, 0] - im_inter[:, 1],
                 im_inter[:, 2] - im_inter[:, 3],
                 im_inter[:, 2] + im_inter[:, 3]),
                axis=-1).ravel() >> self.trunc1,
            self.w_out)
        return re_out, im_out

    def elaborate(self, platform):
        m = Module()
        m.submodules.bfly0 = self.bfly0
        m.submodules.bfly1 = self.bfly1

        # Interstage register.
        re_inter = Signal(signed(self.w_inter), reset_less=True)
        im_inter = Signal(signed(self.w_inter), reset_less=True)

        # The + 1 here accounts for the interstage register.
        bfly1_input_delay = self.bfly0.delay + 1
        swap_delay = Signal(bfly1_input_delay, reset_less=True)
        bfly1_mux_delay = Signal(bfly1_input_delay, reset_less=True)

        with m.If(self.clken):
            m.d.sync += [
                # We use swap_delay[-2] rather than [-1] because the swap is
                # before the interstage register.
                re_inter.eq(Mux(
                    swap_delay[-2],
                    self.bfly0.im_out,
                    self.bfly0.re_out)),
                im_inter.eq(Mux(
                    swap_delay[-2],
                    self.bfly0.re_out,
                    self.bfly0.im_out)),
                swap_delay.eq(Cat(self.mux_count.all(), swap_delay[:-1])),
                bfly1_mux_delay.eq(Cat(self.mux_count[0],
                                       bfly1_mux_delay[:-1])),
            ]

        m.d.comb += [
            self.bfly0.re_in.eq(self.re_in),
            self.bfly0.im_in.eq(self.im_in),
            self.bfly1.re_in.eq(re_inter),
            self.bfly1.im_in.eq(im_inter),

            self.bfly0.mux_control.eq(self.mux_count[1]),
            self.bfly1.mux_control.eq(bfly1_mux_delay[-1]),
            self.bfly1.i_control.eq(swap_delay[-1]),

            self.re_out.eq(self.bfly1.re_out),
            self.im_out.eq(self.bfly1.im_out),
        ]
        for bfly in [self.bfly0, self.bfly1]:
            m.d.comb += bfly.clken.eq(self.clken)
            if self.storage == 'bram':
                # Note that bfly1 doesn't use the MSB of the addresses, because
                # it has a smaller order than bfly0.
                m.d.comb += [
                    bfly.bram_raddr.eq(self.bram_raddr),
                    bfly.bram_waddr.eq(self.bram_waddr),
                ]

        return m


class TwiddleI(Elaboratable):
    """Multiplication by i twiddle factor

    This twiddle factor can only multiply by 1 or i.

    Parameters
    ----------
    width : int
        Width of the input and output samples.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    twiddle_index : Signal(2), in
        When ``twiddle_index == 3``, the input is multiplied by -i. Otherwise,
        the input is multplied by 1.
    re_in : Signal(signed(width)), in
        Real part of the input sample.
    im_in : Signal(signed(width)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(width)), out
        Real part of the output sample.
    im_out : Signal(signed(width)), out
        Imaginary part of the output sample.
    """
    def __init__(self, width):
        self.w = width
        self.radix_log2 = 1
        self.order = 2

        self.clken = Signal()
        self.twiddle_index = Signal(2)
        self.re_in = Signal(signed(self.w))
        self.im_in = Signal(signed(self.w))
        self.re_out = Signal(signed(self.w), reset_less=True)
        self.im_out = Signal(signed(self.w), reset_less=True)

    @property
    def delay(self):
        return 1

    @property
    def twiddle_index_advance(self):
        return 0

    @property
    def model_vlen(self):
        return 4

    def model(self, re_in, im_in):
        v = self.model_vlen
        re_in, im_in = (np.array(x, 'int').reshape(-1, v)
                        for x in [re_in, im_in])
        re_out = re_in.copy()
        im_out = im_in.copy()
        re_out[:, 3] = im_in[:, 3]
        im_out[:, 3] = -re_in[:, 3]
        return re_out.ravel(), im_out.ravel()

    def elaborate(self, platform):
        m = Module()
        with m.If(self.clken):
            m.d.sync += [
                self.re_out.eq(self.re_in),
                self.im_out.eq(self.im_in)
            ]
            with m.If(self.twiddle_index == 3):
                m.d.sync += [
                    self.re_out.eq(self.im_in),
                    self.im_out.eq(-self.re_in)
                ]
        return m


class Twiddle(Elaboratable):
    """Twiddle factor multiplication

    This module contains the memory storing twiddle factors and the complex
    multiplier.

    Parameters
    ----------
    order : int
        The order of the twiddle factor, together with the ``radix_log2``
        determines the roots of unity to consider. If
        ``w = exp(2*pi*i/2**(radix_log2*order)))`,
        the list of twiddle factors is
        ``[w**(j*k) for j in range(2**radix_log2)
           for k in range(2**((radix_log2-1)*order)]``.

        Note that the period of the sequence produced is
        ``2**(radix_log2*order)``.
    radix_log2 : int
        See ``order``.
    sample_width : int
        Width of the input samples.
    twiddle_width : int
        Width of the twiddle factors to store.
    storage : str
        Storage mode for the twiddle factors. There are three possible storage
        modes:
            * ``'lut'`` uses combinational LUTs
            * ``'bram'`` uses BRAMs with 2 clock cycles of latency
            * ``'auto'`` chooses 'lut' or 'bram' depending on the ``order``
    r22_mode : bool
        If this is enabled, twiddles are generated for an R22SDF rather than
        for an R4SDF. Due to the differences in output reordering of an R22SDF,
        the twiddles are
        ``[w**(j*k) for j in [0, 2, 1, 3]
           for k in range(2**((radix_log2-1)*order)]``.
        This mode can only be used with ``radix_log2 = 2``.
    cmult3x : bool
        If this is enabled, a single-multiplier running at 3x the clock
        frequency is used for the complex multiplier, instead of 3 multipliers.
    domain_3x : Optional[str]
        Clock domain for the 3x clock. This is only necessary when
        `cmult3x == True`.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    common_edge : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock. This is only present
        when cmult3x is enabled.
    twiddle_index : Signal(...), in
        Selects the twiddle factor to use. This should be a counter modulo
        ``2**(radix_log2 * order)``
    re_in : Signal(signed(sample_width)), in
        Real part of the input sample.
    im_in : Signal(signed(sample_width)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(sample_width)), out
        Real part of the output sample.
    im_out : Signal(signed(sample_width)), out
        Imaginary part of the output sample.
    """
    def __init__(self, order, radix_log2, sample_width, twiddle_width,
                 storage='auto', r22_mode=False, cmult3x=False,
                 domain_3x=None):
        if cmult3x and domain_3x is None:
            raise ValueError('domain_3x must be specified for cmult3x')
        if r22_mode and radix_log2 != 2:
            raise ValueError('r22_mode can only be used with radix_log2 = 2')
        if storage not in ['auto', 'bram', 'lut']:
            raise ValueError(f'invalid storage class for Twiddle: {storage}')
        self.order = order
        self.radix_log2 = radix_log2
        self.r22_mode = r22_mode
        self.sw = sample_width
        self.tw = twiddle_width
        self.outw = sample_width
        self.cmult3x = cmult3x
        self._3x = domain_3x

        self.clken = Signal()
        if domain_3x is not None:
            self.common_edge = Signal()
        self.twiddle_index = Signal(radix_log2 * order)
        self.re_in = Signal(signed(self.sw))
        self.im_in = Signal(signed(self.sw))
        self.re_out = Signal(signed(self.outw), reset_less=True)
        self.im_out = Signal(signed(self.outw), reset_less=True)
        self.storage = (
            storage if storage != 'auto' else self.auto_storage_rule())
        cmult_opts = {
            'a_width': self.sw,
            'b_width': self.tw,
            'truncate': self.twiddle_scale_clog2(),
        }
        self.cmult = (
            Cmult3x(self._3x, **cmult_opts) if self.cmult3x
            else Cmult(**cmult_opts))

    @property
    def delay(self):
        # This is simply the delay of a Cmult3x() + 2 or a Cmult() (here the +2
        # comes from the fact that we register the input and outputs of the
        # Cmult3x).
        return 2 + self.cmult.delay if self.cmult3x else self.cmult.delay

    @property
    def twiddle_index_advance(self):
        # We use the BRAM output register, so the delay from address to read
        # output is 2 cycles.
        return 2 if self.storage == 'bram' else 0

    def auto_storage_rule(self):
        # If we need to store at most 2^6 twiddles, then we use LUTs, as doing
        # so we will need 2 * tw LUT6's. Even for 2^7 twiddles we use LUTs, as
        # we can do with 4 * tw LUT6's and the F7 muxes. We could do for 2^8
        # twiddles with 8 * tw LUT6's and the F7 and F8 muxes, but in this case
        # and for larger number of twiddles we decide to use BRAM.
        #
        # For the radix 2 case, as long as tw <= 19, a single BRAM18 as 512x36
        # fits all the twiddles for order <= 9, and then we need to double the
        # amount of BRAM18's each time we increase order by one.
        ntwid = len(self.twiddles_elaborate()[0])
        return 'bram' if ntwid >= 2**8 else 'lut'

    @property
    def model_vlen(self):
        return 2**(self.radix_log2 * self.order)

    def model(self, re_in, im_in):
        v = self.model_vlen
        re_in, im_in = (np.array(x, 'int').reshape(-1, v)
                        for x in [re_in, im_in])
        tw_re, tw_im = (np.array(x, 'int')
                        for x in self.twiddles_full())
        trunc = self.twiddle_scale_clog2()
        re_out = clamp_nbits(
            (re_in * tw_re - im_in * tw_im).ravel() >> trunc,
            self.outw)
        im_out = clamp_nbits(
            (im_in * tw_re + re_in * tw_im).ravel() >> trunc,
            self.outw)
        return re_out, im_out

    def twiddle_scale_clog2(self):
        return self.tw - 2

    def twiddles_full(self):
        j_iter = (
            range(2**self.radix_log2)
            if not self.r22_mode
            else [0, 2, 1, 3])
        twiddle_complex = np.array([
            np.exp(-1j*np.pi*j*k/2**(self.radix_log2*self.order-1))
            for j in j_iter
            for k in range(2**(self.radix_log2*(self.order-1)))])
        twiddle_scale = 1 << self.twiddle_scale_clog2()
        twiddle_int_re = [int(a) for a in
                          np.round(twiddle_scale * twiddle_complex.real)]
        twiddle_int_im = [int(a) for a in
                          np.round(twiddle_scale * twiddle_complex.imag)]
        return twiddle_int_re, twiddle_int_im

    def twiddles_elaborate(self):
        twiddles_re, twiddles_im = self.twiddles_full()
        if self.radix_log2 == 1:
            # Optimization for radix 2:
            # The twiddle factors to generate are
            # 1, 1, ..., 1 2**(order-1) times, and
            # 1, w, w**2, ..., w**(2**(order-1)).
            # Therefore, we only store the second half of the list
            # and play some logic with the addressing to return the
            # first twiddle when the MSB of the address is zero.
            n = len(twiddles_re)
            return twiddles_re[n // 2:], twiddles_im[n // 2:]
        return twiddles_re, twiddles_im

    def elaborate(self, platform):
        m = Module()

        twiddles_re, twiddles_im = self.twiddles_elaborate()
        if self.radix_log2 == 1:
            # See optimization for radix 2 in self.twiddles_elaborate().
            address = Signal(self.order - 1)
            m.d.comb += address.eq(Mux(
                self.twiddle_index[-1],
                self.twiddle_index[:-1],
                0))
        else:
            address = self.twiddle_index

        # Pack re and im together in the same Memory
        mask = 2**self.tw - 1
        twiddles_packed = [((re & mask) << self.tw) | (im & mask)
                           for re, im in zip(twiddles_re, twiddles_im)]
        mem_attrs = {
            'ram_style': (
                'distributed' if self.storage == 'lut'
                else 'block'),
        }
        mem_domain = 'comb' if self.storage == 'lut' else 'sync'
        m.submodules.twiddle_mem = twiddle_mem = (
            Memory(
                shape=2*self.tw,
                depth=len(twiddles_packed),
                init=twiddles_packed,
                attrs=mem_attrs,
            ))
        rdport = twiddle_mem.read_port(domain=mem_domain)
        # Use BRAM output register
        if self.storage == 'bram':
            twiddle_mem_out = Signal(2*self.tw, reset_less=True)
            with m.If(self.clken):
                m.d.sync += twiddle_mem_out.eq(rdport.data)
            m.d.comb += rdport.en.eq(self.clken)
        else:
            twiddle_mem_out = rdport.data
        m.submodules.cmult = cmult = self.cmult
        if self.cmult3x:
            m.d.comb += cmult.common_edge.eq(self.common_edge)
            # In this case we register the inputs and outputs of the cmult3x
            # with the 1x clock to improve the timing paths.
            extra_delay = 2
        else:
            extra_delay = 0
        # Check that our delay definition is correct
        assert self.delay == cmult.delay + extra_delay
        m.d.comb += [
            rdport.addr.eq(address),
            cmult.clken.eq(self.clken),
        ]
        cmult_ios = [
            cmult.re_a.eq(self.re_in),
            cmult.im_a.eq(self.im_in),
            cmult.re_b.eq(twiddle_mem_out[self.tw:]),
            cmult.im_b.eq(twiddle_mem_out[:self.tw]),
            self.re_out.eq(cmult.re_out),
            self.im_out.eq(cmult.im_out),
        ]
        if self.cmult3x:
            with m.If(self.clken):
                m.d.sync += cmult_ios
        else:
            m.d.comb += cmult_ios
        return m


class Window(Elaboratable):
    """Window multiplication

    This module contains the memory storing the window coefficients and the
    multiplier that multiplies the input with these coefficients.

    Parameters
    ----------
    domain_2x : str
        Clock domain for the 2x clock.
    order_log2 : int
        log2 of the FFT size.
    sample_width : int
        Width of the input samples.
    coeff_width : int
        Width of the window coefficients. The coefficients are assumed to be
        non-negative.
    window : str
        Name of the window (among those supported by scipy.signal.window).

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    common_edge : Signal(), in
        A signal that toggles with the 2x clock and is high immediately
        after the rising edge of the 1x clock.
    coeff_index : Signal(order_log2), in
        Index of the coefficient to read from the coefficient BRAM.
    re_in : Signal(signed(sample_width)), in
        Real part of the input sample.
    im_in : Signal(signed(sample_width)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(sample_width)), out
        Real part of the output sample.
    im_out : Signal(signed(sample_width)), out
        Imaginary part of the output sample.
    """
    def __init__(self, domain_2x, order_log2, sample_width, coeff_width,
                 window='blackmanharris'):
        self._domain_2x = domain_2x
        self.order_log2 = order_log2
        self.sw = sample_width
        self.cw = coeff_width
        self.outw = sample_width
        self.truncate = self.sw + self.cw - self.outw
        self.window_name = window

        self.clken = Signal()
        self.common_edge = Signal()
        self.coeff_index = Signal(order_log2)
        self.re_in = Signal(signed(self.sw))
        self.im_in = Signal(signed(self.sw))
        self.re_out = Signal(signed(self.outw))
        self.im_out = Signal(signed(self.outw))

    @property
    def delay(self):
        # This is the delay of the mult2x, but we haven't built a mult2x yet
        # to ask it.
        return 3

    @property
    def coeff_index_advance(self):
        # We use the BRAM output register, so the delay from address to read
        # output is 2 cycles.
        return 2

    @property
    def model_vlen(self):
        return 2**self.order_log2

    def model(self, re_in, im_in):
        v = self.model_vlen
        re_in, im_in = (np.array(x, 'int').reshape(-1, v)
                        for x in [re_in, im_in])
        w = self.window()
        re_out = (re_in * w).ravel() >> self.truncate
        im_out = (im_in * w).ravel() >> self.truncate
        return re_out, im_out

    def window(self):
        # We use fftbins=False to get a symmetric window. Even though we want
        # the window for an FFT, we prefer a symmetric window because this
        # allows us to store only the left half of the window.
        w = scipy.signal.get_window(self.window_name, 2**self.order_log2,
                                    fftbins=False)
        if np.any(w < 0):
            raise ValueError(
                'windows with negative coefficients not supported')
        scale = 2**self.cw - 1
        return [int(a) for a in np.round(scale * w)]

    def elaborate(self, platform):
        m = Module()

        # We only store the left half of the window to save BRAM space.
        m.submodules.window_mem = window_mem = (
            Memory(
                shape=self.cw,
                depth=2**(self.order_log2-1),
                init=self.window()[:2**(self.order_log2-1)],
                attrs={'ram_style': 'block'},
            ))
        rdport = window_mem.read_port(domain='sync')
        # BRAM output register
        window_mem_out = Signal(self.cw, reset_less=True)
        with m.If(self.clken):
            m.d.sync += window_mem_out.eq(rdport.data)
        m.d.comb += rdport.en.eq(self.clken)
        # This implements symmetrical reading in the BRAM
        addr = Mux(self.coeff_index[-1],
                   ~self.coeff_index[:-1], self.coeff_index[:-1])

        # Use real width = self.cw + 1 for unsigned -> signed conversion
        # without losses.
        m.submodules.mult2x = mult2x = Mult2x(
            self._domain_2x, self.sw, self.cw + 1, self.truncate)
        # Check that our delay definition is correct
        assert self.delay == mult2x.delay
        m.d.comb += [
            rdport.addr.eq(addr),
            mult2x.clken.eq(self.clken),
            mult2x.common_edge.eq(self.common_edge),
            mult2x.re_in.eq(self.re_in),
            mult2x.im_in.eq(self.im_in),
            mult2x.real_in.eq(window_mem_out),
            self.re_out.eq(mult2x.re_out),
            self.im_out.eq(mult2x.im_out),
        ]

        return m


class FFTControl(Elaboratable):
    """FFT controller

    This module supplies control inputs to the twiddle factor and butterfly
    modules.

    Parameters
    ----------
    butterflies : list
        List of butterfly modules, ordered from input to output.
    twiddles : list
        List of twiddle factor modules, ordered from input to output.
    window : Optional[Elaboratable]
        Window module (if present).

    Attributes
    ----------
    delay : int
        Delay (in samples) from input to output of the FFT.
    clken : Signal(), in
        Clock enable.
    mux_control : list[Optional[Signal()]], out
        List of ``mux_control`` output signals for each of the butterflies
        (and None in the positions corresponding to R22SDF butterflies).
    mux_count : list[Optional[Signal(2)]], out
        List of ``mux_count`` output signals for each of the R22SDF butterflies
        (and None in the positions corresponding to non-R22SDF butterflies).
    bram_raddr : list[Optional[Signal(...)]], out
        List of ``bram_raddr`` output signals for each of the butterflies using
        BRAM storage (and None in the positions corresponding to distributed
        storage butterflies).
    bram_waddr : list[Optional[Signal(...)]], out
        List of ``bram_raddr`` output signals for each of the butterflies using
        BRAM storage (and None in the positions corresponding to distributed
        storage butterflies).
    twiddle_index : list[Signal(...)], out
        List of ``twiddle_index`` output signals for each of the twiddles.
    out_last : Signal(), out
        This signal is asserted when the last sample of each transform is
        presented at the output.
    """
    # butterflies and twiddles are passed ordered from input to output
    def __init__(self, butterflies, twiddles, window):
        assert len(butterflies) == len(twiddles) + 1
        self.butterflies = butterflies
        self.twiddles = twiddles
        self.window = window
        self.stages = len(butterflies)

        self.clken = Signal()
        self._clken_out = Signal()  # used to connect clken of stages
        if self.window is not None:
            self.window_index = Signal(self.window.coeff_index.shape())
        self.mux_control = [Signal(name=f'mux_control{j}')
                            if not isinstance(self.butterflies[j], R22SDF)
                            else None
                            for j in range(self.stages)]
        self.mux_count = [Signal(2, name=f'mux_count{j}')
                          if isinstance(self.butterflies[j], R22SDF)
                          else None
                          for j in range(self.stages)]
        self.bram_raddr = [Signal(bfly.bram_raddr.shape(),
                                  name=f'bram_raddr{j}')
                           if bfly.storage == 'bram' else None
                           for j, bfly in enumerate(self.butterflies)]
        self.bram_waddr = [Signal(bfly.bram_waddr.shape(),
                                  name=f'bram_waddr{j}')
                           if bfly.storage == 'bram' else None
                           for j, bfly in enumerate(self.butterflies)]
        self.twiddle_index = [Signal(twiddles[j].twiddle_index.shape(),
                                     name=f'twiddle_index{j}')
                              for j in range(self.stages - 1)]
        self.out_last = Signal()

    @property
    def fft_delay(self):
        return self.delay_butterflies_input()[-1] + self.butterflies[-1].delay

    @property
    def delay_window(self):
        return self.window.delay if self.window is not None else 0

    def delay_butterflies_input(self):
        """Gives the delay from the FFT input to the input of each of the
        butterflies"""
        return [
            self.delay_window
            + sum([butterfly.delay for butterfly in self.butterflies[:j]])
            + sum([twiddle.delay for twiddle in self.twiddles[:j]])
            for j in range(self.stages)
        ]

    def delay_twiddles_input(self):
        """Gives the delay from the FFT input to the input of each of the
        twiddles"""
        delay_butterflies_input = self.delay_butterflies_input()
        return [
            delay_butterflies_input[j] + self.butterflies[j].delay
            for j in range(self.stages - 1)
        ]

    def order_stage(self, n):
        return sum([bfly.radix_log2 for bfly in self.butterflies[n:]])

    def elaborate(self, platform):
        m = Module()
        m.d.comb += self._clken_out.eq(self.clken)

        delay_butterflies_input = self.delay_butterflies_input()
        delay_twiddles_input = self.delay_twiddles_input()
        any_bfly_bram = any([bfly.use_bram_reg for bfly in self.butterflies])

        # Counter to control the window
        if self.window is not None:
            counter_window = Signal(self.window.order_log2,
                                    init=self.window.coeff_index_advance)
            counter_window_next = Signal(counter_window.shape())
            # If we use the window counter, then the mux of the first butterfly
            # is generated by delaying the MSB of the window counter.
            mux_bfly0_ndel = (delay_butterflies_input[0]
                              + self.window.coeff_index_advance)
            assert mux_bfly0_ndel > 0
            mux_bfly0_delay = [
                Signal(2 if isinstance(self.butterflies[0], R22SDF) else 1,
                       name=f'mux_bfly0_delay{j}', reset_less=True)
                for j in range(mux_bfly0_ndel)]
            if any_bfly_bram:
                # We use init=-1 to get correct results for the first FFT in
                # simulation. Any other reset value would be good if we don't
                # need the first FFT to be correct.
                counter_window_q = Signal(counter_window.shape(), init=-1)
                with m.If(self.clken):
                    m.d.sync += counter_window_q.eq(counter_window)
            with m.If(self.clken):
                m.d.sync += [
                    counter_window.eq(counter_window_next),
                    mux_bfly0_delay[0].eq(
                        self.butterfly_delay_in(counter_window, 0)),
                ]
                m.d.sync += [
                    mux_bfly0_delay[j].eq(mux_bfly0_delay[j - 1])
                    for j in range(1, len(mux_bfly0_delay))]
            m.d.comb += [
                counter_window_next.eq(counter_window + 1),
                self.window_index.eq(counter_window),
                self.control_output(0).eq(mux_bfly0_delay[-1]),
            ]

        # Counters to control the butterflies muxes.
        #
        # Only butterfly 0 has a counter, and only when there is no window. The
        # muxes for the remaining stages are generated by delaying one or two
        # bits generated from the counters of the preceding twiddle.
        if self.window is None:
            counter_bfly0 = Signal(self.order_stage(0))
            counter_bfly0_next = Signal(counter_bfly0.shape())
            if any_bfly_bram:
                # We use init=-1 to get correct results for the first FFT in
                # simulation. Any other reset value would be good if we don't
                # need the first FFT to be correct.
                counter_bfly0_q = Signal(counter_bfly0.shape(), init=-1)
                with m.If(self.clken):
                    m.d.sync += counter_bfly0_q.eq(counter_bfly0)
            with m.If(self.clken):
                m.d.sync += counter_bfly0.eq(counter_bfly0_next)
            m.d.comb += [
                counter_bfly0_next.eq(counter_bfly0 + 1),
                self.control_output(0).eq(
                    self.butterfly_delay_in(counter_bfly0, 0)),
            ]

        mux_bfly_delay = [
            [Signal(2 if isinstance(self.butterflies[j], R22SDF) else 1,
                    name=f'mux_bfly{j}_delay{k}', reset_less=True)
             for k in range(0,
                            delay_butterflies_input[j]
                            - delay_twiddles_input[j-1]
                            + self.twiddles[j-1].twiddle_index_advance)]
            for j in range(1, self.stages)]

        # Counters to control the twiddle indexes.
        counters_twiddles = [
            Signal(w := self.order_stage(j), name=f'counter_twiddle{j}',
                   init=(self.twiddles[j].twiddle_index_advance
                         - delay_twiddles_input[j]) % 2**w)
            for j in range(self.stages - 1)]

        # Counter to generate the out_last signal
        out_last_counter = Signal(
            w := self.order_stage(0), init=(-self.fft_delay + 1) % 2**w)
        out_last_counter_next = Signal(self.order_stage(0) + 1)
        out_last_counter_carry = out_last_counter_next[-1]

        with m.If(self.clken):
            m.d.sync += [
                counter.eq(counter + 1) for counter in counters_twiddles]
            for j in range(self.stages - 1):
                m.d.sync += mux_bfly_delay[j][0].eq(
                    self.butterfly_delay_in(counters_twiddles[j], j + 1))
                m.d.sync += [
                    mux_bfly_delay[j][k].eq(mux_bfly_delay[j][k - 1])
                    for k in range(1, len(mux_bfly_delay[j]))]
            m.d.sync += [
                out_last_counter.eq(out_last_counter_next),
                self.out_last.eq(out_last_counter_carry)]
        m.d.comb += [
            self.control_output(j).eq(
                mux_bfly_delay[j - 1][-1])
            for j in range(1, self.stages)]
        m.d.comb += [
            self.twiddle_index[j].eq(counters_twiddles[j])
            for j in range(self.stages-1)]
        m.d.comb += out_last_counter_next.eq(out_last_counter + 1)

        # counter_bfly0_next and counter_bfly0 (or counter_bfly0_q, depending
        # on whether use_bram_reg is enabled) are used to provide the read and
        # write addresses of the butterflies that use BRAMs. If the counter for
        # butterfly0 is replaced by the counter for the window, the counter for
        # the window is used here instead.
        if self.window is not None:
            counter0 = counter_window
            counter0_next = counter_window_next
            if any_bfly_bram:
                counter0_q = counter_window_q
        else:
            counter0 = counter_bfly0
            counter0_next = counter_bfly0_next
            if any_bfly_bram:
                counter0_q = counter_bfly0_q
        for j in range(self.stages):
            if (bfly := self.butterflies[j]).storage == 'bram':
                w = len(bfly.bram_raddr)
                m.d.comb += [
                    self.bram_raddr[j].eq(counter0_next[:w]),
                    self.bram_waddr[j].eq(
                        counter0[:w] if not bfly.use_bram_reg
                        else counter0_q[:w]),
                ]

        return m

    def butterfly_counter(self, counter, stage):
        r = self.butterflies[stage].radix_log2
        o = self.order_stage(stage)
        return counter[:o][-r:]

    def butterfly_mux(self, counter, stage):
        return self.butterfly_counter(counter, stage).all()

    def butterfly_delay_in(self, counter, stage):
        return (self.butterfly_counter(counter, stage)
                if isinstance(self.butterflies[stage], R22SDF)
                else self.butterfly_mux(counter, stage))

    def control_output(self, stage):
        if (m := self.mux_control[stage]) is not None:
            return m
        return self.mux_count[stage]

    def control_input(self, stage):
        if self.mux_control[stage] is not None:
            return self.butterflies[stage].mux_control
        return self.butterflies[stage].mux_count

    def connect_stages(self, module):
        """Connects the FFTControl to the stages and the datapaths of the
        stages"""
        m = module
        if self.window is not None:
            m.d.comb += [
                self.window.clken.eq(self._clken_out),
                self.window.coeff_index.eq(self.window_index),
                self.butterflies[0].re_in.eq(self.window.re_out),
                self.butterflies[0].im_in.eq(self.window.im_out),
            ]
        m.d.comb += (
            [stage.clken.eq(self._clken_out)
             for stage in self.butterflies + self.twiddles]
            + [self.control_input(j).eq(self.control_output(j))
               for j in range(self.stages)]
            + [bfly.bram_raddr.eq(self.bram_raddr[j])
               for j, bfly in enumerate(self.butterflies)
               if bfly.storage == 'bram']
            + [bfly.bram_waddr.eq(self.bram_waddr[j])
               for j, bfly in enumerate(self.butterflies)
               if bfly.storage == 'bram']
            + [self.twiddles[j].twiddle_index.eq(self.twiddle_index[j])
               for j in range(self.stages - 1)]
            + [self.butterflies[j].re_in.eq(self.twiddles[j-1].re_out)
               for j in range(1, self.stages)]
            + [self.butterflies[j].im_in.eq(self.twiddles[j-1].im_out)
               for j in range(1, self.stages)]
            + [self.twiddles[j].re_in.eq(self.butterflies[j].re_out)
               for j in range(self.stages - 1)]
            + [self.twiddles[j].im_in.eq(self.butterflies[j].im_out)
               for j in range(self.stages - 1)]
        )


class FFT(Elaboratable):
    """FFT

    Allowed input values:

    In order to prevent internal overflows after the twiddle factor
    multiplications, the input must have complex amplitude smaller or equal
    than 2**(width_in-1)-1 (the complex amplitude is defined as
    sqrt(re**2 + im**2)).

    To allow the full set of possible signed width_in bit complex values,
    the first twiddle factor multiplication should include an additional
    bit growth of one bit, but a mode to enable this behaviour is not
    implemented currently.

    Parameters
    ----------
    width_in : int
        Input width of the FFT.
    order_log2 : int
        log2 of the FFT order (size).
    radix : Union[int, str]
        Radix of the FFT. The possible options are 2, 4, and ``'R22'``, which
        uses radix 2^2 to implement a radix-4 FFT.
    width_twiddle : Optional[int]
        Width of the twiddle factors. By default, the same width as the input
        width is used.
    truncates : Optional[List[Union[int, List[int]]]]
        Truncate schedule to use. By default all the stages truncate their
        bit growth, so that the datapath width does not grow. This parameter
        allows to specify a custom trucate schedule. The schedule is specified
        as a list with length equal to the number of stages. Each element in
        the list gives the truncate parameter for the corresponding butterfly.
        For radix 2 and radix 4 the trucate parameter is an integer, while for
        radix 2^2 it is a list of two integers.
    butterfly_storage : str
        Storage method to use for the butterflies (see :class: ``R2SDF``).
    twiddle_storage : str
        Storage method to use for the twiddle factors (see
        :class: ``Twiddle``).
    use_bram_reg : bool
        Controls whether to use the output register for the butterfly BRAMs
        (when the butterfly delay lines are implemented using BRAMs).
    window : Optional[str]
        Window to use. This must be either ``None``, for no window, or a str
        containing the name of one of the windows supported by
        ``scipy.signal.window``.
    cmult3x : bool
        If this is enabled, a single-multiplier running at 3x the clock
        frequency is used for the complex multiplier, instead of 3 multipliers.
    domain_2x : Optional[str]
        Name of the clock domain of the 2x clock. This is only used when
        a window is used.
    domain_3x : Optional[str]
        Name of the clock domain of the 3x clock. This is only used when
        cmult3x is enabled.

    Attributes
    ----------
    clken : Signal(), in
        Clock enable.
    common_edge_2x : Signal(), in
        A signal that toggles with the 2x clock and is high immediately
        after the rising edge of the 1x clock. This is only present when
        a window is used.
    common_edge_3x : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock. This is only
        present when cmult3x is enabled.
    re_in : Signal(signed(width_in)), in
        Real part of the input sample.
    im_in : Signal(signed(width_in)), in
        Imaginary part of the input sample.
    re_out : Signal(signed(width_out)), out
        Real part of the output sample.
    im_out : Signal(signed(width_out)), out
        Imaginary part of the output sample.
    out_last : Signal(), out
        This signal is asserted whenever the last sample of the FFT vector
        is presented on the output.
    """
    def __init__(self, width_in, order_log2, radix,
                 width_twiddle=None, truncates=None,
                 butterfly_storage='auto', twiddle_storage='auto',
                 use_bram_reg=True, window=None, cmult3x=False,
                 domain_2x=None, domain_3x=None):
        if radix not in [2, 4, 'R22']:
            raise ValueError(
                f"invalid radix {radix} (radix can only be 2, 4 or 'R22')")
        self.order_log2 = order_log2

        if width_twiddle is None:
            width_twiddle = width_in

        butterfly = {2: R2SDF, 4: R4SDF, 'R22': R22SDF}[radix]
        radix_log2 = {2: 1, 4: 2, 'R22': 2}[radix]
        bfly_trunc = {2: 1, 4: 2, 'R22': [1, 1]}[radix]
        r22_mode = radix == 'R22'
        self.nstages = nstages = self.order_log2 // radix_log2

        if truncates is None:
            truncates = [bfly_trunc] * nstages
        widths = [width_in]
        w = width_in
        for j in range(nstages):
            w += radix_log2 - int(np.sum(truncates[j]))
            widths.append(w)

        if domain_2x is not None:
            self.common_edge_2x = Signal()
        elif window is not None:
            raise ValueError('domain_2x must be used when a window is used')
        if domain_3x is not None:
            self.common_edge_3x = Signal()
        elif cmult3x:
            raise ValueError('domain_3x must be used with cmult3x')
        self.cmult3x = cmult3x

        self.clken = Signal()
        self.re_in = Signal(signed(width_in))
        self.im_in = Signal(signed(width_in))
        width_out = widths[-1]
        self.re_out = Signal(signed(width_out))
        self.im_out = Signal(signed(width_out))
        self.out_last = Signal()

        if window is not None:
            # Use 9 bits as coefficient window for efficient packing on BRAM
            self._window = Window(domain_2x, order_log2, width_in, 9, window)
        else:
            self._window = None
        # Always use distributed storage for the last stage, since its buffers
        # have length 1 and cannot be implemented with a BRAM.
        self._butterflies = [
            butterfly(
                nstages - j, widths[j], truncate=truncates[j],
                storage=(butterfly_storage if j < nstages - 1
                         else 'distributed'),
                use_bram_reg=use_bram_reg)
            for j in range(nstages)]
        self._twiddles = [
            Twiddle(nstages - j, radix_log2,
                    sample_width=widths[j + 1],
                    twiddle_width=width_twiddle,
                    storage=twiddle_storage, r22_mode=r22_mode,
                    cmult3x=cmult3x, domain_3x=domain_3x)
            if radix_log2 != 1 or j != nstages - 2
            else TwiddleI(widths[j + 1])  # use TwiddleI for last radix 2 stage
            for j in range(nstages - 1)]
        self._control = FFTControl(
            self._butterflies, self._twiddles, self._window)

    @property
    def delay(self):
        return self._control.fft_delay

    @property
    def model_vlen(self):
        return 2**self.order_log2

    def model(self, re_in, im_in):
        v = self.model_vlen
        re = re_in
        im = im_in
        if self._window is not None:
            re, im = self._window.model(re, im)
        for j in range(self.nstages):
            re, im = self._butterflies[j].model(re, im)
            if j != self.nstages - 1:
                re, im = self._twiddles[j].model(re, im)
        return re, im

    def elaborate(self, platform):
        m = Module()
        if self._window is not None:
            m.submodules.window = self._window
            m.d.comb += self._window.common_edge.eq(self.common_edge_2x)
            first = self._window
        else:
            first = self._butterflies[0]
        for j, bfly in enumerate(self._butterflies):
            m.submodules[f'bfly{j}'] = bfly
        for j, twiddle in enumerate(self._twiddles):
            m.submodules[f'twiddle{j}'] = twiddle
        if self.cmult3x:
            m.d.comb += [twiddle.common_edge.eq(self.common_edge_3x)
                         for twiddle in self._twiddles
                         if not isinstance(twiddle, TwiddleI)]
        m.submodules.control = ctrl = self._control
        ctrl.connect_stages(m)
        last_bfly = self._butterflies[-1]
        m.d.comb += [
            ctrl.clken.eq(self.clken),
            first.re_in.eq(self.re_in),
            first.im_in.eq(self.im_in),
            self.re_out.eq(last_bfly.re_out),
            self.im_out.eq(last_bfly.im_out),
            self.out_last.eq(ctrl.out_last),
        ]
        return m


def gen_verilog():
    order_log2 = 12
    for radix in [2, 4, 'R22']:
        for window in [None, 'blackmanharris']:
            for cmult3x in [False, True]:
                w = window if window is not None else 'nowindow'
                truncates = {
                    2: [0] * (order_log2 // 2) + [1] * (order_log2 // 2),
                    4: [0] * (order_log2 // 4) + [2] * (order_log2 // 4),
                    'R22': (
                        [[0, 0]] * (order_log2 // 4)
                        + [[1, 1]] * (order_log2 // 4)),
                }[radix]
                x3 = '_cmult3x' if cmult3x else ''
                file_out = f'fft_radix{radix}_{w}{x3}.v'
                m = FFT(12, order_log2, radix,
                        width_twiddle=16,
                        truncates=truncates,
                        use_bram_reg=True,
                        window=window,
                        cmult3x=cmult3x,
                        domain_2x='clk2x' if window is not None else None,
                        domain_3x='clk3x' if cmult3x else None)
                ports = [m.clken,
                         m.re_in, m.im_in,
                         m.re_out, m.im_out,
                         m.out_last]
                if window is not None:
                    ports.append(m.common_edge_2x)
                if cmult3x:
                    ports.append(m.common_edge_3x)
                with open(file_out, 'w') as f:
                    platform = PlutoPlatform()
                    f.write(
                        amaranth.back.verilog.convert(
                            m,
                            name=f'fft_radix{radix}_{w}{x3}',
                            ports=ports, platform=platform,
                            emit_src=False))
                print('wrote verilog to', file_out)


if __name__ == '__main__':
    gen_verilog()
