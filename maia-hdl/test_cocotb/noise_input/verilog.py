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

from maia_hdl.maia_sdr import MaiaSDR


def main():
    dut = MaiaSDR()
    with open('dut.v', 'w') as f:
        f.write(convert(
            dut, name='dut', ports=dut.ports(), emit_src=False))


if __name__ == '__main__':
    main()
