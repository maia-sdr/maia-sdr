#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from .config import MaiaSDRConfig


def default():
    """Default Maia SDR configuration"""
    return MaiaSDRConfig()


def maia_iio():
    """Configuration for Maia SDR + IIO"""
    config = MaiaSDRConfig()
    config.spectrometer_address = 0x1600_0000
    config.recorder_address_range = (0x0600_0000, 0x1600_0000)
    return config
