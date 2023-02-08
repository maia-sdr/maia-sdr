#
# Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.back.verilog

import collections
import enum
from typing import Dict, List
import xml.etree.ElementTree as ET


Field = collections.namedtuple('RegisterField',
                               ['name', 'access', 'width', 'reset'])


class Access(enum.Enum):
    R = enum.auto()
    RW = enum.auto()
    W = enum.auto()
    Wpulse = enum.auto()
    Rsticky = enum.auto()


class Register(Elaboratable):
    """Register

    This implements a register that contains some fields and can be accessed
    through a custom bus.

    The custom bus works as follows. When the ``ren`` input is asserted, the
    ``rdata`` output should contain the read data for the register. Otherwise,
    ``rdata`` should contain zero. The value of ``rdata`` is updated
    combinationally depending on ``ren``. At each rising edge of the clock, the
    value of ``wdata`` should be written into the register for those bytes for
    which the corresponding bit in ``wstrobe`` is asserted. When no write is
    desired, all the bits of ``wstrobe`` should be deasserted.

    Register fields are accessed by using ``__getitem__``, as if the
    ``Register`` was a dictionary. The field name is used as key. The fields
    accessed in this way are Signal()'s of the appropriate width. Depending on
    the ``Access`` mode of the field, these signals are used as input (``R``
    and ``Rsticky``) or output (``RW``, ``W``, and ``Wpulse``).

    The register supports interrupt generation. If interrupt generation is
    enabled, the register will have an interrupt output pin that is asserted
    whenever any ``Rsticky`` field is non-zero.

    Several ``Register``'s are collected together in a ``Registers``
    elaboratable, which provides access to them by address.

    Parameters
    ----------
    name : str
        Register name. This is used in the ``Registers`` to access the register
        and to generate the SVD.
    fields : List[Field]
        List of fields stored in the register.
    width : int
        Data width of the register.
    interrupt : bool
        Interrupt support. When this flag is enabled, the register has an
        interrupt output.

    Attributes
    ----------
    ren : Signal(), in
        Read enable.
    wstrobe : Signal(width // 8), in
        Write strobe.
    rdata : Signal(width), out
        Read data.
    wdata : Signal(width), in
        Write data.
    interrupt : Signal(), out
        Interrupt output. Only present when the interrupt parameter is True.
    """
    def __init__(self, name: str, fields: List[Field], width: int = 32,
                 interrupt: bool = False):
        self.name = name
        self.w = width
        self.fields = fields
        self.nstrobes = width // 8

        self.ren = Signal()
        self.wstrobe = Signal(self.nstrobes)
        self.rdata = Signal(width, reset=0)
        self.wdata = Signal(width)
        if interrupt:
            self.interrupt = Signal()
        for field in fields:
            sig = Signal(field.width,
                         name=self._sig_name(field.name),
                         reset=field.reset)
            setattr(self, self._sig_name(field.name), sig)
            if field.access == Access.Rsticky:
                sig = Signal(field.width,
                             name=self._sig_name_sticky(field.name),
                             reset=field.reset)
                setattr(self, self._sig_name_sticky(field.name), sig)

    def __getitem__(self, name: str) -> Signal:
        return getattr(self, self._sig_name(name))

    def _sig_name(self, name: str) -> str:
        return f'field_{name}'

    def _sig_name_sticky(self, name: str) -> str:
        return f'field_sticky_{name}'

    def elaborate(self, platform):
        m = Module()
        offset = 0
        # Fake last field, to prevent amaranth/yosis from creating
        # an assign to the slice of rdata that we haven't touched.
        # See https://github.com/amaranth-lang/amaranth/issues/717
        # This is overwritten by the appropriate assignments in
        # with m.If(self.ren) blocks below.
        with m.If(self.ren):
            m.d.comb += self.rdata.eq(0)
        for field in self.fields:
            if field.access == Access.Rsticky:
                sticky = getattr(self, self._sig_name_sticky(field.name))
                m.d.sync += sticky.eq(sticky | self[field.name])
            if field.access in [Access.R, Access.RW, Access.Rsticky]:
                rfield = (
                    self[field.name] if field.access != Access.Rsticky
                    else getattr(self, self._sig_name_sticky(field.name)))
                with m.If(self.ren):
                    m.d.comb += (
                        self.rdata[offset:][:field.width].eq(rfield))
                    if field.access == Access.Rsticky:
                        m.d.sync += rfield.eq(self[field.name])
            if field.access == Access.Wpulse:
                m.d.sync += self[field.name].eq(0)
            if field.access in [Access.RW, Access.W, Access.Wpulse]:
                for j in range(field.width):
                    k = offset + j
                    strobe = self.wstrobe[k // 8]
                    with m.If(strobe):
                        m.d.sync += self[field.name][j].eq(self.wdata[k])
            offset += field.width
            if offset > self.w:
                raise ValueError('fields are too wide for register')
        if hasattr(self, 'interrupt'):
            m.d.sync += self.interrupt.eq(
                Cat(*[getattr(self, self._sig_name_sticky(field.name))
                      for field in self.fields
                      if field.access == Access.Rsticky]).any())
        return m

    def svd(self):
        register = ET.Element('register')
        name = ET.SubElement(register, 'name')
        name.text = self.name
        description = ET.SubElement(register, 'description')
        # TODO: include something more meaningful as description
        description.text = self.name
        address_offset = ET.SubElement(register, 'addressOffset')
        # We do not set the address offset text by now, since we
        # don't know it yet.
        has_read = any([f.access in [Access.R, Access.RW, Access.Rsticky]
                        for f in self.fields])
        has_write = any([f.access in [Access.W, Access.RW, Access.Wpulse]
                         for f in self.fields])
        access_value = {(True, True): 'read-write',
                        (True, False): 'read-only',
                        (False, True): 'write-only'}[(has_read, has_write)]
        access = ET.SubElement(register, 'access')
        access.text = access_value
        fields = ET.SubElement(register, 'fields')
        offset = 0
        for field in self.fields:
            f = ET.SubElement(fields, 'field')
            fname = ET.SubElement(f, 'name')
            fname.text = field.name
            fdescr = ET.SubElement(f, 'description')
            # TODO: include something more meaningful as description
            fdescr.text = field.name
            bitrange = ET.SubElement(f, 'bitRange')
            w = field.width
            bitrange.text = f'[{offset+w-1}:{offset}]'
            offset += w
            faccess = ET.SubElement(f, 'access')
            faccess.text = {
                Access.RW: 'read-write',
                Access.R: 'read-only',
                Access.W: 'write-only',
                Access.Wpulse: 'write-only',
                Access.Rsticky: 'read-only',
            }[field.access]
        # TODO: add reset value and reset mask
        return register


class Registers(Elaboratable):
    """Register bank

    A register bank collects several registers and gives access to
    them by address. Registers in the bank can be accessed by using
    ``__getitem__``, as if the ``Registers`` was a dictionary. The
    name of the register is used as key.

    The register bank refines the bus protocol used for ``Register``,
    in order to add addressing and support for multi-cycle delays in
    the transactions. When ``ren`` is pulsed, the register corresponding
    to the address in ``address`` will be accessed for read. Some
    cycles later, ``rdone`` will be pulsed and the read data will appear
    in ``rdata`` simultaneously. When some bits in ``wstrobe`` are pulsed,
    the register corresponding to the address in ``address`` will be
    accessed for write. Some cycles later, ``wdone`` will be pulsed,
    indicating that the write has finished. After pulsing ``ren`` or
    ``rstrobe`` and before ``rdone`` or ``wdone`` is pulsed as response,
    the bus is busy and no other transactions can be initiated. Simultaneous
    reads and writes are not allowed.

    Parameters
    ----------
    name : str
        Bank name. This is used for the SVD generation.
    registers : Dict[int, Register]
        A dictionary of the registers to be added to the bank. The
        key of the dictionary is the address of each register.
    address_width : int
        Address width.
    width : int
        Data width.

    Attributes
    ----------
    ren : Signal(), in
        Read enable.
    rdone : Signal(), out
        Read done.
    wstrobe : Signal(width // 8), in
        Write strobe.
    wdone : Signal(), out
        Write done.
    address : Signal(address_width), in
        Read and write address.
    rdata : Signal(width), out
        Read data.
    wdata : Signal(width), in
        Write data.
    """
    def __init__(self, name: str, registers: Dict[int, Register],
                 address_width: int, width: int = 32):
        self.name = name
        self.w = width
        self.aw = address_width
        self.registers = registers
        self.nstrobes = width // 8

        self.ren = Signal()
        self.rdone = Signal()
        self.wstrobe = Signal(self.nstrobes)
        self.wdone = Signal()
        self.address = Signal(self.aw)
        self.rdata = Signal(self.w, reset_less=True)
        self.wdata = Signal(self.w)

    def __getitem__(self, name: str):
        for register in self.registers.values():
            if register.name == name:
                return register
        raise KeyError

    def elaborate(self, platform):
        m = Module()
        reg_enable = Signal(len(self.registers))
        rdata = 0
        for j, register in enumerate(self.registers.items()):
            address = register[0]
            reg = register[1]
            m.submodules[reg.name] = reg
            m.d.comb += [
                reg_enable[j].eq(self.address == address),
                reg.ren.eq(self.ren & reg_enable[j]),
                reg.wstrobe.eq(Mux(reg_enable[j], self.wstrobe, 0)),
                reg.wdata.eq(self.wdata),
            ]
            rdata |= reg.rdata
        m.d.sync += [
            self.rdata.eq(rdata),
            self.rdone.eq(self.ren),
            self.wdone.eq(self.wstrobe != 0),
        ]
        return m

    def svd(self):
        registers = ET.Element('registers')
        for address, register in self.registers.items():
            reg = register.svd()
            addr = address * self.nstrobes
            reg.find('addressOffset').text = f'0x{addr:x}'
            registers.append(reg)
        return registers

    @property
    def size(self):
        return (
            2**Shape.cast(range(max(self.registers.keys()))).width
            * self.nstrobes)


class RegisterMap:
    def __init__(self, registers: Dict[int, Registers],
                 metadata: Dict[str, str]):
        self.registers = registers
        self.meta = metadata

    @property
    def size(self):
        return max([k + v.size for k, v in self.registers.items()])

    def svd(self):
        device = ET.Element('device')
        device.set('schemaVersion', '1.1')
        device.set('xmlns:xs', 'http://www.w3.org/2001/XMLSchema-instance')
        device.set('xs:noNamespaceSchemaLocation', 'CMSIS-SVD.xsd')
        for key in ['vendor', 'vendorID', 'name', 'series', 'version',
                    'description', 'licenseText']:
            el = ET.SubElement(device, key)
            el.text = self.meta[key]
        for element in ['width', 'size']:
            el = ET.SubElement(device, element)
            el.text = '32'
        peripherals = ET.SubElement(device, 'peripherals')
        peripheral = ET.SubElement(peripherals, 'peripheral')
        for key in ['name', 'version', 'description']:
            el = ET.SubElement(peripheral, key)
            el.text = self.meta[key]
        baseAddress = ET.SubElement(peripheral, 'baseAddress')
        baseAddress.text = '0x00000000'
        access = ET.SubElement(peripheral, 'access')
        access.text = 'read-write'
        address_block = ET.SubElement(peripheral, 'addressBlock')
        offset = ET.SubElement(address_block, 'offset')
        offset.text = '0'
        size = ET.SubElement(address_block, 'size')
        size.text = f'0x{self.size:x}'
        usage = ET.SubElement(address_block, 'usage')
        usage.text = 'registers'
        registers = ET.SubElement(peripheral, 'registers')
        for addr_offset, regs in self.registers.items():
            regs_svd = regs.svd()
            for reg in regs_svd.findall('register'):
                addr = reg.find('addressOffset')
                addr.text = f'0x{addr_offset + int(addr.text, 16):x}'
                registers.append(reg)
        ET.indent(device, space=' '*2, level=0)
        xml = b'<?xml version="1.0" encoding="utf-8"?>\n' + ET.tostring(device)
        return xml
