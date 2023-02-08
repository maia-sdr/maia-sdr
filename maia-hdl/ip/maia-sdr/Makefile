VIVADO := vivado -mode batch -source
IP_CORE_VERSION := $(shell PYTHONPATH="../../:${PYTHONPATH}" python3 -m maia_hdl.maia_sdr_version)

.PHONY: all clean maia_sdr.v

all: component.xml

maia_sdr.v:
	PYTHONPATH="../../:${PYTHONPATH}" python3 -m maia_hdl.maia_sdr generate -t v > maia_sdr.v

component.xml: maia_sdr.v
	IP_CORE_VERSION=$(IP_CORE_VERSION) $(VIVADO) package_ip.tcl

clean:
	rm -rf *.jou *.log *.xpr *.srcs *.hw *.ip_user_files \
		*.cache *.runs *.sim *~ *.str component.xml \
		xgui src maia_sdr.v
