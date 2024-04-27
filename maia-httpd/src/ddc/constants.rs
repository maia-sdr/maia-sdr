//! FPGA DDC implementation constants.
//!
//! This module contains constants that define the characteristics of the DDC
//! implementation in the FPGA.

/// Number of bits used for the FIR filter coefficients.
pub const COEFFICIENT_BITS: u8 = 18;

/// Maximum decimation for a FIR filter stage.
///
/// This restriction is given by the width of the decimation register.
pub const MAX_DECIMATION: usize = (1 << 7) - 1;

/// Maximum number of "operations" for a FIR filter stage.
///
/// This restriction is given by the width of the operations register.
pub const MAX_OPERATIONS: usize = 1 << 7;

/// Maximum number of coefficients that can be stored in a FIR with 4 DSPs.
pub const MAX_COEFFICIENTS_4DSP: usize = 256;

/// Maximum number of coefficients that can be stored in a FIR with 2 DSPs.
pub const MAX_COEFFICIENTS_2DSP: usize = 128;

/// Clock frequency at which the DDC runs.
pub const CLOCK_FREQUENCY: f64 = 187.5e6;

/// Truncation of the multiply-accumulate (MACC) output in each of the FIR
/// stages of the DDC.
pub const MACC_TRUNC: [u16; 3] = [17, 18, 18];

/// Datapath word width growth in each of the FIR stages of the DDC.
pub const WIDTH_GROWTH: [u16; 3] = [4, 0, 0];
