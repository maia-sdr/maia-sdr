#
# Copyright (C) 2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

class MaiaSDRConfig:
    """Maia SDR configuration

    This class defines configuration parameters for the Maia SDR top-level.
    """
    def __init__(self):
        # create default configuration

        # general
        self.platform = 0

        # spectrometer
        self.spectrometer_address = 0x1a00_0000
        self.spectrometer_buffers = 8

        # IQ recorder
        self.recorder_address_range = (0x0100_0000, 0x1a00_0000)

    def validate(self):
        assert self.platform >= 0 and self.platform < 256
        assert self.spectrometer_buffers > 0
        assert self.spectrometer_buffers.bit_count() == 1
        assert self.recorder_address_range[0] < self.recorder_address_range[1]
        # TODO: check that spectrometer and recorder buffers do not overlap
