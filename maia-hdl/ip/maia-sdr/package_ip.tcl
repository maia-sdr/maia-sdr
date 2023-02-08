# Package Maia SDR IP core

create_project maia_sdr . -force
add_files maia_sdr.v
add_files -fileset constrs_1 -norecurse maia_sdr.xdc
set_property top top [current_fileset]
load_features ipservices
ipx::package_project -import_files -root_dir . -vendor destevez.net -library user -taxonomy /Maia-SDR -force
set_property name maia_sdr [ipx::current_core]
set_property library maia_sdr [ipx::current_core]
set_property display_name {Maia SDR} [ipx::current_core]
set_property description {Maia SDR} [ipx::current_core]
set_property vendor_display_name {Daniel Estevez} [ipx::current_core]
set_property company_url {https://destevez.net} [ipx::current_core]
set_property version $::env(IP_CORE_VERSION) [ipx::current_core]

# sampling_clk interface
ipx::add_bus_interface sampling_clk [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:clock_rtl:1.0 \
    [ipx::get_bus_interfaces sampling_clk -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:clock:1.0 \
    [ipx::get_bus_interfaces sampling_clk -of_objects [ipx::current_core]]
ipx::add_bus_parameter FREQ_HZ [ipx::get_bus_interfaces sampling_clk -of_objects [ipx::current_core]]

# clk interface
ipx::add_bus_interface clk [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:clock_rtl:1.0 \
    [ipx::get_bus_interfaces clk -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:clock:1.0 \
    [ipx::get_bus_interfaces clk -of_objects [ipx::current_core]]
ipx::add_bus_parameter FREQ_HZ [ipx::get_bus_interfaces clk -of_objects [ipx::current_core]]
ipx::add_port_map CLK [ipx::get_bus_interfaces clk -of_objects [ipx::current_core]]
set_property physical_name clk [ipx::get_port_maps CLK \
                                    -of_objects [ipx::get_bus_interfaces clk -of_objects [ipx::current_core]]]

# clk2x_clk interface
ipx::add_bus_interface clk2x_clk [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:clock_rtl:1.0 \
    [ipx::get_bus_interfaces clk2x_clk -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:clock:1.0 \
    [ipx::get_bus_interfaces clk2x_clk -of_objects [ipx::current_core]]
ipx::add_bus_parameter FREQ_HZ [ipx::get_bus_interfaces clk2x_clk -of_objects [ipx::current_core]]

# clk3x_clk interface
ipx::add_bus_interface clk3x_clk [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:clock_rtl:1.0 \
    [ipx::get_bus_interfaces clk3x_clk -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:clock:1.0 \
    [ipx::get_bus_interfaces clk3x_clk -of_objects [ipx::current_core]]
ipx::add_bus_parameter FREQ_HZ [ipx::get_bus_interfaces clk3x_clk -of_objects [ipx::current_core]]

# rst output
ipx::add_bus_parameter POLARITY [ipx::get_bus_interfaces rst -of_objects [ipx::current_core]]
set_property value ACTIVE_HIGH \
    [ipx::get_bus_parameters POLARITY -of_objects \
         [ipx::get_bus_interfaces rst -of_objects [ipx::current_core]]]

# s_axi_lite_clk interface
ipx::add_bus_interface s_axi_lite_clk [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:clock_rtl:1.0 \
    [ipx::get_bus_interfaces s_axi_lite_clk -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:clock:1.0 \
    [ipx::get_bus_interfaces s_axi_lite_clk -of_objects [ipx::current_core]]
ipx::add_bus_parameter FREQ_HZ [ipx::get_bus_interfaces s_axi_lite_clk -of_objects [ipx::current_core]]

# s_axi_lite_rst interface
ipx::add_bus_interface s_axi_lite_rst [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:reset_rtl:1.0 \
    [ipx::get_bus_interfaces s_axi_lite_rst -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:reset:1.0 \
    [ipx::get_bus_interfaces s_axi_lite_rst -of_objects [ipx::current_core]]
ipx::add_bus_parameter POLARITY [ipx::get_bus_interfaces s_axi_lite_rst -of_objects [ipx::current_core]]
set_property value ACTIVE_HIGH [ipx::get_bus_parameters POLARITY -of_objects [ipx::get_bus_interfaces s_axi_lite_rst -of_objects [ipx::current_core]]]

# associate s_axi_lite, m_axi_recorder to s_axi_lite_clk
ipx::associate_bus_interfaces -busif s_axi_lite -clock clk -remove [ipx::current_core]
ipx::associate_bus_interfaces -busif s_axi_lite -clock s_axi_lite_clk [ipx::current_core]
ipx::associate_bus_interfaces -busif m_axi_recorder -clock clk -remove [ipx::current_core]
ipx::associate_bus_interfaces -busif m_axi_recorder -clock s_axi_lite_clk [ipx::current_core]

# interrupt
ipx::add_bus_interface interrupt [ipx::current_core]
set_property abstraction_type_vlnv xilinx.com:signal:interrupt_rtl:1.0 \
    [ipx::get_bus_interfaces interrupt -of_objects [ipx::current_core]]
set_property bus_type_vlnv xilinx.com:signal:interrupt:1.0 \
    [ipx::get_bus_interfaces interrupt -of_objects [ipx::current_core]]
set_property interface_mode master \
    [ipx::get_bus_interfaces interrupt -of_objects [ipx::current_core]]
ipx::add_port_map INTERRUPT \
    [ipx::get_bus_interfaces interrupt -of_objects [ipx::current_core]]
set_property physical_name interrupt_out \
    [ipx::get_port_maps INTERRUPT \
         -of_objects [ipx::get_bus_interfaces interrupt -of_objects [ipx::current_core]]]

ipx::create_xgui_files [ipx::current_core]
ipx::save_core [ipx::current_core]
