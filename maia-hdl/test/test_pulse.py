#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *

import unittest

from maia_hdl.pulse import PulseStretcher
from .amaranth_sim import AmaranthSim


class TestPulseStretcher(AmaranthSim):
    def test_pulse_stretcher(self):
        pulse_len_log2 = 2
        pulse_len = 2**pulse_len_log2
        self.dut = PulseStretcher(pulse_len_log2)

        def bench():
            for _ in range(10):
                yield
                assert not (yield self.dut.pulse_out)
            yield self.dut.pulse_in.eq(1)
            assert not (yield self.dut.pulse_out)
            yield
            yield self.dut.pulse_in.eq(0)
            assert not (yield self.dut.pulse_out)
            yield
            for _ in range(pulse_len):
                assert (yield self.dut.pulse_out)
                yield
            for _ in range(15):
                assert not (yield self.dut.pulse_out)
                yield
            yield self.dut.pulse_in.eq(1)
            assert not (yield self.dut.pulse_out)
            yield
            yield self.dut.pulse_in.eq(0)
            assert not (yield self.dut.pulse_out)
            yield
            assert (yield self.dut.pulse_out)
            yield
            yield self.dut.pulse_in.eq(1)
            assert (yield self.dut.pulse_out)
            yield
            yield self.dut.pulse_in.eq(0)
            for _ in range(pulse_len + 1):
                assert (yield self.dut.pulse_out)
                yield
            for _ in range(8):
                assert not (yield self.dut.pulse_out)
                yield

        self.simulate(bench)


if __name__ == '__main__':
    unittest.main()
