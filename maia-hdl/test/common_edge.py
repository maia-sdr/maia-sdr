#
# Copyright (C) 2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *


class CommonEdgeTb(Elaboratable):
    def __init__(self, dut, domains):
        self.dut = dut
        self.domains = domains

    def elaborate(self, platform):
        m = Module()
        m.submodules.dut = self.dut
        for domain, nx, name in self.domains:
            if hasattr(self.dut, name):
                common_edge_del = Signal(nx, reset=1,
                                         name=f'common_edge_del_{domain}')
                m.d[domain] += common_edge_del.eq(
                    Cat(common_edge_del[-1], common_edge_del))
                m.d.comb += getattr(self.dut, name).eq(common_edge_del[1])
        return m
