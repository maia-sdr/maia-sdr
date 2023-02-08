#!/usr/bin/env python3
#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
from amaranth.back.verilog import convert

from maia_hdl.fifo import AsyncFifo18_36


def main():
    dut = AsyncFifo18_36()
    ports = [dut.reset, dut.data_in, dut.wren, dut.full, dut.wrerr,
             dut.data_out, dut.rden, dut.empty, dut.rderr]
    with open('dut.v', 'w') as f:
        f.write('`timescale 1ps/1ps\n')
        f.write(convert(
            dut, name='dut', ports=ports, emit_src=False))


if __name__ == '__main__':
    main()
