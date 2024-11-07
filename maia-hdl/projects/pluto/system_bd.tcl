# create board design

# Add IP repo path for Maia SDR
#
# We need to do this here because adi_project_create overwrites whatever we had
# set beforehand.
set_property ip_repo_paths {../../ip ../../adi-hdl/library} [current_fileset]
update_ip_catalog

# default ports

create_bd_intf_port -mode Master -vlnv xilinx.com:interface:ddrx_rtl:1.0 ddr
create_bd_intf_port -mode Master -vlnv xilinx.com:display_processing_system7:fixedio_rtl:1.0 fixed_io

create_bd_port -dir O spi0_csn_2_o
create_bd_port -dir O spi0_csn_1_o
create_bd_port -dir O spi0_csn_0_o
create_bd_port -dir I spi0_csn_i
create_bd_port -dir I spi0_clk_i
create_bd_port -dir O spi0_clk_o
create_bd_port -dir I spi0_sdo_i
create_bd_port -dir O spi0_sdo_o
create_bd_port -dir I spi0_sdi_i

create_bd_port -dir I -from 16 -to 0 gpio_i
create_bd_port -dir O -from 16 -to 0 gpio_o
create_bd_port -dir O -from 16 -to 0 gpio_t

# instance: sys_ps7

ad_ip_instance processing_system7 sys_ps7

# ps7 settings

ad_ip_parameter sys_ps7 CONFIG.PCW_PRESET_BANK0_VOLTAGE {LVCMOS 1.8V}
ad_ip_parameter sys_ps7 CONFIG.PCW_PRESET_BANK1_VOLTAGE {LVCMOS 1.8V}
ad_ip_parameter sys_ps7 CONFIG.PCW_PACKAGE_NAME clg225
ad_ip_parameter sys_ps7 CONFIG.PCW_USE_S_AXI_HP1 1
ad_ip_parameter sys_ps7 CONFIG.PCW_USE_S_AXI_HP2 1
ad_ip_parameter sys_ps7 CONFIG.PCW_EN_CLK1_PORT 1
ad_ip_parameter sys_ps7 CONFIG.PCW_EN_RST1_PORT 1
ad_ip_parameter sys_ps7 CONFIG.PCW_FPGA0_PERIPHERAL_FREQMHZ 100.0
ad_ip_parameter sys_ps7 CONFIG.PCW_FPGA1_PERIPHERAL_FREQMHZ 200.0
ad_ip_parameter sys_ps7 CONFIG.PCW_GPIO_EMIO_GPIO_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_GPIO_EMIO_GPIO_IO 17

if {[info exists plutoplus]} {
    # Pluto+ Ethernet (not available in ADALM Pluto)
    ad_ip_parameter sys_ps7 CONFIG.PCW_EN_ENET0 1
    ad_ip_parameter sys_ps7 CONFIG.PCW_ENET0_PERIPHERAL_ENABLE 1
    ad_ip_parameter sys_ps7 CONFIG.PCW_ENET0_ENET0_IO {MIO 16 .. 27}
    ad_ip_parameter sys_ps7 CONFIG.PCW_ENET0_GRP_MDIO_ENABLE 1
    ad_ip_parameter sys_ps7 CONFIG.PCW_ENET0_GRP_MDIO_IO {MIO 52 .. 53}
}

ad_ip_parameter sys_ps7 CONFIG.PCW_SPI1_PERIPHERAL_ENABLE 0
ad_ip_parameter sys_ps7 CONFIG.PCW_I2C0_PERIPHERAL_ENABLE 0
ad_ip_parameter sys_ps7 CONFIG.PCW_UART1_PERIPHERAL_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_UART1_UART1_IO {MIO 12 .. 13}
ad_ip_parameter sys_ps7 CONFIG.PCW_I2C1_PERIPHERAL_ENABLE 0
ad_ip_parameter sys_ps7 CONFIG.PCW_QSPI_PERIPHERAL_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_QSPI_GRP_SINGLE_SS_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_SPI0_PERIPHERAL_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_SPI0_SPI0_IO EMIO

if {[info exists plutoplus]} {
    # Pluto+ SD card (not available in ADALM Pluto)
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_PERIPHERAL_ENABLE 1
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_SD0_IO "MIO 40 .. 45"
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_GRP_CD_ENABLE 1
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_GRP_CD_IO "MIO 47"
    ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_47_PULLUP {enabled}
    ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_47_SLEW {slow}
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_GRP_POW_ENABLE    0
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_GRP_WP_ENABLE     0
} else {
    ad_ip_parameter sys_ps7 CONFIG.PCW_SD0_PERIPHERAL_ENABLE 0
}

