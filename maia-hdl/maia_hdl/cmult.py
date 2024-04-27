#
# Copyright (C) 2022-2024 Daniel Estevez <daniel@destevez.net>
#
# This file is part of maia-sdr
#
# SPDX-License-Identifier: MIT
#

from amaranth import *
import amaranth.back.verilog
from amaranth.vendor.xilinx import XilinxPlatform

from .pluto_platform import PlutoPlatform


# This is based on Xilinx template for the complex multiplier using DSP48e's.
class Cmult(Elaboratable):
    """Complex multiplier

    A complex multiplier that uses 3 multipliers in pipeline to work at one
    sample per clock cycle. It is based on the Xilinx Verilog template for the
    complex multiplier using DSP48e's.

    Parameters
    ----------
    a_width : int
        Width of operand 'a'.
    b_width : int
        Width of operand 'b'.
    truncate : int
        Determines how many bits to truncate in the output.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    clken : Signal(), in
        Clock enable.
    re_a : Signal(signed(a_width)), in
        Real part of operand 'a'.
    im_a : Signal(signed(a_width)), in
        Imaginary part of operand 'a'.
    re_b : Signal(signed(b_width)), in
        Real part of operand 'b'.
    im_b : Signal(signed(b_width)), in
        Imaginary part of operand 'b'.
    re_out : Signal(signed(a_width + b_width + 1 - truncate)), out
        Real part of result 'a * b'.
    im_out : Signal(signed(a_width + b_width + 1 - truncate)), out
        Imaginary part of result 'a * b'.
    """
    def __init__(self, a_width, b_width, truncate=0):
        self.aw = a_width
        self.bw = b_width
        self.outw = self.aw + self.bw + 1 - truncate
        self.truncate = truncate

        self.clken = Signal()
        self.re_a = Signal(signed(self.aw))
        self.im_a = Signal(signed(self.aw))
        self.re_b = Signal(signed(self.bw))
        self.im_b = Signal(signed(self.bw))
        self.re_out = Signal(signed(self.outw))
        self.im_out = Signal(signed(self.outw))

    @property
    def delay(self):
        return 6

    def elaborate(self, platform):
        m = Module()
        re_a_q = [Signal(signed(self.aw), name=f're_a_q{i+1}',
                         reset_less=True)
                  for i in range(4)]
        im_a_q = [Signal(signed(self.aw), name=f'im_a_q{i+1}',
                         reset_less=True)
                  for i in range(4)]
        re_b_q = [Signal(signed(self.bw), name=f're_b_q{i+1}',
                         reset_less=True)
                  for i in range(3)]
        im_b_q = [Signal(signed(self.bw), name=f'im_b_q{i+1}',
                         reset_less=True)
                  for i in range(3)]
        add_common = Signal(signed(self.aw+1), reset_less=True)
        add_re = Signal(signed(self.bw+1), reset_less=True)
        add_im = Signal(signed(self.bw+1), reset_less=True)
        multw = self.aw + self.bw + 1
        mult0 = Signal(signed(multw), reset_less=True)
        mult_re = Signal(signed(multw), reset_less=True)
        mult_im = Signal(signed(multw), reset_less=True)
        common = Signal(signed(multw), reset_less=True)
        common_q_re = Signal(signed(multw), reset_less=True)
        common_q_im = Signal(signed(multw), reset_less=True)
        re_prod = Signal(signed(multw), reset_less=True)
        im_prod = Signal(signed(multw), reset_less=True)

        with m.If(self.clken):
            m.d.sync += re_a_q[0].eq(self.re_a)
            m.d.sync += [re_a_q[j].eq(re_a_q[j-1])
                         for j in range(1, len(re_a_q))]
            m.d.sync += im_a_q[0].eq(self.im_a)
            m.d.sync += [im_a_q[j].eq(im_a_q[j-1])
                         for j in range(1, len(im_a_q))]
            m.d.sync += re_b_q[0].eq(self.re_b)
            m.d.sync += [re_b_q[j].eq(re_b_q[j-1])
                         for j in range(1, len(re_b_q))]
            m.d.sync += im_b_q[0].eq(self.im_b)
            m.d.sync += [im_b_q[j].eq(im_b_q[j-1])
                         for j in range(1, len(im_b_q))]
            # common factor (re_a - im_a) * im_b
            m.d.sync += [
                add_common.eq(re_a_q[0] - im_a_q[0]),
                mult0.eq(add_common * im_b_q[1]),
                common.eq(mult0),
                common_q_re.eq(common),
                common_q_im.eq(common),
            ]
            m.d.sync += [
                # real product
                add_re.eq(re_b_q[2] - im_b_q[2]),
                mult_re.eq(add_re * re_a_q[3]),
                re_prod.eq(mult_re + common_q_re),
                # imaginary product
                add_im.eq(re_b_q[2] + im_b_q[2]),
                mult_im.eq(add_im * im_a_q[3]),
                im_prod.eq(mult_im + common_q_im),
            ]
        m.d.comb += [
            self.re_out.eq(re_prod >> self.truncate),
            self.im_out.eq(im_prod >> self.truncate),
        ]
        return m


