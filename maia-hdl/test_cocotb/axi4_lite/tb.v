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
   output wire [2:0]  AWPROT,
   input wire         AWREADY,
   output wire        AWVALID,
   output wire        BREADY,
   input wire [1:0]   BRESP,
   input wire         BVALID,
   input wire         clk,
   input wire         rst,
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

   dut dut
     (.awaddr(AWADDR), .awprot(AWPROT), .awvalid(AWVALID), .awready(AWREADY),
      .wdata(WDATA), .wstrb(WSTRB), .wvalid(WVALID), .wready(WREADY),
      .bresp(BRESP), .bvalid(BVALID), .bready(BREADY),
      .arvalid(ARVALID), .arready(ARREADY), .araddr(ARADDR), .arprot(ARPROT),
      .rvalid(RVALID), .rready(RREADY), .rdata(RDATA), .rresp(RRESP),
      .clk(clk), .rst(rst));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
