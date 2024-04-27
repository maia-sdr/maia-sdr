#!/usr/bin/env python3
#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
from amaranth.back.verilog import convert

from maia_hdl.maia_sdr import MaiaSDR
from maia_hdl.pluto_platform import PlutoPlatform


def main():
    dut = MaiaSDR()
    with open('dut.v', 'w') as f:
        f.write(convert(
            dut, name='dut', ports=dut.ports(), emit_src=False,
            platform=PlutoPlatform()))


if __name__ == '__main__':
    main()
