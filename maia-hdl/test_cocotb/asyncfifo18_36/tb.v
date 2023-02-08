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
   input wire        read_clk,
   input wire        read_rst,
   input wire        write_clk,
   input wire        write_rst,
   input wire        fifo_rst,
   input wire [35:0] data_in,
   input wire        wren,
   output wire       full,
   output wire       wrerr,
   input wire [35:0] data_out,
   input wire        rden,
   output wire       empty,
   output wire       rderr
   );

   glbl glbl ();

   dut dut
     (.read_clk(read_clk), .read_rst(read_rst),
      .write_clk(write_clk), .write_rst(write_rst),
      .reset(fifo_rst),
      .data_in(data_in), .wren(wren), .full(full), .wrerr(wrerr),
      .data_out(data_out), .rden(rden), .empty(empty), .rderr(rderr));

`ifdef COCOTB_SIM
   initial begin
      $dumpfile("dump.vcd");
      $dumpvars(0, dut);
   end
`endif
endmodule // tb
