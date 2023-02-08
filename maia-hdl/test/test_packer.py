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

from maia_hdl.packer import Pack12IQto32, Pack8IQto32, PackFifoTwice
from .amaranth_sim import AmaranthSim


class TestPack12IQto32(AmaranthSim):
    def test_pack(self):
        nsamples = 4096
        re = np.random.randint(-2**11, 2**11, size=nsamples)
        im = np.random.randint(-2**11, 2**11, size=nsamples)
        self.dut = Pack12IQto32()

        def set_input():
            yield self.dut.enable.eq(1)
            for r, i in zip(re, im):
                while True:
                    yield self.dut.re_in.eq(int(r))
                    yield self.dut.im_in.eq(int(i))
                    strobe = int(np.random.randint(2))
                    yield self.dut.strobe_in.eq(strobe)
                    yield
                    if strobe:
                        break

        def check_output():
            data = np.zeros(nsamples // 4 * 3, 'uint32')
            for j in range(data.size):
                while True:
                    yield
                    if (yield self.dut.strobe_out):
                        data[j] = yield self.dut.out
                        break
            data_bytes = data.view('uint8')
            for j in range(re.size):
                b = data_bytes[3*j:3*(j+1)]
                r = re[j]
                i = im[j]
                mask = 2**12 - 1
                self.assertEqual(r & mask, (b[0] << 4) | (b[1] >> 4))
                self.assertEqual(i & mask, (b[1] & 0xf) << 8 | b[2])

        self.simulate([set_input, check_output])


class TestPack8IQto32(AmaranthSim):
    def test_pack(self):
        nsamples = 4096
        re = np.random.randint(-2**7, 2**7, size=nsamples)
        im = np.random.randint(-2**7, 2**7, size=nsamples)
        self.dut = Pack8IQto32()

        def set_input():
            yield self.dut.enable.eq(1)
            for r, i in zip(re, im):
                while True:
                    yield self.dut.re_in.eq(int(r))
                    yield self.dut.im_in.eq(int(i))
                    strobe = int(np.random.randint(2))
                    yield self.dut.strobe_in.eq(strobe)
                    yield
                    if strobe:
                        break

        def check_output():
            data = np.zeros(nsamples // 2, 'uint32')
            for j in range(data.size):
                while True:
                    yield
                    if (yield self.dut.strobe_out):
                        data[j] = yield self.dut.out
                        break
            data_samples = data.view('int8')
            np.testing.assert_equal(re, data_samples[::2])
            np.testing.assert_equal(im, data_samples[1::2])

        self.simulate([set_input, check_output])


class TestPackFifoTwice(AmaranthSim):
    def test_pack(self):
        nsamples = 4096
        x = np.random.randint(-2**31, 2**31, size=nsamples)
        self.dut = PackFifoTwice()

        def set_input():
            yield self.dut.enable.eq(1)
            for a in x:
                yield self.dut.empty.eq(1)
                while True:
                    if np.random.randint(2):
                        break
                    yield
                yield self.dut.empty.eq(0)
                while True:
                    yield
                    if (yield self.dut.rden):
                        break
                yield self.dut.fifo_data.eq(int(a))
            yield self.dut.empty.eq(1)

        def check_output():
            data = np.zeros(nsamples // 2, 'uint64')
            for j in range(data.size):
                yield self.dut.out_ready.eq(0)
                while True:
                    if np.random.randint(2):
                        break
                    yield
                yield self.dut.out_ready.eq(1)
                while True:
                    yield
                    if (yield self.dut.out_valid):
                        break
                data[j] = yield self.dut.out_data
            yield
            yield
            yield self.dut.enable.eq(0)
            data_samples = data.view('int32')
            np.testing.assert_equal(x, data_samples)

        def check_rderr():
            while True:
                yield
                if (yield self.dut.enable):
                    break
            while True:
                yield
                if not (yield self.dut.enable):
                    break
                rden = yield self.dut.rden
                empty = yield self.dut.empty
                assert not empty or not rden

        self.simulate([set_input, check_output, check_rderr])


if __name__ == '__main__':
    unittest.main()