ad_ip_parameter sys_ps7 CONFIG.PCW_TTC0_PERIPHERAL_ENABLE 0
ad_ip_parameter sys_ps7 CONFIG.PCW_USE_FABRIC_INTERRUPT 1
ad_ip_parameter sys_ps7 CONFIG.PCW_USB0_PERIPHERAL_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_GPIO_MIO_GPIO_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_GPIO_MIO_GPIO_IO MIO

if {[info exists plutoplus]} {
    # Different USB reset MIO for Pluto+
    ad_ip_parameter sys_ps7 CONFIG.PCW_USB0_RESET_IO {MIO 46}
    ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_46_SLEW {slow}
    ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_46_PULLUP {enabled}
} else {
    ad_ip_parameter sys_ps7 CONFIG.PCW_USB0_RESET_IO {MIO 52}
}

ad_ip_parameter sys_ps7 CONFIG.PCW_USB0_RESET_ENABLE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_IRQ_F2P_INTR 1
ad_ip_parameter sys_ps7 CONFIG.PCW_IRQ_F2P_MODE REVERSE
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_0_PULLUP {enabled}
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_9_PULLUP {enabled}
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_10_PULLUP {enabled}
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_11_PULLUP {enabled}
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_48_PULLUP {enabled}
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_49_PULLUP {disabled}
ad_ip_parameter sys_ps7 CONFIG.PCW_MIO_53_PULLUP {enabled}

# DDR MT41K256M16 HA-125 (32M, 16bit, 8banks)

ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_PARTNO {MT41K256M16 RE-125}
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_BUS_WIDTH {16 Bit}
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_USE_INTERNAL_VREF 0
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_TRAIN_WRITE_LEVEL 1
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_TRAIN_READ_GATE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_TRAIN_DATA_EYE 1
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_DQS_TO_CLK_DELAY_0 0.048
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_DQS_TO_CLK_DELAY_1 0.050
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_BOARD_DELAY0 0.241
ad_ip_parameter sys_ps7 CONFIG.PCW_UIPARAM_DDR_BOARD_DELAY1 0.240

ad_ip_instance xlconcat sys_concat_intc
ad_ip_parameter sys_concat_intc CONFIG.NUM_PORTS 16

ad_ip_instance proc_sys_reset sys_rstgen
ad_ip_parameter sys_rstgen CONFIG.C_EXT_RST_WIDTH 1

# system reset/clock definitions

ad_connect  sys_cpu_clk sys_ps7/FCLK_CLK0
ad_connect  sys_200m_clk sys_ps7/FCLK_CLK1
ad_connect  sys_cpu_reset sys_rstgen/peripheral_reset
ad_connect  sys_cpu_resetn sys_rstgen/peripheral_aresetn
ad_connect  sys_cpu_clk sys_rstgen/slowest_sync_clk
ad_connect  sys_rstgen/ext_reset_in sys_ps7/FCLK_RESET0_N

# interface connections

ad_connect  ddr sys_ps7/DDR
ad_connect  gpio_i sys_ps7/GPIO_I
ad_connect  gpio_o sys_ps7/GPIO_O
ad_connect  gpio_t sys_ps7/GPIO_T
ad_connect  fixed_io sys_ps7/FIXED_IO

# ps7 spi connections

ad_connect  spi0_csn_2_o sys_ps7/SPI0_SS2_O
ad_connect  spi0_csn_1_o sys_ps7/SPI0_SS1_O
ad_connect  spi0_csn_0_o sys_ps7/SPI0_SS_O
ad_connect  spi0_csn_i sys_ps7/SPI0_SS_I
ad_connect  spi0_clk_i sys_ps7/SPI0_SCLK_I
ad_connect  spi0_clk_o sys_ps7/SPI0_SCLK_O
ad_connect  spi0_sdo_i sys_ps7/SPI0_MOSI_I
ad_connect  spi0_sdo_o sys_ps7/SPI0_MOSI_O
ad_connect  spi0_sdi_i sys_ps7/SPI0_MISO_I

# interrupts

