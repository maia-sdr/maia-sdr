#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth.sim import Simulator

import unittest


class AmaranthSim(unittest.TestCase):
    def simulate(self, benches, *, vcd=None, named_clocks={}):
        sim = Simulator(self.dut)
        sim.add_clock(12e-9)
        for domain, period in named_clocks.items():
            sim.add_clock(period, domain=domain, phase=6e-9)
        if hasattr(benches, '__iter__'):
            for bench in benches:
                sim.add_testbench(bench)
        else:
            sim.add_testbench(benches)
        if vcd is None:
            sim.run()
        else:
            with sim.write_vcd(vcd):
                sim.run()
