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
   input wire         clk,
   input wire         rst,
   input wire         iq_clk,
   input wire         iq_rst,
   input wire [11:0]  re_in,
   input wire [11:0]  im_in,
   input wire         strobe_in,
   input wire         mode_8bit,
   output wire        dropped_samples,
   output wire [31:0] next_address,
   input wire         start,
   input wire         stop,
   output wire        finished,
   output wire [31:0] AWADDR,
   output wire [1:0]  AWBURST,
   output wire [3:0]  AWCACHE,
   output wire [3:0]  AWLEN,
   output wire [1:0]  AWLOCK,
   output wire [2:0]  AWPROT,
   input wire         AWREADY,
   output wire [2:0]  AWSIZE,
   output wire        AWVALID,
   output wire        BREADY,
   input wire [1:0]   BRESP,
   input wire         BVALID,
   output wire [63:0] WDATA,
   output wire        WLAST,
   input wire         WREADY,
   output wire [7:0]  WSTRB,
   output wire        WVALID,
   // These are used by cocotb
   input wire         ARREADY,
   input wire         RVALID,
   input wire         RLAST,
   output wire        ARVALID
   );

   glbl glbl ();

   assign ARVALID = 1'b0;

   dut dut
     (.clk(clk), .rst(rst), .iq_clk(iq_clk), .iq_rst(iq_rst),
      .re_in(re_in), .im_in(im_in), .strobe_in(strobe_in),
      .mode_8bit(mode_8bit), .dropped_samples(dropped_samples), .start(start),
      .stop(stop), .finished(finished), .next_address(next_address),
      .awaddr(AWADDR), .awlen(AWLEN), .awsize(AWSIZE), .awburst(AWBURST),
      .awcache(AWCACHE), .awprot(AWPROT), .awvalid(AWVALID), .awready(AWREADY),
      .wdata(WDATA), .wstrb(WSTRB), .wlast(WLAST), .wvalid(WVALID),
      .wready(WREADY), .bresp(BRESP), .bvalid(BVALID), .bready(BREADY));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
