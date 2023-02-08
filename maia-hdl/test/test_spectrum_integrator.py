#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
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


class TestSpectrumIntegrator(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.nint_width = 8
        self.read_delay = 2  # we are using a BRAM output register

    def test_model(self):
        self.fft_order_log2 = 8
        self.nfft = 2**self.fft_order_log2
        for integrations in [5, 2]:
            with self.subTest(integrations=integrations):
                self.common_model(integrations)

    def common_model(self, integrations):
        self.dut = SpectrumIntegrator(
            self.width, self.nint_width, self.fft_order_log2)

        re_in, im_in = (
            np.random.randint(-2**(self.width-1), 2**(self.width-1),
                              size=(3*integrations + 1)*self.nfft)
            for _ in range(2))

        def set_inputs():
            yield self.dut.nint.eq(integrations)
            for j, x in enumerate(zip(re_in, im_in)):
                re = x[0]
                im = x[1]
                yield self.dut.re_in.eq(int(re))
                yield self.dut.im_in.eq(int(im))
                yield self.dut.input_last.eq(
                    j % self.nfft == self.nfft - 1)
                yield self.dut.clken.eq(1)
                yield
                yield self.dut.clken.eq(0)
                yield

        def check_ram_contents():
            def wait_ready():
                while True:
                    yield
                    if (yield self.dut.done):
                        return

            def check_ram(expected):
                read = []
                yield self.dut.rden.eq(1)
                for j in range(self.nfft + self.read_delay):
                    if j < self.nfft:
                        yield self.dut.rdaddr.eq(j)
                    yield
                    if j >= self.read_delay:
                        k = j - self.read_delay
                        assert (yield self.dut.rdata) == expected[k]

            # The first run doesn't produce good results, so we don't check
            # anything.
            yield from wait_ready()
            for n in range(2):
                yield from wait_ready()
                sel = slice(
                    (n * integrations + 1) * self.nfft,
                    ((n + 1) * integrations + 1) * self.nfft)
                expected = self.dut.model(
                    integrations, re_in[sel], im_in[sel])
                yield from check_ram(expected)

        self.simulate([set_inputs, check_ram_contents])

    def test_constant_input(self):
        self.fft_order_log2 = 6
        self.nfft = 2**self.fft_order_log2

        self.dut = SpectrumIntegrator(
            self.width, self.nint_width, self.fft_order_log2)
        integrations = 5

        def set_inputs():
            yield self.dut.nint.eq(integrations)
            for n in range(10 * integrations):
                integration_num = (n - 1) // integrations
                amplitude = 2**(self.width//2 + (integration_num % 2) + 1)
                for j in range(self.nfft):
                    yield self.dut.re_in.eq(0 if j % 2 else amplitude)
                    yield self.dut.im_in.eq(amplitude if j % 2 else 0)
                    yield self.dut.input_last.eq(
                        j % self.nfft == self.nfft - 1)
                    yield self.dut.clken.eq(1)
                    yield
                    yield self.dut.clken.eq(0)
                    yield
                    yield

        def check_ram_contents():
            def wait_ready():
                while True:
                    yield
                    if (yield self.dut.done):
                        return

            def check(num_check):
                amplitude = 8 if num_check % 2 else 2
                yield self.dut.rden.eq(1)
                for j in range(self.nfft + self.read_delay):
                    if j < self.nfft:
                        yield self.dut.rdaddr.eq(j)
                    yield
                    if j >= self.read_delay:
                        assert \
                            (yield self.dut.rdata) == integrations * amplitude

            # The first run doesn't produce good results, so we don't check
            # anything.
            yield from wait_ready()
            for n in range(6):
                yield from wait_ready()
                yield from check(n)

        self.simulate([set_inputs, check_ram_contents]),


if __name__ == '__main__':
    unittest.main()
