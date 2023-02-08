# maia-hdl

## Installation

It is not necessary to install maia-hdl in order to generate the Vivado IP cores
or projects, or to run the tests and simulations. These can be run as indicated
below.

It is possible to install maia-hdl as a Python package in order to be able to
use it (with `import maia_hdl`) in other Python code and projects. The latest
packaged version of maia-hdl can be installed with
```
pip install maia_hdl
```

The current checkout of this source tree can be installed with
```
pip install .
```

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
