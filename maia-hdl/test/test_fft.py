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

from maia_hdl.fft import R2SDF, R4SDF, R22SDF, TwiddleI, Twiddle, Window, FFT
from maia_hdl.util import bit_invert
from .amaranth_sim import AmaranthSim
from .common_edge import CommonEdgeTb


class TestR2SDF(AmaranthSim):
    def test_model(self):
        self.order = 5
        self.width_in = 24
        for truncate in [0, 1]:
            for storage in ['distributed', 'bram']:
                for use_bram_reg in [False, True]:
                    with self.subTest(truncate=truncate,
                                      storage=storage,
                                      use_bram_reg=use_bram_reg):
                        self.common_test_model(
                            truncate, storage, use_bram_reg)

    def common_test_model(self, truncate, storage, use_bram_reg):
        self.dut = R2SDF(self.order, self.width_in,
                         truncate=truncate, storage=storage,
                         use_bram_reg=use_bram_reg)

        n_vec = 64
        re_in, im_in = (
            np.random.randint(
                -2**(self.width_in-1), 2**(self.width_in-1),
                n_vec * self.dut.model_vlen)
            for _ in range(2))

        async def set_inputs(ctx):
            for j in range(re_in.size + self.dut.delay):
                await ctx.tick()
                ctx.set(self.dut.clken, 1)
                if j < re_in.size:
                    ctx.set(self.dut.re_in, int(re_in[j]))
                    ctx.set(self.dut.im_in, int(im_in[j]))
                mux_control = (j // 2**(self.order - 1)) % 2
                ctx.set(self.dut.mux_control, mux_control)
                if storage == 'bram':
                    waddr = j % 2**(self.order - 1)
                    offset = 1 if not use_bram_reg else 2
                    ctx.set(self.dut.bram_raddr, waddr + offset)
                    ctx.set(self.dut.bram_waddr, waddr)

        async def read_outputs(ctx):
            await ctx.tick().repeat(self.dut.delay)
            re_out, im_out = (
                np.empty_like(re_in) for _ in range(2))
            for j in range(re_out.size):
                await ctx.tick()
                re_out[j] = ctx.get(self.dut.re_out)
                im_out[j] = ctx.get(self.dut.im_out)
            model_re, model_im = self.dut.model(re_in, im_in)
            np.testing.assert_equal(re_out, model_re,
                                    'real parts do not match')
            np.testing.assert_equal(im_out, model_im,
                                    'imaginary parts do not match')

        self.simulate([set_inputs, read_outputs])


class TestR4SDF(AmaranthSim):
    def test_model(self):
        self.order = 2
        self.width_in = 24
        for truncate in range(3):
            for storage in ['distributed', 'bram']:
                for use_bram_reg in [False, True]:
                    with self.subTest(truncate=truncate,
                                      storage=storage,
                                      use_bram_reg=use_bram_reg):
                        self.common_test_model(
                            truncate, storage, use_bram_reg)

    def common_test_model(self, truncate, storage, use_bram_reg):
        self.dut = R4SDF(self.order, self.width_in,
                         truncate=truncate, storage=storage,
                         use_bram_reg=use_bram_reg)

        n_vec = 64
        re_in, im_in = (
            np.random.randint(
                -2**(self.width_in-1), 2**(self.width_in-1),
                n_vec * self.dut.model_vlen)
            for _ in range(2))

        async def set_inputs(ctx):
            for j in range(re_in.size + self.dut.delay):
                await ctx.tick()
                ctx.set(self.dut.clken, 1)
                if j < re_in.size:
                    ctx.set(self.dut.re_in, int(re_in[j]))
                    ctx.set(self.dut.im_in, int(im_in[j]))
                mux_control = (j // 4**(self.order - 1)) % 4 == 3
                ctx.set(self.dut.mux_control, mux_control)
                if storage == 'bram':
                    waddr = j % 4**(self.order - 1)
                    offset = 1 if not use_bram_reg else 2
                    ctx.set(self.dut.bram_raddr, waddr + offset)
                    ctx.set(self.dut.bram_waddr, waddr)

        async def read_outputs(ctx):
            await ctx.tick().repeat(self.dut.delay)
            re_out, im_out = (
                np.empty_like(re_in) for _ in range(2))
            for j in range(re_out.size):
                await ctx.tick()
                re_out[j] = ctx.get(self.dut.re_out)
                im_out[j] = ctx.get(self.dut.im_out)
            model_re, model_im = self.dut.model(re_in, im_in)
            np.testing.assert_equal(re_out, model_re,
                                    'real parts do not match')
            np.testing.assert_equal(im_out, model_im,
                                    'imaginary parts do not match')

        self.simulate([set_inputs, read_outputs])


class TestR22SDF(AmaranthSim):
    def test_model(self):
        self.order = 2
        self.width_in = 24
        for truncate in [[0, 0], [0, 1], [1, 0], [1, 1]]:
            for storage in ['distributed', 'bram']:
                for use_bram_reg in [False, True]:
                    with self.subTest(truncate=truncate,
                                      storage=storage,
                                      use_bram_reg=use_bram_reg):
                        self.common_test_model(
                            truncate, storage, use_bram_reg)

    def common_test_model(self, truncate, storage, use_bram_reg):
        self.dut = R22SDF(self.order, self.width_in,
                          truncate=truncate, storage=storage,
                          use_bram_reg=use_bram_reg)

        n_vec = 64
        re_in, im_in = (
            np.random.randint(
                -2**(self.width_in-1), 2**(self.width_in-1),
                n_vec * self.dut.model_vlen)
            for _ in range(2))

        async def set_inputs(ctx):
            for j in range(re_in.size + self.dut.delay):
                await ctx.tick()
                ctx.set(self.dut.clken, 1)
                if j < re_in.size:
                    ctx.set(self.dut.re_in, int(re_in[j]))
                    ctx.set(self.dut.im_in, int(im_in[j]))
                mux_count = (j // 4**(self.order - 1)) % 4
                ctx.set(self.dut.mux_count, mux_count)
                if storage == 'bram':
                    waddr = j % 2**(2 * self.order - 1)
                    offset = 1 if not use_bram_reg else 2
                    ctx.set(self.dut.bram_raddr, waddr + offset)
                    ctx.set(self.dut.bram_waddr, waddr)

        async def read_outputs(ctx):
            await ctx.tick().repeat(self.dut.delay)
            re_out, im_out = (
                np.empty_like(re_in) for _ in range(2))
            for j in range(re_out.size):
                await ctx.tick()
                re_out[j] = ctx.get(self.dut.re_out)
                im_out[j] = ctx.get(self.dut.im_out)
            model_re, model_im = self.dut.model(re_in, im_in)
            np.testing.assert_equal(re_out, model_re,
                                    'real parts do not match')
            np.testing.assert_equal(im_out, model_im,
                                    'imaginary parts do not match')

        self.simulate([set_inputs, read_outputs])


class TestTwiddle(AmaranthSim):
    def setUp(self):
        self.width = 24

    def test_twiddleI(self):
        self.dut = TwiddleI(self.width)
        self.common_test_model()

    def test_twiddle_lut(self):
        self.dut = Twiddle(5, 1, self.width, self.width,
                           storage='lut')
        self.common_test_model()

    def test_twiddle_bram(self):
        self.dut = Twiddle(3, 2, self.width, self.width,
                           storage='bram')
        self.common_test_model()

    def common_test_model(self):
        n_vec = 64
        adv = self.dut.twiddle_index_advance
        re_in, im_in = (
            np.random.randint(
                -2**(self.width-1), 2**(self.width-1),
                n_vec * self.dut.model_vlen)
            for _ in range(2))

        async def set_inputs(ctx):
            for j in range(re_in.size + self.dut.delay):
                await ctx.tick()
                ctx.set(self.dut.clken, 1)
                if j < re_in.size:
                    ctx.set(self.dut.re_in, int(re_in[j]))
                    ctx.set(self.dut.im_in, int(im_in[j]))
                twiddle_index = (j + adv) % self.dut.model_vlen
                ctx.set(self.dut.twiddle_index, twiddle_index)

        async def read_outputs(ctx):
            await ctx.tick().repeat(self.dut.delay)
            re_out, im_out = (
                np.empty_like(re_in) for _ in range(2))
            for j in range(re_out.size):
                await ctx.tick()
                re_out[j] = ctx.get(self.dut.re_out)
                im_out[j] = ctx.get(self.dut.im_out)
            model_re, model_im = self.dut.model(re_in, im_in)
            # The first twiddle_index_advance elements should not be checked
            # because the BRAM read pipeline is still not full, so they produce
            # 0's (or whatever is in the BRAM reset state).
            np.testing.assert_equal(
                re_out[adv:], model_re[adv:], 'real parts do not match')
            np.testing.assert_equal(
                im_out[adv:], model_im[adv:], 'imaginary parts do not match')

        self.simulate([set_inputs, read_outputs])


class TestWindow(AmaranthSim):
    def test_model(self):
        domain_2x = 'clk2x'
        order_log2 = 12
        sample_width = 16
        coeff_width = 18
        self.window = Window(domain_2x, order_log2, sample_width, coeff_width)
        self.dut = CommonEdgeTb(self.window, [(domain_2x, 2, 'common_edge')])

        n_vec = 2
        re_in, im_in = (
            np.random.randint(
                -2**(sample_width-1), 2**(sample_width-1),
                n_vec * self.window.model_vlen)
            for _ in range(2))

        async def set_inputs(ctx):
            for j in range(re_in.size + self.window.delay):
                await ctx.tick()
                ctx.set(self.window.clken, 1)
                if j < re_in.size:
                    ctx.set(self.window.re_in, int(re_in[j]))
                    ctx.set(self.window.im_in, int(im_in[j]))
                coeff_index = (
                    (j + self.window.coeff_index_advance) % 2**order_log2)
                ctx.set(self.window.coeff_index, coeff_index)

        async def read_outputs(ctx):
            await ctx.tick().repeat(self.window.delay)
            re_out, im_out = (
                np.empty_like(re_in) for _ in range(2))
            for j in range(re_out.size):
                await ctx.tick()
                re_out[j] = ctx.get(self.window.re_out)
                im_out[j] = ctx.get(self.window.im_out)
            model_re, model_im = self.window.model(re_in, im_in)
            # The first coeff_index_advance elements should not be checked
            # because the BRAM read pipeline is still not full, so they produce
            # 0's (or whatever is in the BRAM reset state).
            adv = self.window.coeff_index_advance
            np.testing.assert_equal(re_out[adv:], model_re[adv:],
                                    'real parts do not match')
            np.testing.assert_equal(im_out[adv:], model_im[adv:],
                                    'imaginary parts do not match')

        self.simulate([set_inputs, read_outputs],
                      named_clocks={domain_2x: 6e-9})


class TestFFT(AmaranthSim):
    def setUp(self):
        self.width = 16
        self.order_log2 = 6
        self.fft_size = 2**self.order_log2

    def test_model_vs_numpy(self):
        for radix, radix_log2 in zip([2, 4, 'R22'], [1, 2, 2]):
            with self.subTest(radix=radix):
                for no_truncate in [False, True]:
                    with self.subTest(no_truncate=no_truncate):
                        self.model_vs_numpy(radix, radix_log2, no_truncate)

    def model_vs_numpy(self, radix, radix_log2, no_truncate):
        truncates = None
        if no_truncate:
            nstages = self.order_log2 // radix_log2
            if radix == 'R22':
                truncates = [[0, 0]] * nstages
            else:
                truncates = [0] * nstages
        self.dut = FFT(self.width, self.order_log2, radix,
                       truncates=truncates)
        self.dummy_simulation()  # keep amaranth happy
        n_vec = 2048
        fft_size = self.fft_size

        def gen_input():
            # generate a signed self.width bit complex number
            # with amplitude <= 2**(self.width-1) - 1
            while True:
                re, im = [
                    np.random.randint(
                        -2**(self.width-1), 2**(self.width-1))
                    for j in range(2)
                ]
                if re**2 + im**2 <= (2**(self.width-1) - 1)**2:
                    return re, im

        z_in = [gen_input() for _ in range(n_vec * fft_size)]
        re_in = np.array([z[0] for z in z_in], 'int')
        im_in = np.array([z[1] for z in z_in], 'int')
        re_out, im_out = self.dut.model(re_in, im_in)
        out_complex = re_out + 1j * im_out
        in_complex = (re_in + 1j * im_in).reshape(
            n_vec, fft_size)
        out_npy = np.fft.fft(in_complex) / fft_size
        if no_truncate:
            out_npy *= fft_size
        # Perform bit-order inversion at the output of the numpy FFT.
        bitinvert_radix = radix_log2 if radix != 'R22' else 1
        invert = np.array([
            bit_invert(n, self.order_log2, bitinvert_radix)
            for n in range(fft_size)])
        out_npy = out_npy[:, invert].ravel()
        relative_error = np.sqrt(
            np.sum(np.abs(out_complex - out_npy)**2)
            / np.sum(np.abs(out_npy)**2)
        )
        assert relative_error < 4e-4, \
            (f'FFT relative error {relative_error} too large\n'
             f'model: {out_complex}\n'
             f'numpy: {out_npy}')

    def dummy_simulation(self):
        # Dummy simulation, to keep amaranth happy (otherwise amaranth
        # complains that we didn't use the DUT if we only use it to run
        # the model).
        async def dummy(ctx):
            pass

        self.simulate(dummy)

    def test_deltas_and_exps_radix2(self):
        self.radix = 2
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_radix2_window(self):
        self.radix = 2
        self.domain_2x = 'clk2x'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False, window='blackmanharris',
                       domain_2x=self.domain_2x)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_bfly_radix2(self):
        self.radix = 2
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       butterfly_storage='bram', use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_twiddles_radix2(self):
        self.radix = 2
        self.fft = FFT(
            self.width, self.order_log2, self.radix,
            twiddle_storage='bram', use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_radix4(self):
        self.radix = 4
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_radix4_window(self):
        self.radix = 4
        self.domain_2x = 'clk2x'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False, window='blackmanharris',
                       domain_2x=self.domain_2x)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_bfly_radix4(self):
        self.radix = 4
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       butterfly_storage='bram', use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_twiddles_radix4(self):
        self.radix = 4
        self.fft = FFT(
            self.width, self.order_log2, self.radix,
            twiddle_storage='bram', use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_radix22(self):
        self.radix = 'R22'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_radix22_window(self):
        self.radix = 'R22'
        self.domain_2x = 'clk2x'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False, window='blackmanharris',
                       domain_2x=self.domain_2x)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_radix22_window_cmult3x(self):
        self.radix = 'R22'
        self.domain_2x = 'clk2x'
        self.domain_3x = 'clk3x'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       use_bram_reg=False, window='blackmanharris',
                       cmult3x=True,
                       domain_2x=self.domain_2x, domain_3x=self.domain_3x)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_bfly_radix22(self):
        self.radix = 'R22'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       butterfly_storage='bram', use_bram_reg=False)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_bfly_reg_radix22(self):
        self.radix = 'R22'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       butterfly_storage='bram', use_bram_reg=True)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_bram_bfly_reg_radix22_window(self):
        self.radix = 'R22'
        self.domain_2x = 'clk2x'
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       butterfly_storage='bram', use_bram_reg=True,
                       window='blackmanharris', domain_2x=self.domain_2x)
        self.common_deltas_and_exps()

    def test_deltas_and_exps_truncates(self):
        self.radix = 'R22'
        truncates = [[0, 0], [0, 1], [1, 1]]
        self.fft = FFT(self.width, self.order_log2, self.radix,
                       truncates=truncates)
        self.common_deltas_and_exps()

    def common_deltas_and_exps(self, vcd=None):
        domains = []
        if hasattr(self, 'domain_2x'):
            domains.append((self.domain_2x, 2, 'common_edge_2x'))
        if hasattr(self, 'domain_3x'):
            domains.append((self.domain_3x, 3, 'common_edge_3x'))
        self.dut = CommonEdgeTb(self.fft, domains)
        self.radix_log2 = (2 if self.radix == 'R22'
                           else int(np.log2(self.radix)))
        fft_size = self.fft_size
        # Required when the FFT uses a window, in order to fill
        # up the pipeline of the window BRAM.
        input_zeros = np.zeros(fft_size)
        input_deltas = np.eye(fft_size).ravel()
        input_exp = np.array([np.exp(1j*2*np.pi*k*np.arange(fft_size)/fft_size)
                              for k in range(fft_size)]).ravel()
        scale = 2**(self.width-1) - 1
        input_all = scale * np.concatenate(
            (input_zeros, input_deltas, input_exp))
        re_in = [int(a) for a in np.round(input_all).real]
        im_in = [int(a) for a in np.round(input_all).imag]

        async def set_inputs(ctx):
            for j in range(len(re_in)):
                await ctx.tick()
                ctx.set(self.fft.clken, 1)
                ctx.set(self.fft.re_in, re_in[j])
                ctx.set(self.fft.im_in, im_in[j])
            await ctx.tick()
            ctx.set(self.fft.re_in, 0)
            ctx.set(self.fft.im_in, 0)

        async def read_outputs(ctx):
            await ctx.tick().repeat(self.fft.delay)
            re_out, im_out = (
                np.empty(input_all.size, 'int') for _ in range(2))
            for j in range(input_all.size):
                await ctx.tick()
                re_out[j] = ctx.get(self.fft.re_out)
                im_out[j] = ctx.get(self.fft.im_out)
                out_last = ctx.get(self.fft.out_last)
                if j % fft_size == fft_size - 1:
                    assert out_last
                else:
                    assert not out_last
            re_model, im_model = self.fft.model(re_in, im_in)
            np.testing.assert_equal(re_out, re_model)
            np.testing.assert_equal(im_out, im_model)

        named_clocks = {}
        if hasattr(self, 'domain_2x'):
            named_clocks[self.domain_2x] = 6e-9
        if hasattr(self, 'domain_3x'):
            named_clocks[self.domain_3x] = 4e-9
        self.simulate([set_inputs, read_outputs], vcd=vcd,
                      named_clocks=named_clocks)


if __name__ == '__main__':
    unittest.main()
