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

from maia_hdl.cmult import Cmult3x
from maia_hdl.clknx import ClkNxCommonEdge
from maia_hdl.pluto_platform import PlutoPlatform


class Tb(Elaboratable):
    def __init__(self):
        self.clk3x = 'clk3x'
        self.dut = Cmult3x(self.clk3x, 16, 16)
        self.dut_wide = Cmult3x(self.clk3x, 22, 16)

    def elaborate(self, platform):
        m = Module()
        m.submodules.dut = self.dut
        m.submodules.dut_wide = self.dut_wide
        m.submodules.common_edge = common_edge = ClkNxCommonEdge(
            'sync', self.clk3x, 3)
        m.d.comb += self.dut.common_edge.eq(common_edge.common_edge)
        m.d.comb += self.dut_wide.common_edge.eq(common_edge.common_edge)
        return m


def main():
    tb = Tb()
    platform = PlutoPlatform()
    port_names = ['clken', 're_a', 'im_a', 're_b', 'im_b',
                  're_out', 'im_out']
    for n in port_names:
        getattr(tb.dut_wide, n).name = f'wide_{n}'
    ports = [getattr(dut, n)
             for dut in [tb.dut, tb.dut_wide]
             for n in port_names]
    with open('dut.v', 'w') as f:
        f.write('`timescale 1ps/1ps\n')
        f.write(convert(
            tb, name='dut', ports=ports, platform=platform,
            emit_src=False))


if __name__ == '__main__':
    main()
