#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import numpy as np

import unittest

from maia_hdl.spectrum_integrator import SpectrumIntegrator
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


class TestSpectrumIntegrator(AmaranthSim):
    def setUp(self):
        self.width = 22
        self.fp_width = 18
        self.nint_width = 10
        self.read_delay = 2  # we are using a BRAM output register
        self.domain_3x = 'clk3x'

    def test_model(self):
        self.fft_order_log2 = 8
        self.nfft = 2**self.fft_order_log2
        for peak_detect in [False, True]:
            for integrations in [5, 2]:
                with self.subTest(integrations=integrations,
                                  peak_detect=peak_detect):
                    self.common_model(integrations, peak_detect)

    def common_model(self, integrations, peak_detect):
        self.dut0 = SpectrumIntegrator(
            self.domain_3x, self.width, self.fp_width, self.nint_width,
            self.fft_order_log2)
        self.dut = CommonEdgeTb(
            self.dut0, [(self.domain_3x, 3, 'common_edge')])

        re_in, im_in = (
            np.random.randint(-2**(self.width-1), 2**(self.width-1),
                              size=(3*integrations + 1)*self.nfft)
            for _ in range(2))

        async def set_inputs(ctx):
            ctx.set(self.dut0.nint, integrations)
            ctx.set(self.dut0.peak_detect, peak_detect)
            for j, x in enumerate(zip(re_in, im_in)):
                await ctx.tick()
                re = x[0]
                im = x[1]
                ctx.set(self.dut0.re_in, int(re))
                ctx.set(self.dut0.im_in, int(im))
                ctx.set(self.dut0.input_last,
                        j % self.nfft == self.nfft - 1)
                ctx.set(self.dut0.clken, 1)
                await ctx.tick()
                ctx.set(self.dut0.clken, 0)

        async def check_ram_contents(ctx):
            async def wait_ready():
                while True:
                    await ctx.tick()
                    if ctx.get(self.dut0.done):
                        return

            async def check_ram(expected, expected_exponent):
                read = []
                for j in range(self.nfft + self.read_delay):
                    ctx.set(self.dut0.rden, 1)
                    if j < self.nfft:
                        ctx.set(self.dut0.rdaddr, j)
                    if j >= self.read_delay:
                        k = j - self.read_delay
                        value = ctx.get(self.dut0.rdata_value)
                        exponent = ctx.get(self.dut0.rdata_exponent)
                        assert value == expected[k], \
                            (f'value = {value}, '
                             f'expected = {expected[k]} @ k = {k}')
                        assert exponent == expected_exponent[k], \
                            (f'exponent = {exponent}, '
                             f'expected = {expected_exponent[k]} @ k = {k}')
                    await ctx.tick()
                ctx.set(self.dut0.rden, 0)

            # The first run doesn't produce good results, so we don't check
            # anything.
            await wait_ready()
            for n in range(2):
                await wait_ready()
                sel = slice(
                    (n * integrations + 1) * self.nfft,
                    ((n + 1) * integrations + 1) * self.nfft)
                expected = self.dut0.model(
                    integrations, re_in[sel], im_in[sel], peak_detect)
                await check_ram(*expected)

        self.simulate([set_inputs, check_ram_contents],
                      named_clocks={self.domain_3x: 4e-9})

    def test_constant_input(self):
        for peak_detect in [False, True]:
            with self.subTest(peak_detect):
                self.common_constant_input(peak_detect)

    def common_constant_input(self, peak_detect):
        self.fft_order_log2 = 6
        self.nfft = 2**self.fft_order_log2

        self.dut0 = SpectrumIntegrator(
            self.domain_3x, self.width, self.fp_width, self.nint_width,
            self.fft_order_log2)
        self.dut = CommonEdgeTb(
            self.dut0, [(self.domain_3x, 3, 'common_edge')])
        integrations = 5

        async def set_inputs(ctx):
            ctx.set(self.dut0.nint, integrations)
            ctx.set(self.dut0.peak_detect, peak_detect)
            for n in range(10 * integrations):
                integration_num = (n - 1) // integrations
                amplitude = 2**(integration_num % 2)
                for j in range(self.nfft):
                    await ctx.tick()
                    ctx.set(self.dut0.re_in, 0 if j % 2 else amplitude)
                    ctx.set(self.dut0.im_in, amplitude if j % 2 else 0)
                    ctx.set(self.dut0.input_last,
                            j % self.nfft == self.nfft - 1)
                    ctx.set(self.dut0.clken, 1)
                    await ctx.tick()
                    ctx.set(self.dut0.clken, 0)

        async def check_ram_contents(ctx):
            async def wait_ready():
                while True:
                    await ctx.tick()
                    if ctx.get(self.dut0.done):
                        return

            async def check(num_check):
                amplitude = 4**(num_check % 2)
                expected_out = (amplitude if peak_detect
                                else integrations * amplitude)
                for j in range(self.nfft + self.read_delay):
                    ctx.set(self.dut0.rden, 1)
                    if j < self.nfft:
                        ctx.set(self.dut0.rdaddr, j)
                    if j >= self.read_delay:
                        value = ctx.get(self.dut0.rdata_value)
                        exponent = ctx.get(self.dut0.rdata_exponent)
                        assert value == expected_out
                        assert exponent == 0
                    await ctx.tick()
                ctx.set(self.dut0.rden, 0)

            # The first run doesn't produce good results, so we don't check
            # anything.
            await wait_ready()
            for n in range(6):
                await wait_ready()
                await check(n)

        self.simulate([set_inputs, check_ram_contents],
                      named_clocks={self.domain_3x: 4e-9}),


if __name__ == '__main__':
    unittest.main()
