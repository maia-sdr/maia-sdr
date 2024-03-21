#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
from amaranth.lib.cdc import FFSynchronizer, PulseSynchronizer
from amaranth.lib import enum
import amaranth.cli

from .dma import DmaStreamWrite
from .fifo import AsyncFifo18_36
from .packer import Pack16IQto32, Pack12IQto32, Pack8IQto32, PackFifoTwice


class RecorderMode(enum.Enum, shape=2):
    MODE_16BIT = 0
    MODE_12BIT = 1
    MODE_8BIT = 2


class Recorder16IQ(Elaboratable):
    """IQ recorder (16-bit input).

    This recorder has a 16-bit input and support recording the full
    16-bit, only the 12 MSBs, or only the 8 MSBs. When recording in
    12-bit mode, each sample is packed into 3 bytes. It uses a
    ``DmaStreamWrite`` to write to  RAM.

    The module has two clock domains, ``domain_in``, which corresponds
    to the input IQ samples, and ``domain_dma``, which corresponds to
    the DMA AXI interface and the control interface.

    Parameters
    ----------
    start_address : int
        Start address to use when writing the DMA data.
    end_address : int
        End address to finish the DMA transfer unless stopped manually first.
        The end address is not written to. The last byte written to is the
        previous to the end address.
    axi_awidth : int
        Address width of the AXI3 port of the DMA.
    dma_name : Optional[str]
        Name for the AXI3 Manager interface of the DMA.
    domain_in : str
        Clock domain for the IQ samples.
    domain_dma : str
        Clock domain for the DMA and control interface.

    Attributes
    ----------
    strobe_in : Signal(), in
        Asserted to indicate that a valid sample is presented at the input.
    re_in : Signal(16), in
        Input real part.
    im_in : Signal(16), in
        Input imaginary part.
    mode : Signal(RecorderMode), in
        Controls the recording mode.
    start : Signal(), in
       This signal should be pulsed for a clock cycle to start the recording.
       It is undefined behaviour to pulse this signal while the module is
       running.
    stop : Signal(), in
       This signal should be pulsed for a clock cycle to stop the recording
       before it reaches the end address. It is undefined behaviour to pulse
       this signal while the module is stopped. The module will not stop
       immediately. It will finish its outstanding DMA bursts.
    finished : Signal(), out
       This signal is pulsed for one cycle after the module has finished the
       recording. This happens either some time after the module has been
       commanded to stop by pulsing the stop line or after the module has
       reached the end address.
    next_address : Signal(), out
       After the DMA is finished, this contains the next address that would
       have been written to. This can be used to obtain the length of the
       recording when ``stop`` was used.
    """
    def __init__(self, start_address, end_address, dma_name=None,
                 axi_awidth=32,
                 domain_in='sync', domain_dma='sync'):
        self.domain_in = domain_in
        self.domain_dma = domain_dma

        # domain_in
        self.strobe_in = Signal()
        self.re_in = Signal(16)
        self.im_in = Signal(16)

        # domain_dma
        self.mode = Signal(RecorderMode)
        self.start = Signal()
        self.stop = Signal()
        self.finished = Signal()
        self.dropped_samples = Signal()
        self.next_address = Signal(axi_awidth)

        self.dma_renamer = DomainRenamer({'sync': self.domain_dma})
        self.dma = self.dma_renamer(
            DmaStreamWrite(start_address, end_address, name=dma_name,
                           axi_awidth=axi_awidth))

    def ports(self):
        return [
            self.strobe_in, self.re_in, self.im_in,
            self.mode.as_value(), self.start, self.stop, self.finished,
            self.dropped_samples, self.next_address,
        ] + self.dma.axi.ports()

    def elaborate(self, platform):
        m = Module()
        in_renamer = DomainRenamer({'sync': self.domain_in})
        dma_renamer = self.dma_renamer

        m.submodules.pack16 = pack16 = in_renamer(Pack16IQto32())
        m.submodules.pack12 = pack12 = in_renamer(Pack12IQto32())
        m.submodules.pack8 = pack8 = in_renamer(Pack8IQto32())
        m.submodules.fifo = fifo = AsyncFifo18_36(
            w_domain=self.domain_in, r_domain=self.domain_dma)
        m.submodules.pack64 = pack64 = dma_renamer(PackFifoTwice(width_in=32))
        m.submodules.dma = dma = self.dma

        # domain_dma
        run = Signal()
        # used to guarantee that the FIFO reset is deasserted at least
        # 2 read cycles before the first assertion of rden/wren.
        run_delay = Signal(2)
        with m.If(self.start):
            m.d[self.domain_dma] += run.eq(1)
        with m.If(self.finished):
            m.d[self.domain_dma] += run.eq(0)
        m.d[self.domain_dma] += run_delay.eq(Cat(run, run_delay))

        # mode & run synchronizer: domain_dma -> domain_in
        if self.domain_in == self.domain_dma:
            mode = self.mode
            run_in = run_delay[-1]
        else:
            mode = Signal(RecorderMode, reset_less=True)
            run_in = Signal()
            m.submodules.sync_mode = FFSynchronizer(
                self.mode.as_value(), mode, o_domain=self.domain_in)
            m.submodules.sync_run = FFSynchronizer(
                run_delay[-1], run_in, o_domain=self.domain_in)

        # domain_in
        m.d.comb += [
            pack16.re_in.eq(self.re_in),
            pack16.im_in.eq(self.im_in),
            pack16.strobe_in.eq(self.strobe_in),
            pack16.enable.eq(run_in & (mode == RecorderMode.MODE_16BIT)),

            pack12.re_in.eq(self.re_in >> 4),
            pack12.im_in.eq(self.im_in >> 4),
            pack12.strobe_in.eq(self.strobe_in),
            pack12.enable.eq(run_in & (mode == RecorderMode.MODE_12BIT)),

            pack8.re_in.eq(self.re_in >> 8),
            pack8.im_in.eq(self.im_in >> 8),
            pack8.strobe_in.eq(self.strobe_in),
            pack8.enable.eq(run_in & (mode == RecorderMode.MODE_8BIT)),
        ]
        with m.Switch(mode):
            with m.Case(RecorderMode.MODE_16BIT):
                m.d.comb += [
                    fifo.data_in.eq(pack16.out),
                    fifo.wren.eq(pack16.strobe_out),
                ]
            with m.Case(RecorderMode.MODE_12BIT):
                m.d.comb += [
                    fifo.data_in.eq(pack12.out),
                    fifo.wren.eq(pack12.strobe_out),
                ]
            with m.Case(RecorderMode.MODE_8BIT):
                m.d.comb += [
                    fifo.data_in.eq(pack8.out),
                    fifo.wren.eq(pack8.strobe_out),
                ]
        dropped = Signal()
        run_in_q = Signal()
        m.d[self.domain_in] += run_in_q.eq(run_in)
        with m.If(fifo.wrerr):
            m.d[self.domain_in] += dropped.eq(1)
        with m.If(run_in & ~run_in_q):
            m.d[self.domain_in] += dropped.eq(0)

        # dropped, run_in synchronizer: domain_in -> domain_dma
        if self.domain_in == self.domain_dma:
            m.d.comb += self.dropped_samples.eq(dropped)
            run_in_dma = run_in
        else:
            m.submodules.sync_dropped = FFSynchronizer(
                dropped, self.dropped_samples, o_domain=self.domain_dma)
            run_in_dma = Signal()
            m.submodules.sync_run_in = FFSynchronizer(
                run_in, run_in_dma, o_domain=self.domain_dma)

        # FIFO reset
        fifo_run = Signal()
        run_in_dma_q = Signal()
        m.d[self.domain_dma] += run_in_dma_q.eq(run_in_dma)
        m.d.comb += fifo.reset.eq(~fifo_run)
        with m.If(self.start):
            m.d[self.domain_dma] += fifo_run.eq(1)
        with m.If(~run_in_dma & run_in_dma_q):
            m.d[self.domain_dma] += fifo_run.eq(0)

        # domain_dma
        m.d.comb += [
            pack64.enable.eq(run_delay[-1]),
            pack64.fifo_data.eq(fifo.data_out),
            fifo.rden.eq(pack64.rden),
            pack64.empty.eq(fifo.empty),
            dma.stream_data.eq(pack64.out_data),
            dma.stream_valid.eq(pack64.out_valid),
            pack64.out_ready.eq(dma.stream_ready),
            dma.start.eq(self.start),
            dma.stop.eq(self.stop),
            self.finished.eq(dma.finished),
            self.next_address.eq(dma.next_address),
        ]

        return m


if __name__ == '__main__':
    recorder = Recorder16IQ(
        0x01100000, 0x1a000000, domain_in='iq', domain_dma='sync')
    amaranth.cli.main(
        recorder, ports=recorder.ports())
