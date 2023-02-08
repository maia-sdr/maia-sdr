#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.back.verilog

from math import log2

from . import axi


class DmaBRAMWrite(Elaboratable):
    def __init__(self, base_address, num_buffers_log2,
                 bram_awidth, bram_latency=2,
                 axi_width=64, axi_awidth=32,
                 name=None):
        """Cyclic DMA BRAM -> AXI3

        This module contains an AXI3 Manager that reads data from a BRAM and
        writes it to an AXI3 port. This is done in a cyclic DMA way. Each time
        that the BRAM is read, it is written to a different buffer in a
        ring-buffer.

        The write address to use is hardcoded at synthesis time.

        Parameters
        ----------
        base_address : int
            Base address to use for the ring-buffer to write to.
        num_buffers_log2 : int
            The number of buffers in the ring-buffer, expressed as
            ``2**num_buffers_log2``.
        bram_awidth : int
            Address width of the BRAM to read from.
        bram_latency : int
            Read latency of the BRAM, in clock cycles.
        axi_width : int
            Data width of the AXI3 port.
        axi_awdith : int
            Address width of the AXI3 port.
        name : Optional[str]
            Name for the AXI3 Manager interface.

        Attributes
        ----------
        axi : AXI3 Manager interface
            The AXI3 port used for writing.
        start : Signal(), in
            This signal should be pulsed for a clock cycle to start a
            DMA transfer from the BRAM to the AXI3 port. It is undefined
            behaviour to pulse this signal while the module is busy.
        busy : Signal(), out
            This signal is asserted while the DMA transfer is in progress.
        last_buffer : Signal(num_buffers_log2), out
            Contains the buffer index of the buffer that was transferred
            previously.
        raddr : Signal(), out
            BRAM read address.
        rdata : Signal(), in
            BRAM read data.
        ren : Signal(), out
            BRAM read enable.
        """
        self.bytes_per_word_log2 = int(log2(axi_width // 8))
        self.address_shift = (
            num_buffers_log2 + bram_awidth + self.bytes_per_word_log2)
        if base_address != (
                base_address >> self.address_shift) << self.address_shift:
            raise ValueError('address is not aligned correctly')
        self.base_address = base_address
        self.num_buffers_log2 = num_buffers_log2
        self.axi_awidth = axi_awidth
        self.axi = axi.AxiInterface(
            axi.AxiDevice.MANAGER,
            [axi.AxiChannel(axi.AxiDirection.WRITE, axi_awidth, axi_width)],
            axi.AxiVersion.AXI3, name=name)
        self.start = Signal()
        self.busy = Signal()
        self.last_buffer = Signal(num_buffers_log2, reset=-1)
        # BRAM ports
        self.bram_latency = bram_latency
        self.raddr = Signal(bram_awidth)
        self.rdata = Signal(axi_width)
        self.ren = Signal()

    def ports(self):
        return self.axi.ports() + [
            self.start, self.busy, self.last_buffer,
            self.raddr, self.rdata, self.ren]

    def elaborate(self, platform):
        m = Module()

        # 16-word burst
        burst_len_log2 = 4

        # Addresses are generated independently of writes, since we know all
        # the addresses we will use beforehand.
        assert len(self.raddr) > burst_len_log2
        axi_addr_counter = Signal(
            self.num_buffers_log2 + len(self.raddr) - burst_len_log2)
        m.d.comb += self.axi.awaddr.eq(
            Cat(axi_addr_counter
                << (burst_len_log2 + self.bytes_per_word_log2),
                Const(self.base_address >> self.address_shift,
                      self.axi_awidth - self.address_shift)))
        with m.If(self.axi.aw_handshake()):
            m.d.sync += axi_addr_counter.eq(axi_addr_counter + 1)

        # Beat counter to determine the end of bursts
        beat_counter = Signal(burst_len_log2)
        beat_counter_next = Signal(len(beat_counter) + 1)
        last_beat = beat_counter_next[-1]
        m.d.comb += beat_counter_next.eq(beat_counter + 1)

        raddr_next = Signal(len(self.raddr) + 1)
        last_bram_addr = raddr_next[-1]
        m.d.comb += raddr_next.eq(self.raddr + 1)

        with m.If(self.ren):
            m.d.sync += self.raddr.eq(raddr_next)
        with m.If(self.start):
            m.d.sync += self.raddr.eq(0)

        m.d.comb += [
            self.axi.wdata.eq(self.rdata),
            self.axi.awlen.eq(2**burst_len_log2 - 1),
            self.axi.awburst.eq(axi.AxiBurst.INCR),
            # Normal non-cacheable buffereable memory
            self.axi.awcache.eq(0b0011),
            self.axi.awprot.eq(0b0000),
            self.axi.awlock.eq(0),
            self.axi.awsize.eq(self.bytes_per_word_log2),
            self.axi.wstrb.eq(-1),
            self.axi.wlast.eq(last_beat),
        ]

        m.d.sync += [
            self.axi.bready.eq(1),
            self.axi.awvalid.eq(1),
        ]

        start_del = Signal(self.bram_latency)
        last_bram_addr_del = Signal(self.bram_latency)
        m.d.sync += start_del.eq(Cat(self.start, start_del[:-1]))
        m.d.comb += self.ren.eq(start_del.any() | self.axi.w_handshake())

        with m.If(start_del[-1]):
            m.d.sync += self.axi.wvalid.eq(1)
        with m.If(last_bram_addr_del[-1] & self.axi.w_handshake()):
            m.d.sync += self.axi.wvalid.eq(0)

        bvalid_counter = Signal(len(self.raddr) - burst_len_log2)
        bvalid_counter_next = Signal(len(bvalid_counter) + 1)
        last_bvalid = bvalid_counter_next[-1]
        m.d.comb += bvalid_counter_next.eq(bvalid_counter + 1)
        # We use bvalid instead of b_handshake() here and below because bready
        # is always asserted except when in reset.
        with m.If(self.axi.bvalid):
            m.d.sync += bvalid_counter.eq(bvalid_counter_next)

        with m.If(self.start):
            m.d.sync += self.busy.eq(1)
        with m.If(last_bvalid & self.axi.bvalid):
            m.d.sync += [
                self.busy.eq(0),
                self.last_buffer.eq(self.last_buffer + 1),
            ]

        with m.If(self.axi.w_handshake()):
            m.d.sync += [
                beat_counter.eq(beat_counter_next),
                last_bram_addr_del.eq(
                    Cat(last_bram_addr, last_bram_addr_del[:-1]))
            ]

        return m


class DmaStreamWrite(Elaboratable):
    """DMA stream -> AXI3

    This module contains an AXI3 Manager that reads data from an
    AXI4-Stream-like interface and writes it to an AXI3 port.

    There are hardcoded start and end addresses. When started, the DMA will
    read from the stream and write data from the start to the end address,
    unless it is stopped earlier.

    Parameters
    ----------
    start_address : int
        Start address to use when writing the data.
    end_address : int
        End address to finish the transfer unless stopped manually first.
        The end address is not written to. The last byte written to is the
        previous to the end address.
    width : int
        Data width of the AXI3 and stream ports.
    axi_awidth : int
        Address width of the AXI3 port.
    name : Optional[str]
        Name for the AXI3 Manager interface.

    Attributes
    ----------
    axi : AXI3 Manager interface
       The AXI3 port used for writing.
    start : Signal(), in
       This signal should be pulsed for a clock cycle to start a DMA transfer.
       It is undefined behaviour to pulse this signal while the module is
       running.
    stop : Signal(), in
       This signal should be pulsed for a clock cycle to stop the module before
       it reaches the end address. It is undefined behaviour to pulse this
       signal while the module is stopped. The module will not stop
       immediately. It will finish its outstanding write bursts.
    finished : Signal(), out
       This signal is pulsed for one cycle after the module has finished its
       operation, including all the outstanding write bursts. This happens
       either some time after the module has been commanded to stop by pulsing
       the stop line or after the module has reached the end address.
    next_address : Signal(), out
       After the DMA is finished, this contains the next address that would
       have been written to.
    stream_data : Signal(width), in
       Stream data input.
    stream_valid : Signal(), in
       Stream valid. Semantics are as in AXI4-Stream.
    stream_ready : Signal(), out
       Stream ready. Semantics are as in AXI4-Stream.
    """
    def __init__(self, start_address, end_address, width=64, axi_awidth=32,
                 name=None):
        self.start_address = start_address
        self.end_address = end_address
        self.w = width
        self.axi_awidth = axi_awidth
        self.axi = axi.AxiInterface(
            axi.AxiDevice.MANAGER,
            [axi.AxiChannel(axi.AxiDirection.WRITE, axi_awidth, width)],
            axi.AxiVersion.AXI3, name=name)
        self.start = Signal()
        self.stop = Signal()
        self.finished = Signal()
        self.next_address = Signal(axi_awidth)
        # Stream ports
        self.stream_data = Signal(width)
        self.stream_valid = Signal()
        self.stream_ready = Signal()

    def ports(self):
        return self.axi.ports() + [
            self.start, self.stop, self.finished,
            self.stream_data, self.stream_valid, self.stream_ready]

    def elaborate(self, platform):
        m = Module()

        running = Signal()

        # 16-word burst
        burst_len_log2 = 4
        bytes_per_word_log2 = int(log2(self.w // 8))
        addr_shift = burst_len_log2 + bytes_per_word_log2
        if (((self.start_address >> addr_shift)
             << addr_shift != self.start_address)
            or ((self.end_address >> addr_shift)
                << addr_shift != self.end_address)):
            raise ValueError('address is not aligned correctly')
        one_outstanding_burst = Signal()
        two_outstanding_bursts = Signal()

        axi_addr_counter_reset = self.start_address >> addr_shift
        axi_addr_counter = Signal(
            range(self.start_address >> addr_shift,
                  (self.end_address >> addr_shift) + 1),
            reset=axi_addr_counter_reset)
        addr_counter_end = Signal()
        b = bin(self.end_address >> addr_shift)
        r = len(b) - len(b.rstrip('0'))  # number of zeros on the right
        m.d.comb += [
            self.next_address.eq(self.axi.awaddr),
            self.axi.awaddr.eq(axi_addr_counter << addr_shift),
            addr_counter_end.eq(
                axi_addr_counter[r:]
                == (self.end_address >> (addr_shift + r))),
            self.axi.awvalid.eq(
                running & ~two_outstanding_bursts & ~addr_counter_end),
        ]
        with m.If(self.axi.aw_handshake()):
            m.d.sync += axi_addr_counter.eq(axi_addr_counter + 1)
            with m.If(~(self.axi.w_handshake() & self.axi.wlast)):
                # increase number of outstanding bursts
                m.d.sync += [
                    one_outstanding_burst.eq(~one_outstanding_burst),
                    two_outstanding_bursts.eq(one_outstanding_burst),
                ]

        # Beat counter to determine the end of bursts
        beat_counter = Signal(burst_len_log2)
        beat_counter_next = Signal(len(beat_counter) + 1)
        last_beat = beat_counter_next[-1]
        m.d.comb += beat_counter_next.eq(beat_counter + 1)

        m.d.comb += [
            self.axi.wdata.eq(self.stream_data),
            self.axi.awlen.eq(2**burst_len_log2 - 1),
            self.axi.awburst.eq(axi.AxiBurst.INCR),
            # Normal non-cacheable buffereable memory
            self.axi.awcache.eq(0b0011),
            self.axi.awprot.eq(0b0000),
            self.axi.awlock.eq(0),
            self.axi.awsize.eq(bytes_per_word_log2),
            self.axi.wstrb.eq(-1),
            self.axi.wlast.eq(last_beat),
        ]
        m.d.sync += self.axi.bready.eq(1)

        # outstanding write response control
        max_outstanding_b_log2 = 2
        # outstanding_b contains the number of outstanding write responses - 1
        # in two's complement. This allows us to check 0 outstanding by looking
        # at a single bit and max outstanding by looking at two bits.
        outstanding_b = Signal(max_outstanding_b_log2 + 2, reset=-1)
        no_outstanding_b = outstanding_b[-1]
        full_outstanding_b = ~outstanding_b[-1] & outstanding_b[-2]
        with m.If(self.axi.wlast & self.axi.w_handshake() & ~self.axi.bvalid):
            m.d.sync += outstanding_b.eq(outstanding_b + 1)
        with m.If(self.axi.bvalid &
                  ~(self.axi.wlast & self.axi.w_handshake())):
            m.d.sync += outstanding_b.eq(outstanding_b - 1)

        enable_w = Signal()
        m.d.comb += [
            enable_w.eq((one_outstanding_burst | two_outstanding_bursts)
                        & ~full_outstanding_b),
            self.axi.wvalid.eq(enable_w & self.stream_valid),
            self.stream_ready.eq(enable_w & self.axi.wready),
        ]
        with m.If(self.axi.w_handshake()):
            m.d.sync += beat_counter.eq(beat_counter_next)
            with m.If(self.axi.wlast & ~self.axi.aw_handshake()):
                # decrease number of outstanding bursts
                m.d.sync += [
                    one_outstanding_burst.eq(two_outstanding_bursts),
                    two_outstanding_bursts.eq(0),
                ]

        with m.If(self.stop | addr_counter_end):
            m.d.sync += running.eq(0)
        with m.If(self.start):
            m.d.sync += [
                running.eq(1),
                axi_addr_counter.eq(axi_addr_counter_reset),
            ]
        one_outstanding_burst_q = Signal()
        no_outstanding_b_q = Signal(reset=1)
        m.d.sync += [
            no_outstanding_b_q.eq(no_outstanding_b),
            self.finished.eq(
                ~running
                & ~one_outstanding_burst & ~two_outstanding_bursts
                & ~no_outstanding_b_q & no_outstanding_b),
        ]

        return m


def gen_verilog():
    with open('dma.v', 'w') as f:
        m = DmaBRAMWrite(0x08000000, 6, 12)
        f.write(
            amaranth.back.verilog.convert(
                m,
                name='dma_bram_write',
                ports=m.ports(),
                emit_src=False))
    with open('dma_stream.v', 'w') as f:
        m = DmaStreamWrite(0x03000000, 0x1a000000)
        f.write(
            amaranth.back.verilog.convert(
                m,
                name='dma_stream_write',
                ports=m.ports(),
                emit_src=False))


if __name__ == '__main__':
    gen_verilog()
