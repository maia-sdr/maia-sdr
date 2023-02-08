#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import random


class RandomReady:
    "Random ready"
    def __init__(self, max_on=32, max_off=32):
        self.max_on = max_on
        self.max_off = max_off
        self.__qualname__ = 'RandomReady'
        self.__doc__ = f'RandomReady({max_on}, {max_off})'

    def __call__(self):
        while True:
            yield (random.randint(1, self.max_on),
                   random.randint(1, self.max_off))
