//
// Copyright (C) 2022-2023 Daniel Estevez <daniel@destevez.net>
//
// This file is part of maia-sdr
//
// SPDX-License-Identifier: MIT
//

module tb
  (
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
   input wire         clk,
   input wire         rst,
   output wire [63:0] WDATA,
   output wire        WLAST,
   input wire         WREADY,
   output wire [7:0]  WSTRB,
   output wire        WVALID,
   input wire         start,
   output wire        busy,
   output wire [5:0]  last_buffer,
   // These are used by cocotb
   input wire         ARREADY,
   input wire         RVALID,
   input wire         RLAST,
   output wire        ARVALID
   );

   assign ARVALID = 1'b0;

   wire [11:0]        raddr;
   wire               ren;
   reg [11:0]         raddr_q = 1'b0;
   reg [11:0]         raddr_q2 = 1'b0;

   always @(posedge clk) begin
      if (ren) begin
         raddr_q <= raddr;
         raddr_q2 <= raddr_q;
      end
   end

   wire [63:0] rdata = raddr_q2;

   dut dut
     (.awaddr(AWADDR), .awlen(AWLEN), .awsize(AWSIZE), .awburst(AWBURST),
      .awcache(AWCACHE), .awprot(AWPROT), .awvalid(AWVALID), .awready(AWREADY),
      .wdata(WDATA), .wstrb(WSTRB), .wlast(WLAST), .wvalid(WVALID),
      .wready(WREADY), .bresp(BRESP), .bvalid(BVALID), .bready(BREADY),
      .clk(clk), .rst(rst), .start(start), .busy(busy),
      .last_buffer(last_buffer), .raddr(raddr), .rdata(rdata), .ren(ren));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