class Cmult3x(Elaboratable):
    """Complex multiplier with 3x clock

    A complex multiplier that uses a 3x clock to re-use a single multiplier to
    perform the 3 multiplications in a complex product. It uses a single Xilinx
    DSP48e.

    Parameters
    ----------
    domain_3x : str
        Name of the clock domain of the 3x clock.
    a_width : int
        Width of operand 'a'.
    b_width : int
        Width of operand 'b'.
    truncate : int
        Determines how many bits to truncate in the output.

    Attributes
    ----------
    delay : int
        Delay (in samples) introduced by this module.
    common_edge : Signal(), in
        A signal that changes with the 3x clock and is high on the cycles
        immediately after the rising edge of the 1x clock.
    clken : Signal(), in
        Clock enable.
    re_a : Signal(signed(a_width)), in
        Real part of operand 'a'.
    im_a : Signal(signed(a_width)), in
        Imaginary part of operand 'a'.
    re_b : Signal(signed(b_width)), in
        Real part of operand 'b'.
    im_b : Signal(signed(b_width)), in
        Imaginary part of operand 'b'.
    re_out : Signal(signed(a_width + b_width + 1 - truncate)), out
        Real part of result 'a * b'.
    im_out : Signal(signed(a_width + b_width + 1 - truncate)), out
        Imaginary part of result 'a * b'.
    """
    def __init__(self, domain_3x: str, a_width: int, b_width: int,
                 truncate: int = 0):
        self._3x = domain_3x
        self.aw = a_width
        self.bw = b_width
        self.w = max(self.aw, self.bw)
        self.outw = self.aw + self.bw + 1 - truncate
        self.truncate = truncate

        self.common_edge = Signal()
        self.clken = Signal()
        self.re_a = Signal(signed(self.aw))
        self.im_a = Signal(signed(self.aw))
        self.re_b = Signal(signed(self.bw))
        self.im_b = Signal(signed(self.bw))
        self.re_out = Signal(signed(self.outw), reset_less=True)
        self.im_out = Signal(signed(self.outw), reset_less=True)

    @property
    def delay(self):
        return 3 if self.needs_wide_impl() else 2

    def needs_wide_impl(self):
        return max(self.aw, self.bw) > 18

    def elaborate(self, platform):
        if isinstance(platform, XilinxPlatform):
            return self.elaborate_xilinx(platform)

        # Amaranth design. Vivado doesn't infer a single DSP48E1 as we want.
        m = Module()

        # The wide implementation has an additional delay
        re_a_q = Signal(signed(self.aw), reset_less=True)
        im_a_q = Signal(signed(self.aw), reset_less=True)
        re_b_q = Signal(signed(self.bw), reset_less=True)
        im_b_q = Signal(signed(self.bw), reset_less=True)
        eqs = [
            re_a_q.eq(self.re_a),
            im_a_q.eq(self.im_a),
            re_b_q.eq(self.re_b),
            im_b_q.eq(self.im_b),
        ]

        if self.needs_wide_impl():
            with m.If(self.clken):
                m.d.sync += eqs
        else:
            m.d.comb += eqs

        reg_a1 = Signal(signed(self.w), reset_less=True)
        reg_d = Signal(signed(self.w), reset_less=True)
        reg_ad = Signal(signed(self.w + 1), reset_less=True)
        reg_b1 = Signal(signed(self.w), reset_less=True)
        reg_b2 = Signal(signed(self.w), reset_less=True)
        reg_m = Signal(signed(self.aw + self.bw + 1), reset_less=True)
        reg_c = Signal(signed(self.aw + self.bw + 1), reset_less=True)
        reg_p = Signal(signed(self.aw + self.bw + 1), reset_less=True)
        common_edge_q = Signal()
        common_edge_qq = Signal()

        with m.If(self.clken):
            m.d[self._3x] += [
                common_edge_q.eq(self.common_edge),
                common_edge_qq.eq(common_edge_q),
                reg_b2.eq(reg_b1),
                reg_m.eq(reg_ad * reg_b2),
                reg_c.eq(reg_p),
            ]
            with m.If(self.common_edge):
                m.d[self._3x] += [
                    reg_a1.eq(re_a_q),
                    reg_d.eq(im_a_q),
                    reg_b1.eq(im_b_q),
                    reg_ad.eq(reg_a1 + reg_d),
                    reg_p.eq(reg_m),
                ]
            with m.If(common_edge_q):
                m.d[self._3x] += [
                    reg_a1.eq(re_b_q),
                    reg_d.eq(im_b_q),
                    reg_b1.eq(re_a_q),
                    reg_ad.eq(reg_a1 - reg_d),
                    reg_p.eq(reg_p + reg_m),
                ]
            with m.If(common_edge_qq):
                m.d[self._3x] += [
                    reg_a1.eq(re_b_q),
                    reg_d.eq(im_b_q),
                    reg_b1.eq(im_a_q),
                    reg_ad.eq(reg_a1 - reg_d),
                    reg_p.eq(reg_c + reg_m),
                ]
            m.d.sync += self.re_out.eq(reg_p >> self.truncate)
            with m.If(self.common_edge):
                m.d[self._3x] += self.im_out.eq(reg_p >> self.truncate)

        return m

    def elaborate_xilinx(self, platform):
        # Design with an instantiated DSP48E1
        if min(self.aw, self.bw) > 18:
            raise ValueError('at least one operand must have 18 bits or less')
        if max(self.aw, self.bw) > 25:
            raise ValueError('the widest operand must have 25 bits or less')
        if self.needs_wide_impl():
            if min(self.aw, self.bw) > 17:
                raise ValueError(
                    'if there is an operand wider than 18 bits, '
                    'the other operand must have 17 bits or less')
            return self.elaborate_xilinx_wide(platform)
        assert self.aw <= 18
        assert self.bw <= 18
        m = Module()
        self.instantiate_dsp48(m)
        common_edge_q = Signal()
        common_edge_qq = Signal()
        with m.If(self.clken):
            m.d[self._3x] += [
                common_edge_q.eq(self.common_edge),
                common_edge_qq.eq(common_edge_q),
            ]
            m.d.sync += self.re_out.eq(self.port_p >> self.truncate)
            with m.If(self.common_edge):
                m.d[self._3x] += self.im_out.eq(self.port_p >> self.truncate)

        m.d.comb += [
            self.port_c.eq(self.port_p),
        ]
        with m.If(self.common_edge):
            m.d.comb += [
                self.port_d.eq(self.re_a),
                self.port_a.eq(self.im_a),
                self.port_b.eq(self.im_b),
                self.opmode.eq(0b010_01_01),  # P + M
                self.inmode.eq(0b01101),  # D - A1, B2
            ]
        with m.If(common_edge_q):
            m.d.comb += [
                self.port_d.eq(self.re_b),
                self.port_a.eq(self.im_b),
                self.port_b.eq(self.re_a),
                self.opmode.eq(0b011_01_01),  # C + M
                self.inmode.eq(0b01101),  # D - A1, B2
            ]
        with m.If(common_edge_qq):
            m.d.comb += [
                self.port_d.eq(self.re_b),
                self.port_a.eq(self.im_b),
                self.port_b.eq(self.im_a),
                self.opmode.eq(0b000_01_01),  # M
                self.inmode.eq(0b00101),  # D + A1, B2
            ]
        return m

    def elaborate_xilinx_wide(self, platform):
        # This is a variant for the case when one of the operands has width
        # larger than 18 but not larger than 25. In this case the preadder
        # cannot be used for both operands (because the B input is limited to
        # 18 bits), so there is an external preadder for the narrower input,
        # which is always fed in through port B.
        assert 18 < max(self.aw, self.bw) <= 25
        assert min(self.aw, self.bw) <= 17
        if self.aw >= self.bw:
            re_wide = self.re_a
            im_wide = self.im_a
            re_narrow = self.re_b
            im_narrow = self.im_b
        else:
            re_wide = self.re_b
            im_wide = self.im_b
            re_narrow = self.re_a
            im_narrow = self.im_a
        wide_width = len(re_wide)
        narrow_width = len(re_narrow)
        m = Module()
        self.instantiate_dsp48(m)
        common_edge_q = Signal()
        common_edge_qq = Signal()
        with m.If(self.clken):
            m.d[self._3x] += [
                common_edge_q.eq(self.common_edge),
                common_edge_qq.eq(common_edge_q),
            ]
            m.d.sync += self.re_out.eq(self.port_p >> self.truncate)
            with m.If(self.common_edge):
                m.d[self._3x] += self.im_out.eq(self.port_p >> self.truncate)

        narrow_diff = Signal(signed(narrow_width + 1), reset_less=True)
        re_wide_q = Signal(signed(wide_width), reset_less=True)
        im_wide_q = Signal(signed(wide_width), reset_less=True)
        re_narrow_q = Signal(signed(narrow_width), reset_less=True)
        im_narrow_q = Signal(signed(narrow_width), reset_less=True)
        with m.If(self.clken):
            m.d.sync += [
                narrow_diff.eq(re_narrow - im_narrow),
                re_wide_q.eq(re_wide),
                im_wide_q.eq(im_wide),
                re_narrow_q.eq(re_narrow),
                im_narrow_q.eq(im_narrow),
            ]
        m.d.comb += [
            self.port_c.eq(self.port_p),
            self.port_d.eq(re_wide_q),
            self.port_a.eq(im_wide_q),
        ]
        with m.If(self.common_edge):
            m.d.comb += [
                self.port_b.eq(narrow_diff),
                self.opmode.eq(0b010_01_01),  # P + M
                self.inmode.eq(0b00001),  # A1, B2
            ]
        with m.If(common_edge_q):
            m.d.comb += [
                self.port_b.eq(re_narrow_q),
                self.opmode.eq(0b011_01_01),  # C + M
                self.inmode.eq(0b01101),  # D - A1, B2
            ]
        with m.If(common_edge_qq):
            m.d.comb += [
                self.port_b.eq(im_narrow_q),
                self.opmode.eq(0b000_01_01),  # M
                self.inmode.eq(0b00101),  # D + A1, B2
            ]
        return m

    def instantiate_dsp48(self, m):
        self.port_a = Signal(signed(30), reset_less=True)
        self.port_b = Signal(signed(18), reset_less=True)
        self.port_d = Signal(signed(25), reset_less=True)
        self.port_c = Signal(48, reset_less=True)
        self.port_p = Signal(48, reset_less=True)
        self.inmode = Signal(5, reset_less=True)
        self.opmode = Signal(7, reset_less=True)
        m.submodules.dsp = self.dsp = Instance(
            'DSP48E1',
            p_A_INPUT='DIRECT',  # A port rather than ACIN
            p_B_INPUT='DIRECT',  # B port rather than BCIN
            p_USE_DPORT='TRUE',
            p_USE_MULT='MULTIPLY',
            p_USE_SIMD='ONE48',
            p_AUTORESET_PATDET='NO_RESET',
            p_MASK=2**48-1,  # ignore all bits
            p_PATTERN=0,
            p_SEL_MASK='MASK',
            p_SEL_PATTERN='PATTERN',
            p_USE_PATTERN_DETECT='NO_PATDET',
            p_ACASCREG=1,  # number of A register stages
            p_ADREG=1,
            p_ALUMODEREG=1,
            p_AREG=1,
            p_BCASCREG=2,
            p_BREG=2,
            p_CARRYINREG=1,
            p_CARRYINSELREG=1,
            p_CREG=1,
            p_DREG=1,
            p_INMODEREG=1,
            p_MREG=1,
            p_OPMODEREG=1,
            p_PREG=1,
            o_ACOUT=Signal(30),
            o_BCOUT=Signal(18),
            o_CARRYCASCOUT=Signal(),
            o_CARRYOUT=Signal(4),
            o_MULTSIGNOUT=Signal(),
            o_OVERFLOW=Signal(),
            o_P=self.port_p,
            o_PATTERNBDETECT=Signal(),
            o_PATTERNDETECT=Signal(),
            o_PCOUT=Signal(48),
            o_UNDERFLOW=Signal(),
            i_ACIN=Const(0, unsigned(30)),
            i_BCIN=Const(0, unsigned(18)),
            i_CARRYCASCIN=0,
            i_MULTSIGNIN=0,
            i_PCIN=Const(0, unsigned(48)),
            i_ALUMODE=Const(0, unsigned(4)),  # Z + X + Y + CIN
            i_CARRYINSEL=Const(0, unsigned(3)),
            i_CLK=ClockSignal(self._3x),
            i_INMODE=self.inmode,
            i_OPMODE=self.opmode,
            i_A=self.port_a,
            i_B=self.port_b,
            i_C=self.port_c,
            i_CARRYIN=0,
            i_D=self.port_d,
            i_CEA1=self.clken,
            i_CEA2=0,
            i_CEAD=self.clken,
            i_CEALUMODE=self.clken,
            i_CEB1=self.clken,
            i_CEB2=self.clken,
            i_CEC=self.clken,
            i_CECARRYIN=0,
            i_CECTRL=self.clken,
            i_CED=self.clken,
            i_CEINMODE=self.clken,
            i_CEM=self.clken,
            i_CEP=self.clken,
            i_RSTA=0,
            i_RSTALLCARRYIN=0,
            i_RSTALUMODE=0,
            i_RSTB=0,
            i_RSTC=0,
            i_RSTCTRL=0,
            i_RSTD=0,
            i_RSTINMODE=0,
            i_RSTM=0,
            i_RSTP=0)


if __name__ == '__main__':
    cmult = Cmult(a_width=16, b_width=16)
    with open('cmult.v', 'w') as f:
        f.write(
            amaranth.back.verilog.convert(
                cmult, name='cmult', ports=[
                    cmult.clken, cmult.re_a, cmult.im_a,
                    cmult.re_b, cmult.im_b,
                    cmult.re_out, cmult.im_out],
                emit_src=False))
    m = Cmult3x('clk3x', a_width=16, b_width=16)
    with open('cmult3x.v', 'w') as f:
        f.write(
            amaranth.back.verilog.convert(
                m, name='cmult3x',
                ports=[
                    m.common_edge, m.clken,
                    m.re_a, m.im_a,
                    m.re_b, m.im_b,
                    m.re_out, m.im_out],
                platform=PlutoPlatform(),
                emit_src=False))
