#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
from amaranth.lib.cdc import FFSynchronizer, PulseSynchronizer
import amaranth.back.verilog

from .axi4_lite import Axi4LiteRegisterBridge
from .cdc import RegisterCDC, RxIQCDC
from .clknx import ClkNxCommonEdge
from .pulse import PulseStretcher
from .pluto_platform import PlutoPlatform
from .register import Access, Field, Registers, Register, RegisterMap
from .recorder import Recorder12IQ
from .spectrometer import Spectrometer

# IP core version
_version = '0.4.0'


class MaiaSDR(Elaboratable):
    """Maia-SDR top level

    This elaboratable is the top-level Maia SDR IP core.
    """
    def __init__(self):
        self.axi4_awidth = 4
        self.s_axi_lite = ClockDomain()
        self.sampling = ClockDomain()
        self.clk2x = ClockDomain()
        self.clk3x = ClockDomain()

        self.axi4lite = Axi4LiteRegisterBridge(
            self.axi4_awidth, name='s_axi_lite')
        self.control_registers = Registers(
            'control',
            {
                0b00: Register(
                    'product_id', [
                        Field('product_id', Access.R, 32, 0x6169616d)
                    ]),
                0b01: Register('version', [
                    Field('bugfix', Access.R, 8,
                          int(_version.split('.')[2])),
                    Field('minor', Access.R, 8,
                          int(_version.split('.')[1])),
                    Field('major', Access.R, 8,
                          int(_version.split('.')[0])),
                    Field('platform', Access.R, 8, 0),
                ]),
                0b10: Register('control', [
                    Field('sdr_reset', Access.RW, 1, 1),
                ]),
                0b11: Register('interrupts', [
                    Field('spectrometer', Access.Rsticky, 1, 0),
                    Field('recorder', Access.Rsticky, 1, 0),
                ], interrupt=True),
            },
            2)
        self.recorder_registers = Registers(
            'recorder',
            {
                0b0: Register('recorder_control', [
                    Field('start', Access.Wpulse, 1, 0),
                    Field('stop', Access.Wpulse, 1, 0),
                    Field('mode_8bit', Access.RW, 1, 0),
                    Field('dropped_samples', Access.R, 1, 0),
                ]),
                0b1: Register('recorder_next_address', [
                    Field('next_address', Access.R, 32, 0),
                ]),
            },
            1)
        self.spectrometer = Spectrometer(
            0x1a00_0000, 3, dma_name='m_axi_spectrometer')
        self.recorder = Recorder12IQ(
            0x0100_0000, 0x1a00_0000, dma_name='m_axi_recorder',
            domain_in='sampling', domain_dma='s_axi_lite')
        self.sdr_registers = Registers(
            'sdr', {
                0b0: Register(
                    'spectrometer',
                    [
                        Field('num_integrations',
                              Access.RW,
                              self.spectrometer.nint_width,
                              -1),
                        Field('last_buffer',
                              Access.R,
                              len(self.spectrometer.last_buffer),
                              0),
                        Field('peak_detect',
                              Access.RW,
                              1,
                              0),
                    ]),
            }, 1)
        metadata = {
            'vendor': 'Daniel Estevez',
            'vendorID': 'destevez.net',
            'name': 'Maia SDR',
            'series': 'Maia SDR',
            'version': _version,
            'description': 'Maia SDR IP core',
            'licenseText': ('SPDX-License-Identifier: MIT '
                            'Copyright (C) Daniel Estevez 2022-2023'),
        }
        self.register_map = RegisterMap({
            0x0: self.control_registers,
            0x10: self.recorder_registers,
            0x20: self.sdr_registers,
        }, metadata)

        self.re_in = Signal(self.spectrometer.width_in)
        self.im_in = Signal(self.spectrometer.width_in)
        self.interrupt_out = Signal()

    def ports(self):
        return (
            self.axi4lite.axi.ports()
            + self.spectrometer.dma.axi.ports()
            + self.recorder.dma.axi.ports()
            + [
                self.re_in,
                self.im_in,
                self.interrupt_out,
                self.s_axi_lite.clk,
                self.s_axi_lite.rst,
                self.sampling.clk,
                self.clk2x.clk,
                self.clk3x.clk,
            ]
        )

    def svd(self):
        return self.register_map.svd()

    def elaborate(self, platform):
        m = Module()
        m.domains += [
            self.s_axi_lite,
            self.sampling,
            self.clk2x,
            self.clk3x,
        ]
        s_axi_lite_renamer = DomainRenamer({'sync': 's_axi_lite'})
        m.submodules.axi4lite = s_axi_lite_renamer(self.axi4lite)
        m.submodules.control_registers = s_axi_lite_renamer(
            self.control_registers)
        m.submodules.recorder_registers = s_axi_lite_renamer(
            self.recorder_registers)
        m.submodules.spectrometer = self.spectrometer
        m.submodules.sync_spectrometer_interrupt = \
            sync_spectrometer_interrupt = PulseSynchronizer(
                i_domain='sync', o_domain='s_axi_lite')
        m.submodules.recorder = self.recorder
        m.submodules.sdr_registers = self.sdr_registers
        m.submodules.sdr_registers_cdc = sdr_registers_cdc = RegisterCDC(
            's_axi_lite', 'sync', self.sdr_registers.aw)

        m.submodules.common_edge_2x = common_edge_2x = ClkNxCommonEdge(
            'sync', 'clk2x', 2)
        m.submodules.common_edge_3x = common_edge_3x = ClkNxCommonEdge(
            'sync', 'clk3x', 3)

        # RX IQ CDC
        m.submodules.rxiq_cdc = rxiq_cdc = RxIQCDC('sampling', 'sync', 12)
        m.d.comb += [rxiq_cdc.re_in.eq(self.re_in),
                     rxiq_cdc.im_in.eq(self.im_in)]

        # Spectrometer (sync domain)
        m.d.comb += [
            self.spectrometer.strobe_in.eq(rxiq_cdc.strobe_out),
            self.spectrometer.common_edge_2x.eq(common_edge_2x.common_edge),
            self.spectrometer.common_edge_3x.eq(common_edge_3x.common_edge),
            self.spectrometer.re_in.eq(rxiq_cdc.re_out),
            self.spectrometer.im_in.eq(rxiq_cdc.im_out),
            sync_spectrometer_interrupt.i.eq(self.spectrometer.interrupt_out),
            self.spectrometer.number_integrations.eq(
                self.sdr_registers['spectrometer']['num_integrations']),
            self.spectrometer.peak_detect.eq(
                self.sdr_registers['spectrometer']['peak_detect']),
            self.sdr_registers['spectrometer']['last_buffer'].eq(
                self.spectrometer.last_buffer),
        ]

        # Recorder
        m.d.comb += [
            # sampling domain
            self.recorder.strobe_in.eq(Const(1)),
            self.recorder.re_in.eq(self.re_in),
            self.recorder.im_in.eq(self.im_in),
            # s_axi_lite domain
            self.recorder.mode_8bit.eq(
                self.recorder_registers['recorder_control']['mode_8bit']),
            self.recorder.start.eq(
                self.recorder_registers['recorder_control']['start']),
            self.recorder.stop.eq(
                self.recorder_registers['recorder_control']['stop']),
            self.recorder_registers['recorder_control']['dropped_samples'].eq(
                self.recorder.dropped_samples),
            (self.recorder_registers['recorder_next_address']
             ['next_address'].eq(self.recorder.next_address)),
        ]

        # Registers s_axi_lite domain
        # TODO: convert all of this into a RegisterCrossbar module
        address = Signal(self.axi4_awidth, reset_less=True)
        wdata = Signal(32, reset_less=True)
        sdr_regs_select = self.axi4lite.address[3] == 1
        recorder_regs_select = (
            ~sdr_regs_select & (self.axi4lite.address[2] == 1))
        control_regs_select = (
            ~sdr_regs_select & (self.axi4lite.address[2] == 0))
        m.d.s_axi_lite += [
            self.axi4lite.rdata.eq(self.control_registers.rdata
                                   | self.recorder_registers.rdata
                                   | sdr_registers_cdc.i_rdata),
            self.axi4lite.rdone.eq(self.control_registers.rdone
                                   | self.recorder_registers.rdone
                                   | sdr_registers_cdc.i_rdone),
            self.axi4lite.wdone.eq(self.control_registers.wdone
                                   | self.recorder_registers.wdone
                                   | sdr_registers_cdc.i_wdone),
            self.control_registers.ren.eq(
                self.axi4lite.ren & control_regs_select),
            self.control_registers.wstrobe.eq(
                Mux(control_regs_select, self.axi4lite.wstrobe, 0)),
            self.recorder_registers.ren.eq(
                self.axi4lite.ren & recorder_regs_select),
            self.recorder_registers.wstrobe.eq(
                Mux(recorder_regs_select, self.axi4lite.wstrobe, 0)),
            sdr_registers_cdc.i_ren.eq(
                self.axi4lite.ren & sdr_regs_select),
            sdr_registers_cdc.i_wstrobe.eq(
                Mux(sdr_regs_select, self.axi4lite.wstrobe, 0)),
            address.eq(self.axi4lite.address),
            wdata.eq(self.axi4lite.wdata),
        ]
        m.d.comb += [
            self.control_registers.address.eq(address),
            self.control_registers.wdata.eq(wdata),
            self.recorder_registers.address.eq(address),
            self.recorder_registers.wdata.eq(wdata),
            sdr_registers_cdc.i_address.eq(address),
            sdr_registers_cdc.i_wdata.eq(wdata),
        ]

        # Registers sync domain
        m.d.comb += [
            self.sdr_registers.ren.eq(sdr_registers_cdc.o_ren),
            self.sdr_registers.wstrobe.eq(sdr_registers_cdc.o_wstrobe),
            self.sdr_registers.address.eq(sdr_registers_cdc.o_address),
            self.sdr_registers.wdata.eq(sdr_registers_cdc.o_wdata),
            sdr_registers_cdc.o_rdone.eq(self.sdr_registers.rdone),
            sdr_registers_cdc.o_wdone.eq(self.sdr_registers.wdone),
            sdr_registers_cdc.o_rdata.eq(self.sdr_registers.rdata),
        ]
        # internal resets
        # We use FFSynchronizer rather than ResetSynchronizer because of
        # https://github.com/amaranth-lang/amaranth/issues/721
        for internal in ['sync', 'clk2x', 'clk3x', 'sampling']:
            setattr(m.submodules, f'{internal}_rst', FFSynchronizer(
                self.control_registers['control']['sdr_reset'],
                ResetSignal(internal), o_domain=internal,
                reset=1))
        m.d.comb += rxiq_cdc.reset.eq(
            self.control_registers['control']['sdr_reset'])

        # Interrupts (s_axi_lite domain)
        interrupts_reg = self.control_registers['interrupts']
        m.d.comb += [
            self.interrupt_out.eq(interrupts_reg.interrupt),
            interrupts_reg['spectrometer'].eq(sync_spectrometer_interrupt.o),
            interrupts_reg['recorder'].eq(self.recorder.finished),
        ]

        return m


def write_svd(path):
    top = MaiaSDR()
    with open(path, 'wb') as f:
        f.write(top.svd())


if __name__ == '__main__':
    top = MaiaSDR()
    platform = PlutoPlatform()
    amaranth.cli.main(
        top, platform=platform, ports=top.ports())
