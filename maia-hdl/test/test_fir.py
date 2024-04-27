#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import numpy as np
import scipy.signal

import unittest

from maia_hdl.fir import FIR4DSP, FIR2DSP, FIRDecimator3Stage
from .amaranth_sim import AmaranthSim


def zero_pack(x, n):
    y = np.zeros((x.size, n), dtype=x.dtype)
    y[:, 0] = x
    return y.ravel()


class TestFIR(AmaranthSim):
    def test_FIR4DSP(self):
        self.decimation = 5
        self.operations = 3
        self.odd_operations = False
        self.macc_trunc = 0
        self.FIR4DSP_common()

    def test_FIR4DSP_truncate_round(self):
        self.decimation = 5
        self.operations = 3
        self.odd_operations = False
        self.macc_trunc = 2
        self.FIR4DSP_common()

    def test_FIR4DSP_odd_operations(self):
        self.decimation = 4
        self.operations = 8
        self.odd_operations = True
        self.macc_trunc = 0
        self.min_wait = 4
        self.max_wait = 16
        self.FIR4DSP_common()

    def test_FIR4DSP_one_operation(self):
        self.decimation = 10
        self.operations = 1
        self.odd_operations = False
        self.macc_trunc = 0
        self.min_wait = 0
        self.max_wait = 4
        self.FIR4DSP_common()

    def FIR4DSP_common(self):
        self.dut = FIR4DSP(macc_trunc=self.macc_trunc)
        num_mult = 2 * self.operations
        if self.odd_operations:
            num_mult -= 1
        self.num_taps = self.decimation * num_mult
        self.taps = np.arange(1, self.num_taps // 2 + 1)
        self.taps = np.concatenate((self.taps, self.taps[::-1]))

        num_coeffs = 256
        self.coeffs = np.zeros(num_coeffs, 'int')
        op = self.operations
        dec = self.decimation
        for j in range(op):
            self.coeffs[j::op][:dec] = self.taps[2*j*dec:][:dec][::-1]
            if not self.odd_operations or j != op - 1:
                self.coeffs[num_coeffs//2+j::op][:dec] = (
                        self.taps[(2*j+1)*dec:][:dec][::-1])

        self.fir_common_test()

    def test_FIR2DSP(self):
        self.macc_trunc = 0

    def test_FIR2DSP_macc_trunc(self):
        self.macc_trunc = 2

    def FIR2DSP_common_test(self):
        self.dut = FIR2DSP(macc_trunc=self.macc_trunc)

        self.decimation = 6
        self.operations = 3
        self.num_taps = self.decimation * self.operations
        self.taps = np.arange(1, self.num_taps // 2 + 1)
        self.taps = np.concatenate((self.taps, self.taps[::-1]))

        num_coeffs = 128
        self.coeffs = np.zeros(num_coeffs, 'int')
        op = self.operations
        dec = self.decimation
        for j in range(op):
            self.coeffs[j::op][:dec] = self.taps[j*dec:][:dec][::-1]

        self.min_wait = 0
        self.max_wait = 16
        self.fir_common_test()

    def fir_common_test(self):
        assert self.taps.size == self.num_taps
        re_in = np.zeros(2048, 'int')
        for j in range(self.decimation):
            re_in[4 * self.num_taps * j + j + 10] = 1
        im_in = np.random.randint(-2**15, 2**15, size=re_in.size)
        # set first input samples to 0 to avoid results that do not match
        # the model due to initial samples being written in the wrong memory
        # locations
        keep_out = 7
        im_in[:keep_out] = 0

        model_re, model_im = self.dut.model(
            self.taps, self.decimation, re_in, im_in)

        # drop the first input samples when feeding the DUT to match its
        # polyphase fase with what the model does
        drop_samples = 1

        re_out = np.empty(re_in.size // self.decimation, 'int')
        im_out = np.empty_like(re_out)

        if not hasattr(self, 'min_wait'):
            self.min_wait = 0
        if not hasattr(self, 'max_wait'):
            self.max_wait = 0

        def set_inputs():
            if hasattr(self.dut, 'odd_operations'):
                yield self.dut.odd_operations.eq(self.odd_operations)
            yield self.dut.decimation.eq(self.decimation)
            yield self.dut.operations_minus_one.eq(self.operations - 1)

            # load coefficients
            yield self.dut.coeff_wren.eq(1)
            for addr, coeff in enumerate(self.coeffs):
                yield self.dut.coeff_waddr.eq(addr)
                yield self.dut.coeff_wdata.eq(int(coeff))
                yield
            yield self.dut.coeff_wren.eq(0)

            # feed samples
            for j, z in enumerate(zip(re_in[drop_samples:],
                                      im_in[drop_samples:])):
                re, im = z
                # wait for some time to make sample valid
                yield self.dut.in_valid.eq(0)
                wait_cycles = np.random.randint(self.min_wait,
                                                self.max_wait + 1)
                for _ in range(wait_cycles):
                    yield
                yield self.dut.in_valid.eq(1)
                yield self.dut.re_in.eq(int(re))
                yield self.dut.im_in.eq(int(im))
                while True:
                    yield
                    if (yield self.dut.in_ready):
                        break

        def check_outputs():
            for j in range(re_out.size):
                while True:
                    yield
                    if (yield self.dut.strobe_out):
                        re_out[j] = yield self.dut.re_out
                        im_out[j] = yield self.dut.im_out
                        break
            np.testing.assert_equal(re_out, model_re,
                                    'real parts do not match')
            np.testing.assert_equal(im_out, model_im,
                                    'imaginary parts do not match')

        self.simulate([set_inputs, check_outputs])


class TestFIRDecimator(AmaranthSim):
    def test_fir_decimator_3stage(self):
        self.delta_f = 0.05
        self.fs = 0.5
        self.fp = self.fs * (1 - self.delta_f)
        self.dut = FIRDecimator3Stage()

        D = 42
        D1 = 7
        D2 = 3
        D3 = 2
        N1 = 28
        N2 = 24
        N3 = 134

        operations1 = N1 // D1
        odd1 = operations1 % 2 == 1
        operations1 = (operations1 + 1) // 2
        operations2 = N2 // D2
        operations3 = N3 // D3
        odd3 = operations3 % 2 == 1
        operations3 = (operations3 + 1) // 2

        h1, h2, h3 = self.all_stages(D, D1, D2, D3, N1, N2, N3)

        coeff_width = self.dut.coeff_width
        macc_trunc = np.array(self.dut.macc_trunc)
        # this requires in_width = 12, out_width = [16]*3
        width_growth = np.array([4, 0, 0])
        stage_growth = macc_trunc + width_growth
        max_coeff = 2**(coeff_width - 1) - 1

        h1_max_scale = max_coeff / np.max(np.abs(h1))
        h1_scale_desired = 2**stage_growth[0] / np.sum(np.abs(h1))
        h1_scale = min(h1_scale_desired, h1_max_scale)
        h1 = np.round(h1 * h1_scale)

        h2_max_scale = max_coeff / np.max(np.abs(h2))
        h1h2 = np.convolve(h1, zero_pack(h2, D1), mode='full')
        h2_scale_desired = 2**np.sum(stage_growth[:2]) / np.sum(np.abs(h1h2))
        h2_scale = min(h2_scale_desired, h2_max_scale)
        h2 = np.round(h2 * h2_scale)

        h3_max_scale = max_coeff / np.max(np.abs(h3))
        h1h2h3 = np.convolve(np.convolve(h1, zero_pack(h2, D1), mode='full'),
                             zero_pack(h3, D1 * D2), mode='full')
        h3_scale_desired = 2**np.sum(stage_growth) / np.sum(np.abs(h1h2h3))
        h3_scale = min(h3_scale_desired, h3_max_scale)
        h3 = np.round(h3 * h3_scale)

        num_coeffs = 256
        coeffs1 = np.zeros(num_coeffs, 'int')
        coeffs2 = np.zeros(128, 'int')
        coeffs3 = np.zeros(num_coeffs, 'int')

        op = operations1
        dec = D1
        odd = odd1
        taps = h1
        for j in range(op):
            coeffs1[j::op][:dec] = taps[2*j*dec:][:dec][::-1]
            if not odd or j != op - 1:
                coeffs1[num_coeffs//2+j::op][:dec] = (
                        taps[(2*j+1)*dec:][:dec][::-1])

        op = operations2
        dec = D2
        taps = h2
        for j in range(op):
            coeffs2[j::op][:dec] = taps[j*dec:][:dec][::-1]

        op = operations3
        dec = D3
        odd = odd3
        taps = h3
        for j in range(op):
            coeffs3[j::op][:dec] = taps[2*j*dec:][:dec][::-1]
            if not odd or j != op - 1:
                coeffs3[num_coeffs//2+j::op][:dec] = (
                        taps[(2*j+1)*dec:][:dec][::-1])

        nsamples = 25000

        def set_inputs():
            yield self.dut.decimation1.eq(D1)
            yield self.dut.decimation2.eq(D2)
            yield self.dut.decimation3.eq(D3)
            yield self.dut.bypass2.eq(0)
            yield self.dut.bypass3.eq(0)
            yield self.dut.operations_minus_one1.eq(operations1 - 1)
            yield self.dut.operations_minus_one2.eq(operations2 - 1)
            yield self.dut.operations_minus_one3.eq(operations3 - 1)
            yield self.dut.odd_operations1.eq(odd1)
            yield self.dut.odd_operations3.eq(odd3)

            # load coefficients
            yield self.dut.coeff_wren.eq(1)
            for addr, coeff in enumerate(coeffs1):
                yield self.dut.coeff_waddr.eq(addr)
                yield self.dut.coeff_wdata.eq(int(coeff))
                yield
            for addr, coeff in enumerate(coeffs2):
                yield self.dut.coeff_waddr.eq(addr + 256)
                yield self.dut.coeff_wdata.eq(int(coeff))
                yield
            for addr, coeff in enumerate(coeffs3):
                yield self.dut.coeff_waddr.eq(addr + 512)
                yield self.dut.coeff_wdata.eq(int(coeff))
                yield
            yield self.dut.coeff_wren.eq(0)

            # feed samples
            amplitude = 2**11 - 1
            phase = 0
            freq = 0
            dfreq = 1e-5
            for _ in range(nsamples):
                z = np.exp(1j * phase)
                phase = (phase + freq + np.pi) % (2 * np.pi) - np.pi
                freq += dfreq
                re = int(np.round(amplitude * z.real))
                im = int(np.round(amplitude * z.imag))
                yield self.dut.in_valid.eq(1)
                yield self.dut.re_in.eq(re)
                yield self.dut.im_in.eq(im)
                while True:
                    yield
                    if (yield self.dut.in_ready):
                        break

        def write_output():
            out = np.zeros(2 * (nsamples // D), 'int16')
            for j in range(out.size // 2):
                while True:
                    yield
                    if (yield self.dut.strobe_out):
                        out[2 * j] = yield self.dut.re_out
                        out[2 * j + 1] = yield self.dut.im_out
                        break
            out.tofile('decimator_out.cs16')

        self.simulate([set_inputs, write_output])

    def stage1(self, D, D1, D2, D3, N1):
        # transition [fp, D / D1 - fp]
        h1 = scipy.signal.remez(N1, [0, self.fp, D/D1 - self.fp, 0.5*D],
                                [1, 0], weight=[1, 1], fs=D)
        h1 /= np.sum(h1)
        return h1

    def stage2(self, D, D1, D2, D3, N2):
        # transition [fp, D3 - fp]
        h2 = scipy.signal.remez(N2, [0, self.fp, D3 - self.fp, 0.5*D/D1],
                                [1, 0], weight=[1, 1], fs=D/D1)
        h2 /= np.sum(h2)
        return h2

    def stage3(self, D, D1, D2, D3, N3):
        # transition [fp, fs]
        h3 = scipy.signal.remez(N3, [0, self.fp, 1 - self.fp, 0.5*D3],
                                [1, 0], weight=[1, 1], fs=D3)
        h3 /= np.sum(h3)
        return h3

    def all_stages(self, D, D1, D2, D3, N1, N2, N3):
        h1 = self.stage1(D, D1, D2, D3, N1)
        h2 = self.stage2(D, D1, D2, D3, N2)
        h3 = self.stage3(D, D1, D2, D3, N3)
        return h1, h2, h3


if __name__ == '__main__':
    unittest.main()
