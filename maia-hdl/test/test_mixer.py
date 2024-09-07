#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import numpy as np

import unittest

from maia_hdl.mixer import Mixer
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


class TestMixer(AmaranthSim):
    def test_mixer(self):
        domain_3x = 'clk3x'
        width = 16
        mixer = Mixer(domain_3x, width)
        freq = round(0.01 * 2**mixer.nco_width)
        self.dut = CommonEdgeTb(
            mixer, [(domain_3x, 3, 'common_edge')])
        num_inputs = 1000
        re, im = [np.random.randint(-2**(width-1), 2**(width-1),
                                    size=num_inputs)
                  for _ in range(2)]
        # skip the first few output samples, because the BRAM
        # pipeline is not loaded yet
        skip = 2
        expected_re, expected_im = mixer.model(freq, re[skip:], im[skip:])
        go_back = mixer.delay + skip

        async def bench(ctx):
            for j in range(num_inputs):
                await ctx.tick()
                ctx.set(mixer.clken, 1)
                ctx.set(mixer.frequency, freq)
                ctx.set(mixer.re_in, int(re[j]))
                ctx.set(mixer.im_in, int(im[j]))
                if j >= go_back:
                    out = (
                        ctx.get(mixer.re_out)
                        + 1j * ctx.get(mixer.im_out))
                    expected = (expected_re[j - go_back]
                                + 1j * expected_im[j - go_back])
                    assert out == expected, \
                        f'out = {out}, expected = {expected} @ cycle = {j}'

        self.simulate(bench, named_clocks={domain_3x: 4e-9})


if __name__ == '__main__':
    unittest.main()