ad_connect  sys_concat_intc/dout sys_ps7/IRQ_F2P
ad_connect  sys_concat_intc/In15 GND
ad_connect  sys_concat_intc/In14 GND
ad_connect  sys_concat_intc/In13 GND
ad_connect  sys_concat_intc/In12 GND
ad_connect  sys_concat_intc/In11 GND
ad_connect  sys_concat_intc/In10 GND
ad_connect  sys_concat_intc/In9 GND
ad_connect  sys_concat_intc/In8 GND
ad_connect  sys_concat_intc/In7 GND
ad_connect  sys_concat_intc/In6 GND
ad_connect  sys_concat_intc/In5 GND
ad_connect  sys_concat_intc/In4 GND
ad_connect  sys_concat_intc/In3 GND
ad_connect  sys_concat_intc/In2 GND
ad_connect  sys_concat_intc/In1 GND
ad_connect  sys_concat_intc/In0 GND

# ad9361

create_bd_port -dir I rx_clk_in
create_bd_port -dir I rx_frame_in
create_bd_port -dir I -from 11 -to 0 rx_data_in

create_bd_port -dir O tx_clk_out
create_bd_port -dir O tx_frame_out
create_bd_port -dir O -from 11 -to 0 tx_data_out

create_bd_port -dir O enable
create_bd_port -dir O txnrx
create_bd_port -dir I up_enable
create_bd_port -dir I up_txnrx

# ad9361 core(s)

ad_ip_instance axi_ad9361 axi_ad9361
ad_ip_parameter axi_ad9361 CONFIG.ID 0
ad_ip_parameter axi_ad9361 CONFIG.CMOS_OR_LVDS_N 1
ad_ip_parameter axi_ad9361 CONFIG.MODE_1R1T 1
ad_ip_parameter axi_ad9361 CONFIG.ADC_INIT_DELAY 21

# parameters to reduce size
ad_ip_parameter axi_ad9361 CONFIG.TDD_DISABLE 1
ad_ip_parameter axi_ad9361 CONFIG.DAC_DDS_DISABLE 1
	
if {![info exists maia_iio]} {
	ad_ip_parameter axi_ad9361 CONFIG.ADC_USERPORTS_DISABLE 1
	ad_ip_parameter axi_ad9361 CONFIG.ADC_DCFILTER_DISABLE 1
	ad_ip_parameter axi_ad9361 CONFIG.ADC_IQCORRECTION_DISABLE 1
	ad_ip_parameter axi_ad9361 CONFIG.DAC_USERPORTS_DISABLE 1
	ad_ip_parameter axi_ad9361 CONFIG.DAC_IQCORRECTION_DISABLE 1
}
# Maia SDR core

if {[info exists maia_iio]} {
	ad_ip_instance maia_sdr_maia_iio maia_sdr
} else {
	ad_ip_instance maia_sdr_default maia_sdr
}

ad_ip_instance xlslice adc_i_slice
ad_ip_parameter adc_i_slice CONFIG.DIN_WIDTH 16
ad_ip_parameter adc_i_slice CONFIG.DOUT_WIDTH 12
ad_ip_parameter adc_i_slice CONFIG.DIN_FROM 11

ad_ip_instance xlslice adc_q_slice
ad_ip_parameter adc_q_slice CONFIG.DIN_TO 0
ad_ip_parameter adc_q_slice CONFIG.DIN_WIDTH 16
ad_ip_parameter adc_q_slice CONFIG.DOUT_WIDTH 12
ad_ip_parameter adc_q_slice CONFIG.DIN_FROM 11

# Maia SDR clocking

create_bd_cell -type ip -vlnv xilinx.com:ip:clk_wiz:6.0 maia_sdr_clk
set_property -dict [list CONFIG.USE_PHASE_ALIGNMENT {false} CONFIG.ENABLE_CLOCK_MONITOR {false} CONFIG.PRIM_SOURCE {Global_buffer} \
                        CONFIG.CLKOUT2_USED {true} CONFIG.CLKOUT3_USED {true} CONFIG.NUM_OUT_CLKS {3} \
                        CONFIG.CLKOUT1_REQUESTED_OUT_FREQ {62.500} CONFIG.CLKOUT2_REQUESTED_OUT_FREQ {125.000} \
                        CONFIG.CLKOUT3_REQUESTED_OUT_FREQ {187.5} \
                        CONFIG.PRIMITIVE {MMCM} CONFIG.MMCM_DIVCLK_DIVIDE {1} CONFIG.MMCM_CLKFBOUT_MULT_F {11.250} \
                        CONFIG.MMCM_CLKOUT0_DIVIDE_F {18.000} CONFIG.MMCM_CLKOUT1_DIVIDE {9} \
                        CONFIG.MMCM_CLKOUT3_DIVIDE {6} \
                        CONFIG.CLKOUT1_JITTER {133.663} CONFIG.CLKOUT1_PHASE_ERROR {91.100} \
                        CONFIG.CLKOUT2_JITTER {116.571} CONFIG.CLKOUT2_PHASE_ERROR {91.100} \
                        CONFIG.CLKOUT3_JITTER {108.217} CONFIG.CLKOUT3_PHASE_ERROR {91.100}] [get_bd_cells maia_sdr_clk]

