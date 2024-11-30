#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

import argparse

from amaranth import *
from amaranth.lib.cdc import FFSynchronizer, PulseSynchronizer
import amaranth.back.verilog

from .axi4_lite import Axi4LiteRegisterBridge
from .cdc import RegisterCDC, RxIQCDC
from .clknx import ClkNxCommonEdge
from .config import MaiaSDRConfig
from . import configs
from .ddc import DDC
from .pulse import PulseStretcher
from .pluto_platform import PlutoPlatform
from .register import Access, Field, Registers, Register, RegisterMap
from .recorder import Recorder16IQ, RecorderMode
from .spectrometer import Spectrometer

# IP core version
_version = '0.6.1'


class MaiaSDR(Elaboratable):
    """Maia SDR top level

    This elaboratable is the top-level Maia SDR IP core.
    """
    def __init__(self, config=MaiaSDRConfig()):
        config.validate()
        self.config = config
        self.axi4_awidth = 4
        self.s_axi_lite = ClockDomain()
        self.sampling = ClockDomain()
        # A clock domain called 'sync' is added to override the default
        # behaviour, since we drive the reset internally.
        #
        # See https://github.com/amaranth-lang/amaranth/issues/1506
        self.sync = ClockDomain()
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
                    Field('platform', Access.R, 8, config.platform),
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
                    Field('mode', Access.RW,
                          Shape.cast(RecorderMode).width, 0),
                    Field('dropped_samples', Access.R, 1, 0),
                ]),
                0b1: Register('recorder_next_address', [
                    Field('next_address', Access.R, 32, 0),
                ]),
            },
            1)
        self.spectrometer = Spectrometer(
            config.spectrometer_address,
            config.spectrometer_buffers.bit_length() - 1,
            dma_name='m_axi_spectrometer')
        self.recorder = Recorder16IQ(
            config.recorder_address_range[0],
            config.recorder_address_range[1],
            dma_name='m_axi_recorder', domain_in='sync',
            domain_dma='s_axi_lite')
        self.ddc = DDC('clk3x')
        self.sdr_registers = Registers(
            'sdr', {
                0b000: Register(
                    'spectrometer',
                    [
                        Field('use_ddc_out',
                              Access.RW,
                              1,
                              0),
                        Field('num_integrations',
                              Access.RW,
                              self.spectrometer.nint_width,
                              -1),
                        Field('abort', Access.Wpulse, 1, 0),
                        Field('last_buffer',
                              Access.R,
                              len(self.spectrometer.last_buffer),
                              0),
                        Field('peak_detect',
                              Access.RW,
                              1,
                              0),
                    ]),
                0b001: Register(
                    'ddc_coeff_addr',
                    [
                        Field('coeff_waddr',
                              Access.RW,
                              10,
                              0),
                    ]),
                0b010: Register(
                    'ddc_coeff',
                    [
                        Field('coeff_wren',
                              Access.Wpulse,
                              1,
                              0),
                        Field('coeff_wdata',
                              Access.RW,
                              18,
                              0),
                    ]),
                0b011: Register(
                    'ddc_decimation',
                    [
                        Field('decimation1',
                              Access.RW,
                              7,
                              0),
                        Field('decimation2',
                              Access.RW,
                              6,
                              0),
                        Field('decimation3',
                              Access.RW,
                              7,
                              0),
                    ]),
                0b100: Register(
                    'ddc_frequency',
                    [
                        Field('frequency',
                              Access.RW,
                              28,
                              0),
                    ]),
                0b101: Register(
                    'ddc_control',
                    [
                        Field('operations_minus_one1',
                              Access.RW,
                              7,
                              0),
                        Field('operations_minus_one2',
                              Access.RW,
                              6,
                              0),
                        Field('operations_minus_one3',
                              Access.RW,
                              7,
                              0),
                        Field('odd_operations1',
                              Access.RW,
                              1,
                              0),
                        Field('odd_operations3',
                              Access.RW,
                              1,
                              0),
                        Field('bypass2',
                              Access.RW,
                              1,
                              0),
                        Field('bypass3',
                              Access.RW,
                              1,
                              0),
                        Field('enable_input',
                              Access.RW,
                              1,
                              0),
                    ]),
            }, 3)
        metadata = {
            'vendor': 'Daniel Estevez',
            'vendorID': 'destevez.net',
            'name': 'Maia SDR',
            'series': 'Maia SDR',
            'version': _version,
            'description': f'Maia SDR IP core (platform {config.platform})',
            'licenseText': ('SPDX-License-Identifier: MIT '
                            'Copyright (C) Daniel Estevez 2022-2024'),
        }
        self.register_map = RegisterMap({
            0x0: self.control_registers,
            0x10: self.recorder_registers,
            0x20: self.sdr_registers,
        }, metadata)

        self.iq_in_width = 12
        self.re_in = Signal(self.iq_in_width)
        self.im_in = Signal(self.iq_in_width)
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
                self.sync.clk,
                self.sync.rst,
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
            self.sync,
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
        m.submodules.ddc = self.ddc
        m.submodules.sdr_registers = self.sdr_registers
        m.submodules.sdr_registers_cdc = sdr_registers_cdc = RegisterCDC(
            's_axi_lite', 'sync', self.sdr_registers.aw)

        m.submodules.common_edge_2x = common_edge_2x = ClkNxCommonEdge(
            'sync', 'clk2x', 2)
        m.submodules.common_edge_3x = common_edge_3x = ClkNxCommonEdge(
            'sync', 'clk3x', 3)

        # RX IQ CDC
        m.submodules.rxiq_cdc = rxiq_cdc = RxIQCDC(
            'sampling', 'sync', self.iq_in_width)
        m.d.comb += [rxiq_cdc.re_in.eq(self.re_in),
                     rxiq_cdc.im_in.eq(self.im_in)]

        # Spectrometer (sync domain)
        spectrometer_re_in = Signal(
            self.spectrometer.width_in, reset_less=True)
        spectrometer_im_in = Signal(
            self.spectrometer.width_in, reset_less=True)
        assert len(spectrometer_re_in) == len(self.ddc.re_out)
        assert len(spectrometer_im_in) == len(self.ddc.im_out)
        spectrometer_strobe_in = Signal()
        with m.If(self.sdr_registers['spectrometer']['use_ddc_out']):
            m.d.sync += [
                spectrometer_re_in.eq(self.ddc.re_out),
                spectrometer_im_in.eq(self.ddc.im_out),
                spectrometer_strobe_in.eq(self.ddc.strobe_out),
            ]
        with m.Else():
            shift = self.spectrometer.width_in - self.iq_in_width
            m.d.sync += [
                # The RX IQ samples have 12 bits, but the spectrometer input
                # has 16 bits. Push the 12 bits to the MSBs.
                spectrometer_re_in.eq(rxiq_cdc.re_out << shift),
                spectrometer_im_in.eq(rxiq_cdc.im_out << shift),
                spectrometer_strobe_in.eq(rxiq_cdc.strobe_out),
            ]
        m.d.comb += [
            self.spectrometer.strobe_in.eq(spectrometer_strobe_in),
            self.spectrometer.common_edge_2x.eq(common_edge_2x.common_edge),
            self.spectrometer.common_edge_3x.eq(common_edge_3x.common_edge),
            self.spectrometer.re_in.eq(spectrometer_re_in),
            self.spectrometer.im_in.eq(spectrometer_im_in),
            sync_spectrometer_interrupt.i.eq(self.spectrometer.interrupt_out),
            self.spectrometer.number_integrations.eq(
                self.sdr_registers['spectrometer']['num_integrations']),
            self.spectrometer.abort.eq(
                self.sdr_registers['spectrometer']['abort']),
            self.spectrometer.peak_detect.eq(
                self.sdr_registers['spectrometer']['peak_detect']),
            self.sdr_registers['spectrometer']['last_buffer'].eq(
                self.spectrometer.last_buffer),
        ]

        # Recorder
        m.d.comb += [
            # sync domain
            self.recorder.strobe_in.eq(spectrometer_strobe_in),
            self.recorder.re_in.eq(spectrometer_re_in),
            self.recorder.im_in.eq(spectrometer_im_in),
            # s_axi_lite domain
            self.recorder.mode.eq(
                self.recorder_registers['recorder_control']['mode']),
            self.recorder.start.eq(
                self.recorder_registers['recorder_control']['start']),
            self.recorder.stop.eq(
                self.recorder_registers['recorder_control']['stop']),
            self.recorder_registers['recorder_control']['dropped_samples'].eq(
                self.recorder.dropped_samples),
            (self.recorder_registers['recorder_next_address']
             ['next_address'].eq(self.recorder.next_address)),
        ]

        # DDC
        m.d.comb += [
            self.ddc.common_edge.eq(common_edge_3x.common_edge),
            self.ddc.enable_input.eq(
                self.sdr_registers['ddc_control']['enable_input']),
            self.ddc.frequency.eq(
                self.sdr_registers['ddc_frequency']['frequency']),
            self.ddc.coeff_waddr.eq(
                self.sdr_registers['ddc_coeff_addr']['coeff_waddr']),
            self.ddc.coeff_wren.eq(
                self.sdr_registers['ddc_coeff']['coeff_wren']),
            self.ddc.coeff_wdata.eq(
                self.sdr_registers['ddc_coeff']['coeff_wdata']),
            self.ddc.decimation1.eq(
                self.sdr_registers['ddc_decimation']['decimation1']),
            self.ddc.decimation2.eq(
                self.sdr_registers['ddc_decimation']['decimation2']),
            self.ddc.decimation3.eq(
                self.sdr_registers['ddc_decimation']['decimation3']),
            self.ddc.bypass2.eq(
                self.sdr_registers['ddc_control']['bypass2']),
            self.ddc.bypass3.eq(
                self.sdr_registers['ddc_control']['bypass3']),
            self.ddc.operations_minus_one1.eq(
                self.sdr_registers['ddc_control']['operations_minus_one1']),
            self.ddc.operations_minus_one2.eq(
                self.sdr_registers['ddc_control']['operations_minus_one2']),
            self.ddc.operations_minus_one3.eq(
                self.sdr_registers['ddc_control']['operations_minus_one3']),
            self.ddc.odd_operations1.eq(
                self.sdr_registers['ddc_control']['odd_operations1']),
            self.ddc.odd_operations3.eq(
                self.sdr_registers['ddc_control']['odd_operations3']),
            self.ddc.strobe_in.eq(rxiq_cdc.strobe_out),
            self.ddc.re_in.eq(rxiq_cdc.re_out),
            self.ddc.im_in.eq(rxiq_cdc.im_out),
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
                init=1))
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


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        '--config', default='default',
        help='Maia SDR configuration name [default=%(default)r]')
    parser.add_argument(
        'output_file', help='Output verilog file')
    return parser.parse_args()


def main():
    args = parse_args()
    config = getattr(configs, args.config)()
    top = MaiaSDR(config)
    platform = PlutoPlatform()
    with open(args.output_file, 'w') as f:
        f.write(amaranth.back.verilog.convert(
            top, platform=platform, ports=top.ports()))


if __name__ == '__main__':
    main()
