#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
from amaranth.lib.cdc import FFSynchronizer, PulseSynchronizer
import amaranth.cli

from .dma import DmaStreamWrite
from .fifo import AsyncFifo18_36
from .packer import Pack12IQto32, Pack8IQto32, PackFifoTwice


class Recorder12IQ(Elaboratable):
    """IQ recorder (12-bit input).

    This recorder can pack 12-bit IQ into 3 bytes or drop the 4 LSBs
    and record as 8-bit IQ. It uses a DmaStreamWrite to write to
    RAM.

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
    re_in : Signal(12), in
        Input real part.
    im_in : Signal(12), in
        Input imaginary part.
    mode_8bit : Signal(), in
        Controls the use of 8-bit recording mode.
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
        self.re_in = Signal(12)
        self.im_in = Signal(12)

        # domain_dma
        self.mode_8bit = Signal()
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
            self.mode_8bit, self.start, self.stop, self.finished,
            self.dropped_samples, self.next_address,
        ] + self.dma.axi.ports()

    def elaborate(self, platform):
        m = Module()
        in_renamer = DomainRenamer({'sync': self.domain_in})
        dma_renamer = self.dma_renamer

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

        # mode_8bit, run synchronizer: domain_dma -> domain_in
        if self.domain_in == self.domain_dma:
            mode_8bit = self.mode_8bit
            run_in = run_delay[-1]
        else:
            mode_8bit = Signal()
            run_in = Signal()
            m.submodules.sync_mode_8bit = FFSynchronizer(
                self.mode_8bit, mode_8bit, o_domain=self.domain_in)
            m.submodules.sync_run = FFSynchronizer(
                run_delay[-1], run_in, o_domain=self.domain_in)

        # domain_in
        m.d.comb += [
            pack12.re_in.eq(self.re_in),
            pack12.im_in.eq(self.im_in),
            pack12.strobe_in.eq(self.strobe_in),
            pack12.enable.eq(run_in & ~mode_8bit),
            pack8.re_in.eq(self.re_in >> 4),
            pack8.im_in.eq(self.im_in >> 4),
            pack8.strobe_in.eq(self.strobe_in),
            pack8.enable.eq(run_in & mode_8bit),
            fifo.data_in.eq(Mux(mode_8bit, pack8.out, pack12.out)),
            fifo.wren.eq(Mux(mode_8bit, pack8.strobe_out, pack12.strobe_out)),
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
    recorder = Recorder12IQ(
        0x01100000, 0x1a000000, domain_in='iq', domain_dma='sync')
    amaranth.cli.main(
        recorder, ports=recorder.ports())
