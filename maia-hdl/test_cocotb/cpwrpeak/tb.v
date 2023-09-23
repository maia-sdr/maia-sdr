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
   input wire [15:0]  re_in,
   input wire [15:0]  im_in,
   input wire [23:0]  real_in,
   input wire         peak_detect,
   output wire [24:0] out,
   output wire        is_greater
   );

   glbl glbl ();

   dut dut
     (.clk(clk), .rst(rst), .clk3x_clk(clk3x_clk), .clk3x_rst(clk3x_rst),
      .clken(clken), .re_in(re_in), .im_in(im_in), .real_in(real_in),
      .peak_detect(peak_detect), .out(out), .is_greater(is_greater));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
