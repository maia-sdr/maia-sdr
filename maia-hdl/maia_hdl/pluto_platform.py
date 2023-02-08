#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth.vendor.xilinx import XilinxPlatform


class PlutoPlatform(XilinxPlatform):
    device = "xc7z010"
    package = "clg400"
    speed = "1"
    resources = []
    connectors = []
