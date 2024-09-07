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

from maia_hdl.packer import (
    Pack16IQto32, Pack12IQto32, Pack8IQto32, PackFifoTwice)
from .amaranth_sim import AmaranthSim


class TestPack16IQto32(AmaranthSim):
    def test_pack(self):
        nsamples = 4096
        re = np.random.randint(-2**15, 2**15, size=nsamples)
        im = np.random.randint(-2**15, 2**15, size=nsamples)
        dut = Pack16IQto32()
        # we need a sequential element in the simulation
        m = Module()
        m.submodules.dut = dut
        ignore = Signal()
        m.d.sync += ignore.eq(~ignore)
        self.dut = m

        async def set_input(ctx):
            ctx.set(dut.enable, 1)
            for r, i in zip(re, im):
                while True:
                    await ctx.tick()
                    ctx.set(dut.re_in, int(r))
                    ctx.set(dut.im_in, int(i))
                    strobe = int(np.random.randint(2))
                    ctx.set(dut.strobe_in, strobe)
                    if strobe:
                        break

        async def check_output(ctx):
            data = np.zeros(nsamples, 'uint32')
            for j in range(data.size):
                while True:
                    await ctx.tick()
                    if ctx.get(dut.strobe_out):
                        data[j] = ctx.get(dut.out)
                        break
            data = data.view('int16')
            for j in range(re.size):
                self.assertEqual(re[j], data[2 * j])
                self.assertEqual(im[j], data[2 * j + 1])

        self.simulate([set_input, check_output])


class TestPack12IQto32(AmaranthSim):
    def test_pack(self):
        nsamples = 4096
        re = np.random.randint(-2**11, 2**11, size=nsamples)
        im = np.random.randint(-2**11, 2**11, size=nsamples)
        self.dut = Pack12IQto32()

        async def set_input(ctx):
            ctx.set(self.dut.enable, 1)
            for r, i in zip(re, im):
                while True:
                    await ctx.tick()
                    ctx.set(self.dut.re_in, int(r))
                    ctx.set(self.dut.im_in, int(i))
                    strobe = int(np.random.randint(2))
                    ctx.set(self.dut.strobe_in, strobe)
                    if strobe:
                        break

        async def check_output(ctx):
            data = np.zeros(nsamples // 4 * 3, 'uint32')
            for j in range(data.size):
                while True:
                    await ctx.tick()
                    if ctx.get(self.dut.strobe_out):
                        data[j] = ctx.get(self.dut.out)
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

        async def set_input(ctx):
            ctx.set(self.dut.enable, 1)
            for r, i in zip(re, im):
                while True:
                    await ctx.tick()
                    ctx.set(self.dut.re_in, int(r))
                    ctx.set(self.dut.im_in, int(i))
                    strobe = int(np.random.randint(2))
                    ctx.set(self.dut.strobe_in, strobe)
                    if strobe:
                        break

        async def check_output(ctx):
            data = np.zeros(nsamples // 2, 'uint32')
            for j in range(data.size):
                while True:
                    await ctx.tick()
                    if ctx.get(self.dut.strobe_out):
                        data[j] = ctx.get(self.dut.out)
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

        async def set_input(ctx):
            ctx.set(self.dut.enable, 1)
            for a in x:
                ctx.set(self.dut.empty, 1)
                while True:
                    if np.random.randint(2):
                        break
                    await ctx.tick()
                ctx.set(self.dut.empty, 0)
                while True:
                    await ctx.tick()
                    if ctx.get(self.dut.rden):
                        break
                ctx.set(self.dut.fifo_data, int(a))
            ctx.set(self.dut.empty, 1)

        async def check_output(ctx):
            data = np.zeros(nsamples // 2, 'uint64')
            for j in range(data.size):
                ctx.set(self.dut.out_ready, 0)
                while True:
                    if np.random.randint(2):
                        break
                    await ctx.tick()
                ctx.set(self.dut.out_ready, 1)
                while True:
                    await ctx.tick()
                    if ctx.get(self.dut.out_valid):
                        break
                data[j] = ctx.get(self.dut.out_data)
            await ctx.tick().repeat(2)
            ctx.set(self.dut.enable, 0)
            data_samples = data.view('int32')
            np.testing.assert_equal(x, data_samples)

        async def check_rderr(ctx):
            while True:
                await ctx.tick()
                if ctx.get(self.dut.enable):
                    break
            while True:
                await ctx.tick()
                if not ctx.get(self.dut.enable):
                    break
                rden = ctx.get(self.dut.rden)
                empty = ctx.get(self.dut.empty)
                assert not empty or not rden

        self.simulate([set_input, check_output, check_rderr])


if __name__ == '__main__':
    unittest.main()
