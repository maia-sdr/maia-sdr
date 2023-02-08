# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog

DUT = dma_stream
VERILOG_SOURCES += dut.v
VERILOG_SOURCES += tb.v
TOPLEVEL = tb
MODULE = test_$(DUT)

COMPILE_ARGS += -Wall

export PYTHONPATH := $(PWD)/../..:$(PWD)/..:$(PYTHONPATH)

# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim

.PHONY: dut.v
dut.v:
	python3 verilog.py
