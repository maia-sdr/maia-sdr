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

from maia_hdl.axi4_lite import Axi4LiteRegisterBridge
from maia_hdl.register import Access, Field, Register, Registers


def main():
    m = Module()
    address_width = 2
    m.submodules.registers = registers = Registers(
            'registers',
            {
                0b00: Register('id', [Field('id', Access.R, 32, 0xf001baa2)]),
                0b01: Register('rega', [Field('f0', Access.RW, 32, 0x1234)]),
                0b10: Register('regb', [Field('f2', Access.RW, 32, 0)]),
            },
            address_width)
    m.submodules.bridge = bridge = Axi4LiteRegisterBridge(address_width)
    m.d.comb += [
        registers.ren.eq(bridge.ren),
        registers.wstrobe.eq(bridge.wstrobe),
        registers.address.eq(bridge.address),
        registers.wdata.eq(bridge.wdata),
        bridge.rdone.eq(registers.rdone),
        bridge.wdone.eq(registers.wdone),
        bridge.rdata.eq(registers.rdata),
    ]
    with open('dut.v', 'w') as f:
        f.write(convert(
            m, name='dut', ports=bridge.axi.ports(), emit_src=False))


if __name__ == '__main__':
    main()
