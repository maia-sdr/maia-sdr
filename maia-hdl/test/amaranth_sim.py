#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth.sim import Simulator

import unittest


class AmaranthSim(unittest.TestCase):
    def simulate(self, benches, vcd=None, named_clocks={}):
        sim = Simulator(self.dut)
        sim.add_clock(12e-9)
        for domain, period in named_clocks.items():
            # we add 1/2 picosecond to the phase because Amaranth computes the
            # floor instead of rounding, when converting the period and phase
            # to integer picoseconds. This can give wrong results due to
            # floating point resolution.
            half_ps = 0.5e-12
            sim.add_clock(period, domain=domain, phase=6e-9-period/2 + half_ps)
        if hasattr(benches, '__iter__'):
            for bench in benches:
                sim.add_sync_process(bench)
        else:
            sim.add_sync_process(benches)
        if vcd is None:
            sim.run()
        else:
            with sim.write_vcd(vcd):
                sim.run()
