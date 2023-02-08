# Set false path for Amaranth generated attributes
set_false_path -to [get_cells -hier -filter {amaranth.vivado.false_path == "TRUE"}]

# TODO: change by max delay constraint according to the time
# that the pulse takes to propagate down the synchronizer
# chain.
#
# syntax: sig.attrs['amaranth.vivado.false_path'] = 'delay_ns'
# where delay_ns is the delay in ns (e.g., 2.31)
#
# The delay should be 1 clock cycle of the destination clock.
set_false_path -to [get_pins -hierarchical "*cdc_request_data_dest_reg*/D"]
set_false_path -to [get_pins -hierarchical "*cdc_response_data_dest_reg*/D"]

# False path for the RST of the FIFO18E1 used in the recorder
set_false_path -to [get_pins recorder/fifo/fifo18e1/RST]

# False path for the RST of the FIFO18E1 used in the RX IQ CDC
set_false_path -to [get_pins rxiq_cdc/fifo/fifo18e1/RST]
