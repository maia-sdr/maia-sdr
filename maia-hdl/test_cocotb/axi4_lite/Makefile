# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog

DUT = axi4_lite
# Force Verilog 2005 mode, since Verilog 2012 gives problems with initial value
# for combinatorial blocks.
VERILOG_SOURCES = -g2005
VERILOG_SOURCES += dut.v
VERILOG_SOURCES += tb.v
TOPLEVEL = tb
MODULE = test_$(DUT)

COMPILE_ARGS += -Wall

export PYTHONPATH := $(PWD)/../..:$(PYTHONPATH)

# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim

.PHONY: dut.v
dut.v:
	python3 verilog.py

.PHONY: -g2005
-g2005:
	echo "Verilog 2005 mode enabled"
