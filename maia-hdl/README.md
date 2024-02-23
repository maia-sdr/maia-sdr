# maia-hdl

maia-hdl is the Maia SDR FPGA Amaranth HDL code. See
[maia-sdr.org](https://maia-sdr.org/) for a general introduction to the project,
and also the [maia-sdr/maia-sdr](https://github.com/maia-sdr/maia-sdr) Github
repository.

The maia-hdl Python package can be used as a library of HDL modules in
third-party HDL designs.

## Installation

It is not necessary to install maia-hdl in order to generate the Vivado IP cores
or projects, or to run the tests and simulations. These can be run from a
checkout of the source code as indicated below.

When using maia-hdl as a library in third-party packages, it can be installed
with
```
pip install maia-hdl
```
or with any other method for managing Python packages.

## Building the Vivado IP cores and projects

The build system for the Vivado IP cores and projects is based on Makefiles. The
IP cores can be built by running `make` from the `ip` directory. A specific IP
core can be built by running `make` from its subdirectory inside the `ip`
directory.

The Vivado projects can be built by running `make` from the `projects`
directory. A specific project cna be built by running `make` from its
subdirectory inside the `projects` directory. This will create the Vivado
project and block diagram, run synthesis and place and route, and finally
generate the bitstream.

## Testing

Pure Amaranth tests can be run with
```
python3 -m unittest
```

Mixed Amaranth/Verilog tests use [cocotb](https://www.cocotb.org/) and a Verilog
simulator such as [Icarus Verilog](http://iverilog.icarus.com/). Verilog code is
generated from the Amaranth code, so the simulation involves only Verilog code
and the Python code running from cocotb. The test execution of cocotb is based
on Makefiles. Tests can be run by running `make` from the `test_cocotb`
directory. It is possible to check which tests failed with
```
grep --include=results.xml -r -e failure .
```

## License

Licensed under MIT license ([LICENSE-MIT](LICENSE-MIT) or
http://opensource.org/licenses/MIT).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, shall be licensed as above, without any
additional terms or conditions.
