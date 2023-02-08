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

from maia_hdl.recorder import Recorder12IQ


def main():
    m = Recorder12IQ(0x00000000, 0x00001000,
                     domain_in='iq', domain_dma='sync')
    with open('dut.v', 'w') as f:
        f.write('`timescale 1ps/1ps\n')
        f.write(convert(
            m, name='dut', ports=m.ports(), emit_src=False))


if __name__ == '__main__':
    main()
