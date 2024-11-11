VIVADO := vivado -mode batch -source
IP_CORE_VERSION := $(shell PYTHONPATH="../../:${PYTHONPATH}" python3 -m maia_hdl.maia_sdr_version)

configs = default maia_iio

all: $(configs)

$(configs):
	mkdir -p $@
	PYTHONPATH="../../:${PYTHONPATH}" python3 -m maia_hdl.maia_sdr --config $@ $@/maia_sdr.v
	cd $@ && IP_CORE_VERSION=$(IP_CORE_VERSION) MAIA_SDR_CONFIG=$@ $(VIVADO) ../package_ip.tcl

clean:
	rm -rf $(configs)

.PHONY: all clean $(configs)
