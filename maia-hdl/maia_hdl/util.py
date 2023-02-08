#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import numpy as np


def clamp_nbits(x, nbits):
    offset = 2**(nbits - 1)
    return ((x + offset) % 2**nbits) - offset


def bit_invert(n, nbits, radix_log2):
    bits = ('0'*nbits + bin(n)[2:])[-nbits:]
    bits_arr = np.array([a for a in bits])
    inverted = bits_arr.reshape(-1, radix_log2)[::-1].ravel()
    inverted_str = ''.join(list(inverted))
    return int(inverted_str, 2)