# connections

ad_connect  rx_clk_in axi_ad9361/rx_clk_in
ad_connect  rx_frame_in axi_ad9361/rx_frame_in
ad_connect  rx_data_in axi_ad9361/rx_data_in
ad_connect  tx_clk_out axi_ad9361/tx_clk_out
ad_connect  tx_frame_out axi_ad9361/tx_frame_out
ad_connect  tx_data_out axi_ad9361/tx_data_out
ad_connect  enable axi_ad9361/enable
ad_connect  txnrx axi_ad9361/txnrx
ad_connect  up_enable axi_ad9361/up_enable
ad_connect  up_txnrx axi_ad9361/up_txnrx

ad_connect  axi_ad9361/tdd_sync GND
ad_connect  sys_200m_clk axi_ad9361/delay_clk
ad_connect  axi_ad9361/l_clk axi_ad9361/clk

ad_connect  axi_ad9361/adc_data_i0 adc_i_slice/Din
ad_connect  axi_ad9361/adc_data_q0 adc_q_slice/Din
ad_connect  adc_i_slice/Dout maia_sdr/re_in
ad_connect  adc_q_slice/Dout maia_sdr/im_in
ad_connect  axi_ad9361/l_clk maia_sdr/sampling_clk
ad_connect  sys_cpu_clk maia_sdr/s_axi_lite_clk
ad_connect  sys_cpu_reset maia_sdr/s_axi_lite_rst
ad_connect  maia_sdr_clk/clk_out1 maia_sdr/clk
ad_connect  maia_sdr_clk/clk_out2 maia_sdr/clk2x_clk
ad_connect  maia_sdr_clk/clk_out3 maia_sdr/clk3x_clk

ad_connect  sys_cpu_clk maia_sdr_clk/clk_in1
ad_connect  sys_cpu_reset maia_sdr_clk/reset

if {[info exists maia_iio]} {

	ad_ip_instance axi_dmac axi_ad9361_dac_dma
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.DMA_TYPE_SRC 0
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.DMA_TYPE_DEST 1
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.CYCLIC 1
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.AXI_SLICE_SRC 0
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.AXI_SLICE_DEST 0
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.DMA_2D_TRANSFER 0
	ad_ip_parameter axi_ad9361_dac_dma CONFIG.DMA_DATA_WIDTH_DEST 64

	ad_ip_instance util_upack2 tx_upack

	ad_ip_instance axi_dmac axi_ad9361_adc_dma
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.DMA_TYPE_SRC 2
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.DMA_TYPE_DEST 0
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.CYCLIC 0
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.SYNC_TRANSFER_START 0
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.AXI_SLICE_SRC 0
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.AXI_SLICE_DEST 0
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.DMA_2D_TRANSFER 0
	ad_ip_parameter axi_ad9361_adc_dma CONFIG.DMA_DATA_WIDTH_SRC 64
	ad_ip_instance util_cpack2 cpack

	ad_connect axi_ad9361/adc_enable_i0 cpack/enable_0
	ad_connect axi_ad9361/adc_data_q0 cpack/fifo_wr_data_1

	ad_connect axi_ad9361/l_clk cpack/clk
	ad_connect axi_ad9361/rst cpack/reset
	ad_connect axi_ad9361_adc_dma/fifo_wr cpack/packed_fifo_wr

	ad_connect  axi_ad9361/l_clk tx_upack/clk
	ad_connect  axi_ad9361/rst tx_upack/reset
	ad_connect tx_upack/s_axis  axi_ad9361_dac_dma/m_axis

	ad_ip_instance util_vector_logic logic_or [list \
	  C_OPERATION {or} \
	  C_SIZE 1]

	ad_connect  logic_or/Op1  axi_ad9361/dac_valid_i0
	ad_connect  logic_or/Op2  axi_ad9361/dac_valid_i1
	ad_connect  logic_or/Res  tx_upack/fifo_rd_en
	ad_connect  tx_upack/fifo_rd_underflow axi_ad9361/dac_dunf

	ad_connect  axi_ad9361/l_clk axi_ad9361_adc_dma/fifo_wr_clk
	ad_connect  axi_ad9361/l_clk axi_ad9361_dac_dma/m_axis_aclk
	ad_connect  cpack/fifo_wr_overflow axi_ad9361/adc_dovf

	ad_connect sys_cpu_resetn axi_ad9361_adc_dma/m_dest_axi_aresetn
	ad_connect sys_cpu_resetn axi_ad9361_dac_dma/m_src_axi_aresetn
}
# interconnects

