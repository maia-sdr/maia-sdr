#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import array


class Memory:
    def __init__(self, size):
        self._data = array.array('B', [0] * size)

    @property
    def _len(self):
        return len(self._data)

    def __getitem__(self, key):
        if isinstance(key, int):
            return self._data[key % self._len]
        if isinstance(key, slice):
            return self._data[
                key.start % self._len:key.stop % self._len]
        raise ValueError('unsupported key')

    def __setitem__(self, key, value):
        if isinstance(key, int):
            self._data[key % self._len] = value
            return
        if isinstance(key, slice):
            a = key.start % self._len
            self._data[
                a:(key.stop - key.start) + a] = value
            return
        raise ValueError('unsupported key')
