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

from maia_hdl.dma import DmaStreamWrite


def main():
    with open('dut.v', 'w') as f:
        m = DmaStreamWrite(0x0000f000, 0x00011000)
        f.write(convert(
            m, name='dut', ports=m.ports(), emit_src=False))


if __name__ == '__main__':
    main()
