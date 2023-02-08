//
// Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
//
// This file is part of maia-sdr
//
// SPDX-License-Identifier: MIT
//

`timescale 1ps/1ps

module tb
  (
   input wire         sampling_clk,
   input wire         clk,
   input wire         clk2x_clk,
   input wire         clk3x_clk,
   input wire         s_axi_lite_clk,
   input wire         s_axi_lite_rst,
   input wire [11:0]  re_in,
   input wire [11:0]  im_in,
   output wire        rst,
   output wire [31:0] AWADDR,
   output wire [2:0]  AWPROT,
   input wire         AWREADY,
   output wire        AWVALID,
   output wire        BREADY,
   input wire [1:0]   BRESP,
   input wire         BVALID,
   output wire [31:0] WDATA,
   input wire         WREADY,
   output wire [3:0]  WSTRB,
   output wire        WVALID,
   input wire         ARVALID,
   output wire        ARREADY,
   input wire [3:0]   ARADDR,
   input wire [2:0]   ARPROT,
   output wire        RVALID,
   input wire         RREADY,
   output wire [31:0] RDATA,
   output wire [1:0]  RRESP
   );

   glbl glbl();

   dut dut
     (.sampling_clk(sampling_clk),
      .clk(clk), .clk2x_clk(clk2x_clk), .clk3x_clk(clk3x_clk),
      .s_axi_lite_clk(s_axi_lite_clk), .s_axi_lite_rst(s_axi_lite_rst),
      .re_in(re_in), .im_in(im_in),
      .rst(rst),
      .m_axi_spectrometer_awready(1'b1), .m_axi_spectrometer_wready(1'b1),
      .s_axi_lite_awaddr(AWADDR), .s_axi_lite_awprot(AWPROT),
      .s_axi_lite_awvalid(AWVALID), .s_axi_lite_awready(AWREADY),
      .s_axi_lite_wdata(WDATA), .s_axi_lite_wstrb(WSTRB),
      .s_axi_lite_wvalid(WVALID), .s_axi_lite_wready(WREADY),
      .s_axi_lite_bresp(BRESP), .s_axi_lite_bvalid(BVALID),
      .s_axi_lite_bready(BREADY), .s_axi_lite_arvalid(ARVALID),
      .s_axi_lite_arready(ARREADY), .s_axi_lite_araddr(ARADDR),
      .s_axi_lite_arprot(ARPROT), .s_axi_lite_rvalid(RVALID),
      .s_axi_lite_rready(RREADY), .s_axi_lite_rdata(RDATA),
      .s_axi_lite_rresp(RRESP));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
