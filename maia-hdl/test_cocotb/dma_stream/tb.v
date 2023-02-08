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
   input wire         stop,
   output wire        finished,
   input wire [63:0]  stream_data,
   input wire         stream_valid,
   output wire        stream_ready,
   // These are used by cocotb
   input wire         ARREADY,
   input wire         RVALID,
   input wire         RLAST,
   output wire        ARVALID
   );

   assign ARVALID = 1'b0;

   dut dut
     (.awaddr(AWADDR), .awlen(AWLEN), .awsize(AWSIZE), .awburst(AWBURST),
      .awcache(AWCACHE), .awprot(AWPROT), .awvalid(AWVALID), .awready(AWREADY),
      .wdata(WDATA), .wstrb(WSTRB), .wlast(WLAST), .wvalid(WVALID),
      .wready(WREADY), .bresp(BRESP), .bvalid(BVALID), .bready(BREADY),
      .clk(clk), .rst(rst), .start(start), .stop(stop), .finished(finished),
      .stream_data(stream_data), .stream_valid(stream_valid), .stream_ready(stream_ready));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
