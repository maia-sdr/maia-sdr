#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.cli


class AsyncFifo18_36(Elaboratable):
    """Asynchronous FIFO using Xilinx FIFO18_36 primitive.

    Parameters
    ----------
    r_domain : str
        Read clock domain.
    w_domain : str
        Write clock domain.

    Attributes
    ----------
    reset : Signal(), in
        Asynchronous reset for the FIFO.
    data_in : Signal(36), in
        Data input.
    wren : Signal(), in
        Write enable.
    full : Signal(), out
        FIFO full flag.
    wrerr : Signal(), out
        FIFO write error.
    data_out : Signal(36), out
        Data output.
    rden : Signal(), in
        Read enable.
    empty : Signal(), out
        FIFO empty flag.
    rderr : Signal(), out
        FIFO read error.
    """
    def __init__(self, r_domain='read', w_domain='write'):
        self._r_domain = r_domain
        self._w_domain = w_domain
        self.reset = Signal()

        self.data_in = Signal(36)
        self.wren = Signal()
        self.full = Signal()
        self.wrerr = Signal()

        self.data_out = Signal(36)
        self.rden = Signal()
        self.empty = Signal()
        self.rderr = Signal()

    def elaborate(self, platform):
        m = Module()
        m.submodules.fifo18e1 = fifo18e1 = Instance(
            'FIFO18E1',
            p_DATA_WIDTH=36,
            p_FIFO_MODE="FIFO18_36",
            i_DI=self.data_in[:32],
            i_DIP=self.data_in[32:],
            o_DO=self.data_out[:32],
            o_DOP=self.data_out[32:],
            o_EMPTY=self.empty,
            o_FULL=self.full,
            i_RDCLK=ClockSignal(self._r_domain),
            i_RDEN=self.rden,
            o_RDERR=self.rderr,
            i_REGCE=0,  # REGCE is only used with sync FIFO
            i_RST=self.reset,
            i_RSTREG=ResetSignal(self._r_domain),
            i_WRCLK=ClockSignal(self._w_domain),
            i_WREN=self.wren,
            o_WRERR=self.wrerr,
        )
        return m


if __name__ == '__main__':
    fifo = AsyncFifo18_36()
    amaranth.cli.main(
        fifo, ports=[
            fifo.reset, fifo.data_in, fifo.wren, fifo.full, fifo.wrerr,
            fifo.data_out, fifo.rden, fifo.empty, fifo.rderr])