ad_cpu_interconnect 0x79020000 axi_ad9361
if {[info exists maia_iio]} {
	ad_cpu_interconnect 0x7C460000 maia_sdr
	ad_cpu_interconnect 0x7C400000 axi_ad9361_adc_dma
	ad_cpu_interconnect 0x7C420000 axi_ad9361_dac_dma
} else {
	ad_cpu_interconnect 0x7C400000 maia_sdr
}
ad_ip_parameter sys_ps7 CONFIG.PCW_USE_S_AXI_HP1 {1}
ad_connect maia_sdr_clk/clk_out1 sys_ps7/S_AXI_HP1_ACLK
ad_connect maia_sdr/m_axi_spectrometer sys_ps7/S_AXI_HP1

ad_ip_parameter sys_ps7 CONFIG.PCW_USE_S_AXI_HP2 {1}
if {[info exists maia_iio]} {
	ad_mem_hp2_interconnect sys_cpu_clk sys_ps7/S_AXI_HP2
	ad_mem_hp2_interconnect sys_cpu_clk maia_sdr/m_axi_recorder
	ad_mem_hp2_interconnect sys_cpu_clk axi_ad9361_adc_dma/m_dest_axi
	ad_mem_hp2_interconnect sys_cpu_clk axi_ad9361_dac_dma/m_src_axi
} else {
	ad_connect sys_cpu_clk sys_ps7/S_AXI_HP2_ACLK
	ad_connect maia_sdr/m_axi_recorder sys_ps7/S_AXI_HP2
	create_bd_addr_seg -range 0x20000000 -offset 0x00000000 \
		            [get_bd_addr_spaces maia_sdr/m_axi_recorder] \
		            [get_bd_addr_segs sys_ps7/S_AXI_HP2/HP2_DDR_LOWOCM] \
		            SEG_sys_ps7_HP2_DDR_LOWOCM
}

create_bd_addr_seg -range 0x20000000 -offset 0x00000000 \
                    [get_bd_addr_spaces maia_sdr/m_axi_spectrometer] \
                    [get_bd_addr_segs sys_ps7/S_AXI_HP1/HP1_DDR_LOWOCM] \
                    SEG_sys_ps7_HP1_DDR_LOWOCM


# interrupts
if {[info exists maia_iio]} {
	ad_cpu_interrupt ps-13 mb-13 axi_ad9361_adc_dma/irq
	ad_cpu_interrupt ps-12 mb-12 axi_ad9361_dac_dma/irq
	ad_cpu_interrupt ps-11 mb-11 maia_sdr/interrupt_out
} else {
	ad_cpu_interrupt ps-13 mb-13 maia_sdr/interrupt_out
}

