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

from maia_hdl.cmult import Cmult3x
from maia_hdl.clknx import ClkNxCommonEdge
from maia_hdl.pluto_platform import PlutoPlatform


class Tb(Elaboratable):
    def __init__(self):
        self.clk3x = 'clk3x'
        self.dut = Cmult3x(self.clk3x, 16, 16)

    def elaborate(self, platform):
        m = Module()
        m.submodules.dut = self.dut
        m.submodules.common_edge = common_edge = ClkNxCommonEdge(
            'sync', self.clk3x, 3)
        m.d.comb += self.dut.common_edge.eq(common_edge.common_edge)
        return m


def main():
    tb = Tb()
    platform = PlutoPlatform()
    ports = [tb.dut.clken, tb.dut.re_a, tb.dut.im_a,
             tb.dut.re_b, tb.dut.im_b,
             tb.dut.re_out, tb.dut.im_out]
    with open('dut.v', 'w') as f:
        f.write('`timescale 1ps/1ps\n')
        f.write(convert(
            tb, name='dut', ports=ports, platform=platform,
            emit_src=False))


if __name__ == '__main__':
    main()
