//
// Copyright (C) 2023 Daniel Estevez <daniel@destevez.net>
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
   input wire         clk3x_clk,
   input wire         clk3x_rst,
   input wire         clken,
   input wire [15:0]  re_a,
   input wire [15:0]  im_a,
   input wire [15:0]  re_b,
   input wire [15:0]  im_b,
   output wire [32:0] re_out,
   output wire [32:0] im_out
   );

   glbl glbl ();

   dut dut
     (.clk(clk), .rst(rst), .clk3x_clk(clk3x_clk), .clk3x_rst(clk3x_rst),
      .clken(clken), .re_a(re_a), .im_a(im_a), .re_b(re_b), .im_b(im_b),
      .re_out(re_out), .im_out(im_out));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