if {[info exists maia_iio]} {

	# ======================= 8BITS RX OUT  ============================
	# I PART
	ad_ip_instance xlslice shiftslicei
	ad_ip_parameter shiftslicei CONFIG.DIN_WIDTH 16
	#MSB make a HIGH DC SPIKE...Try to use LSB
	ad_ip_parameter shiftslicei CONFIG.DIN_FROM 7
	ad_ip_parameter shiftslicei CONFIG.DIN_TO 0

	ad_ip_parameter shiftslicei CONFIG.DOUT_WIDTH 8

	ad_connect shiftslicei/Din axi_ad9361/adc_data_i0

	# Q PART
	ad_ip_instance xlslice shiftsliceq
	ad_ip_parameter shiftsliceq CONFIG.DIN_WIDTH 16
	ad_ip_parameter shiftsliceq CONFIG.DIN_FROM 7
	ad_ip_parameter shiftsliceq CONFIG.DIN_TO 0
	ad_ip_parameter shiftsliceq CONFIG.DOUT_WIDTH 8

	ad_connect shiftsliceq/Din axi_ad9361/adc_data_q0

	#IQ combine 
	ad_ip_instance xlconcat concatslice_iq
	ad_connect concatslice_iq/In0 shiftslicei/Dout
	ad_connect concatslice_iq/In1 shiftsliceq/Dout

	#Mux select CS8
	#Select input depending on qo_enable
	ad_ip_instance util_vector_logic logic_no_q0 [list \
	  C_OPERATION {not} \
	  C_SIZE 1]
	ad_connect axi_ad9361/adc_enable_q0 logic_no_q0/Op1

	#ad_ip_instance ad_bus_mux muxcs8 -> DOESNT WORK , USE create_bd_cell instead
	add_files -norecurse  ../../adi-hdl/library/common/ad_bus_mux.v
	create_bd_cell -type module -reference ad_bus_mux muxcs8
	ad_connect muxcs8/select_path logic_no_q0/Res

	#First input CS16 - > I0 -> I0
	ad_connect axi_ad9361/adc_data_i0 muxcs8/data_in_0
	ad_connect axi_ad9361/adc_valid_i0 muxcs8/valid_in_0
	ad_connect axi_ad9361/adc_enable_q0 muxcs8/enable_in_0

	#Second input CS8 - > I0+Q0
	ad_connect concatslice_iq/Dout muxcs8/data_in_1
	ad_connect axi_ad9361/adc_valid_i0 muxcs8/valid_in_1
	ad_connect GND muxcs8/enable_in_1

	#OUT
	ad_connect muxcs8/valid_out cpack/fifo_wr_en
	ad_connect muxcs8/data_out cpack/fifo_wr_data_0
	ad_connect muxcs8/enable_out cpack/enable_1

	# ======================= 8BITS TX OUT  ============================
	# I PART
	ad_ip_instance xlslice shiftsliceitx
	ad_ip_parameter shiftsliceitx CONFIG.DIN_WIDTH 16
	ad_ip_parameter shiftsliceitx CONFIG.DIN_FROM 15
	ad_ip_parameter shiftsliceitx CONFIG.DIN_TO 8
	ad_ip_parameter shiftsliceitx CONFIG.DOUT_WIDTH 8
	ad_connect tx_upack/fifo_rd_data_0 shiftsliceitx/Din

	ad_ip_instance xlconcat concatslicetx_i

	ad_ip_parameter concatslicetx_i CONFIG.NUM_PORTS 3
	ad_ip_parameter concatslicetx_i CONFIG.IN0_WIDTH 4
	ad_ip_parameter concatslicetx_i CONFIG.IN1_WIDTH 8
	ad_ip_parameter concatslicetx_i CONFIG.IN2_WIDTH 4

	ad_connect shiftsliceitx/Dout concatslicetx_i/In1

	# Q PART
	ad_ip_instance xlslice shiftsliceqtx
	ad_ip_parameter shiftsliceqtx CONFIG.DIN_WIDTH 16
	ad_ip_parameter shiftsliceqtx CONFIG.DIN_FROM 7
	ad_ip_parameter shiftsliceqtx CONFIG.DIN_TO 0
	ad_ip_parameter shiftsliceqtx CONFIG.DOUT_WIDTH 8
	ad_connect tx_upack/fifo_rd_data_0 shiftsliceqtx/Din

	ad_ip_instance xlconcat concatslicetx_q
	ad_ip_parameter concatslicetx_q CONFIG.NUM_PORTS 3
	ad_ip_parameter concatslicetx_q CONFIG.IN0_WIDTH 4
	ad_ip_parameter concatslicetx_q CONFIG.IN1_WIDTH 8
	ad_ip_parameter concatslicetx_q CONFIG.IN2_WIDTH 4

	ad_connect shiftsliceqtx/Dout concatslicetx_q/In1

	ad_ip_instance util_vector_logic logic_no_q0_tx [list \
	  C_OPERATION {not} \
	  C_SIZE 1]
	ad_connect axi_ad9361/dac_enable_q0 logic_no_q0_tx/Op1

	#Select input depending on dac_qo_enable
	# *****  I PART **********

	create_bd_cell -type module -reference ad_bus_mux muxcs8_tx_i

	ad_connect muxcs8_tx_i/select_path logic_no_q0_tx/Res
	ad_connect muxcs8_tx_i/enable_in_0 axi_ad9361/dac_enable_i0
	#First input CS16 - > I0 -> I0
	ad_connect tx_upack/fifo_rd_data_0 muxcs8_tx_i/data_in_0
	#Second input C8 - > CS16 > I0
	ad_connect concatslicetx_i/Dout muxcs8_tx_i/data_in_1
	ad_connect muxcs8_tx_i/enable_in_1 axi_ad9361/dac_enable_i0

	#OUT
	ad_connect muxcs8_tx_i/data_out axi_ad9361/dac_data_i0
	ad_connect muxcs8_tx_i/enable_out tx_upack/enable_0

	#Select input depending on dac_qo_enable
	# *****  Q PART **********

	create_bd_cell -type module -reference ad_bus_mux muxcs8_tx_q

	ad_connect muxcs8_tx_q/select_path logic_no_q0_tx/Res
	ad_connect muxcs8_tx_q/enable_in_0 axi_ad9361/dac_enable_q0
	#First input CS16 - > I0 -> I0
	ad_connect tx_upack/fifo_rd_data_1 muxcs8_tx_q/data_in_0
	#Second input C8 - > CS16 > I0
	ad_connect concatslicetx_q/Dout muxcs8_tx_q/data_in_1
	ad_connect muxcs8_tx_q/enable_in_1 axi_ad9361/dac_enable_i0

	#OUT
	ad_connect muxcs8_tx_q/data_out axi_ad9361/dac_data_q0
	ad_connect muxcs8_tx_q/enable_out tx_upack/enable_1

	# ******************************************************************
	#                       2ND CHANNEL 
	#

	# ======================= 8BITS RX2 OUT  ============================
	# I PART
	ad_ip_instance xlslice shiftslicei2
	ad_ip_parameter shiftslicei2 CONFIG.DIN_WIDTH 16
	#MSB make a HIGH DC SPIKE...Try to use LSB
	ad_ip_parameter shiftslicei2 CONFIG.DIN_FROM 7
	ad_ip_parameter shiftslicei2 CONFIG.DIN_TO 0

	ad_ip_parameter shiftslicei2 CONFIG.DOUT_WIDTH 8

	ad_connect shiftslicei2/Din axi_ad9361/adc_data_i1

	# Q PART
	ad_ip_instance xlslice shiftsliceq2
	ad_ip_parameter shiftsliceq2 CONFIG.DIN_WIDTH 16
	ad_ip_parameter shiftsliceq2 CONFIG.DIN_FROM 7
	ad_ip_parameter shiftsliceq2 CONFIG.DIN_TO 0
	ad_ip_parameter shiftsliceq2 CONFIG.DOUT_WIDTH 8

	ad_connect shiftsliceq2/Din axi_ad9361/adc_data_q1

	#IQ combine 
	ad_ip_instance xlconcat concatslice_iq2
	ad_connect concatslice_iq2/In0 shiftslicei2/Dout
	ad_connect concatslice_iq2/In1 shiftsliceq2/Dout

	#Mux select CS8
	#Select input depending on qo_enable
	ad_ip_instance util_vector_logic logic_no_q02 [list \
	  C_OPERATION {not} \
	  C_SIZE 1]
	ad_connect axi_ad9361/adc_enable_q0 logic_no_q02/Op1

	#ad_ip_instance ad_bus_mux muxcs8 -> DOESNT WORK , USE create_bd_cell instead
	create_bd_cell -type module -reference ad_bus_mux muxcs82

	ad_connect muxcs82/select_path logic_no_q02/Res
	#First input CS16 - > I0 -> I0
	ad_connect axi_ad9361/adc_data_i1 muxcs82/data_in_0
	ad_connect axi_ad9361/adc_valid_i1 muxcs82/valid_in_0
	ad_connect axi_ad9361/adc_enable_q1 muxcs82/enable_in_0
	#Second input CS8 - > I0+Q0
	ad_connect concatslice_iq2/Dout muxcs82/data_in_1
	ad_connect axi_ad9361/adc_valid_i1 muxcs82/valid_in_1
	ad_connect GND muxcs82/enable_in_1

	#OUT
	#ad_connect muxcs82/valid_out cpack/fifo_wr_en
	ad_connect muxcs82/data_out cpack/fifo_wr_data_2
	ad_connect muxcs82/enable_out cpack/enable_3

	# ======================= 8BITS TX2 OUT  ============================
	# I PART
	ad_ip_instance xlslice shiftsliceitx2
	ad_ip_parameter shiftsliceitx2 CONFIG.DIN_WIDTH 16
	ad_ip_parameter shiftsliceitx2 CONFIG.DIN_FROM 15
	ad_ip_parameter shiftsliceitx2 CONFIG.DIN_TO 8
	ad_ip_parameter shiftsliceitx2 CONFIG.DOUT_WIDTH 8
	ad_connect tx_upack/fifo_rd_data_2 shiftsliceitx2/Din

	ad_ip_instance xlconcat concatslicetx_i2

	ad_ip_parameter concatslicetx_i2 CONFIG.NUM_PORTS 3
	ad_ip_parameter concatslicetx_i2 CONFIG.IN0_WIDTH 4
	ad_ip_parameter concatslicetx_i2 CONFIG.IN1_WIDTH 8
	ad_ip_parameter concatslicetx_i2 CONFIG.IN2_WIDTH 4

	ad_connect shiftsliceitx2/Dout concatslicetx_i2/In1

	# Q PART
	ad_ip_instance xlslice shiftsliceqtx2
	ad_ip_parameter shiftsliceqtx2 CONFIG.DIN_WIDTH 16
	ad_ip_parameter shiftsliceqtx2 CONFIG.DIN_FROM 7
	ad_ip_parameter shiftsliceqtx2 CONFIG.DIN_TO 0
	ad_ip_parameter shiftsliceqtx2 CONFIG.DOUT_WIDTH 8
	ad_connect tx_upack/fifo_rd_data_2 shiftsliceqtx2/Din

	ad_ip_instance xlconcat concatslicetx_q2
	ad_ip_parameter concatslicetx_q2 CONFIG.NUM_PORTS 3
	ad_ip_parameter concatslicetx_q2 CONFIG.IN0_WIDTH 4
	ad_ip_parameter concatslicetx_q2 CONFIG.IN1_WIDTH 8
	ad_ip_parameter concatslicetx_q2 CONFIG.IN2_WIDTH 4

	ad_connect shiftsliceqtx2/Dout concatslicetx_q2/In1

	ad_ip_instance util_vector_logic logic_no_q0_tx2 [list \
	  C_OPERATION {not} \
	  C_SIZE 1]
	ad_connect axi_ad9361/dac_enable_q1 logic_no_q0_tx2/Op1


	#Select input depending on dac_qo_enable
	# *****  I PART **********
	create_bd_cell -type module -reference ad_bus_mux muxcs8_tx_i2

	ad_connect muxcs8_tx_i2/select_path logic_no_q0_tx2/Res
	ad_connect muxcs8_tx_i2/enable_in_0 axi_ad9361/dac_enable_i1
	#First input CS16 - > I0 -> I0
	ad_connect tx_upack/fifo_rd_data_2 muxcs8_tx_i2/data_in_0
	#Second input C8 - > CS16 > I0
	ad_connect concatslicetx_i2/Dout muxcs8_tx_i2/data_in_1
	ad_connect muxcs8_tx_i2/enable_in_1 axi_ad9361/dac_enable_i1

	#OUT
	ad_connect muxcs8_tx_i2/data_out axi_ad9361/dac_data_i1
	ad_connect muxcs8_tx_i2/enable_out tx_upack/enable_2

	#Select input depending on dac_qo_enable
	# *****  Q PART **********
	create_bd_cell -type module -reference ad_bus_mux muxcs8_tx_q2

	ad_connect muxcs8_tx_q2/select_path logic_no_q0_tx2/Res
	ad_connect muxcs8_tx_q2/enable_in_0 axi_ad9361/dac_enable_q1
	#First input CS16 - > I0 -> I0
	ad_connect tx_upack/fifo_rd_data_3 muxcs8_tx_q2/data_in_0
	#Second input C8 - > CS16 > I0
	ad_connect concatslicetx_q2/Dout muxcs8_tx_q2/data_in_1
	ad_connect muxcs8_tx_q2/enable_in_1 axi_ad9361/dac_enable_i1

	#OUT
	ad_connect muxcs8_tx_q2/data_out axi_ad9361/dac_data_q1
	ad_connect muxcs8_tx_q2/enable_out tx_upack/enable_3
}

