//! DDC design routines.
//!
//! This module contains code used to design FIR filters for the DDC. The design
//! is done using the Parks-McClellan algorithm with the [pm-remez](mod@pm_remez)
//! crate.

use anyhow::Result;
use pm_remez::{
    BandSetting, PMDesign, constant, linear, order_estimates::ichige, pm_parameters, pm_remez,
};

pub mod constants;

#[derive(Debug, Copy, Clone, PartialEq)]
struct Config {
    // transition bandwidth of the output
    delta_f: f64,
    // passband ripple
    delta_p: f64,
    // stopband ripple
    delta_s: f64,
    // 1/f stopband
    one_over_f: bool,
}

impl Config {
    fn from_ddc_design(design: &maia_json::PutDDCDesign) -> Config {
        Config {
            delta_f: design.transition_bandwidth.unwrap_or(0.05),
            delta_p: design.passband_ripple.unwrap_or(0.01),
            delta_s: design
                .stopband_attenuation_db
                .map_or(0.001, |db| 10.0f64.powf(-db / 20.0)),
            one_over_f: design.stopband_one_over_f.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DecimatorConfig<T> {
    fir1: FIRConfig<T>,
    fir2: Option<FIRConfig<T>>,
    fir3: Option<FIRConfig<T>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FIRConfig<T> {
    coefficients: Vec<T>,
    decimation: usize,
}

impl DecimatorConfig<f64> {
    fn quantize(&self) -> DecimatorConfig<i32> {
        fn max_abs(coeffs: &[f64]) -> f64 {
            coeffs
                .iter()
                .map(|x| x.abs())
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap()
        }

        fn sum_abs(coeffs: &[f64]) -> f64 {
            coeffs.iter().map(|x| x.abs()).sum::<f64>()
        }

        fn apply_scale(coeffs: &[f64], scale: f64) -> Vec<f64> {
            coeffs.iter().map(|&x| (x * scale).round()).collect()
        }

        fn to_i32(coeffs: &[f64]) -> Vec<i32> {
            coeffs.iter().map(|&x| x as i32).collect()
        }

        const MAX_COEFF: i32 = (1 << (constants::COEFFICIENT_BITS - 1)) - 1;
        const STAGE_GROWTH1: u16 = constants::MACC_TRUNC[0] + constants::WIDTH_GROWTH[0];
        const STAGE_GROWTH2: u16 = constants::MACC_TRUNC[1] + constants::WIDTH_GROWTH[1];
        const STAGE_GROWTH3: u16 = constants::MACC_TRUNC[2] + constants::WIDTH_GROWTH[2];

        let h1 = &self.fir1.coefficients;
        let h1_max_scale = MAX_COEFF as f64 / max_abs(h1);
        let h1_scale_desired = (1u64 << STAGE_GROWTH1) as f64 / sum_abs(h1);
        let h1_scale = h1_scale_desired.min(h1_max_scale);
        let h1 = apply_scale(h1, h1_scale);
        let d1 = self.fir1.decimation;
        let fir1 = FIRConfig {
            coefficients: to_i32(&h1),
            decimation: d1,
        };

        let (h2, d2, fir2) = if let Some(fir2) = &self.fir2 {
            let h2 = &fir2.coefficients;
            let h2_max_scale = MAX_COEFF as f64 / max_abs(h2);
            let h1h2 = convolve(&h1, &zero_pack(h2, d1));
            let h2_scale_desired =
                (1u64 << (STAGE_GROWTH1 + STAGE_GROWTH2)) as f64 / sum_abs(&h1h2);
            let h2_scale = h2_scale_desired.min(h2_max_scale);
            let h2 = apply_scale(h2, h2_scale);
            let d2 = fir2.decimation;
            let fir2 = Some(FIRConfig {
                coefficients: to_i32(&h2),
                decimation: d2,
            });
            (h2, d2, fir2)
        } else {
            // fake a bypass
            let h2 = vec![(1 << STAGE_GROWTH2) as f64];
            (h2, 1, None)
        };

        let fir3 = self.fir3.as_ref().map(|fir3| {
            let h3 = &fir3.coefficients;
            let h3_max_scale = MAX_COEFF as f64 / max_abs(h3);
            let h1h2h3 = convolve(&convolve(&h1, &zero_pack(&h2, d1)), &zero_pack(h3, d1 * d2));
            let h3_scale_desired =
                (1u64 << (STAGE_GROWTH1 + STAGE_GROWTH2 + STAGE_GROWTH3)) as f64 / sum_abs(&h1h2h3);
            let h3_scale = h3_scale_desired.min(h3_max_scale);
            let h3 = apply_scale(h3, h3_scale);
            FIRConfig {
                coefficients: to_i32(&h3),
                decimation: fir3.decimation,
            }
        });

        DecimatorConfig { fir1, fir2, fir3 }
    }
}

impl DecimatorConfig<i32> {
    fn into_json(self, frequency: f64) -> maia_json::PutDDCConfig {
        maia_json::PutDDCConfig {
            frequency,
            fir1: self.fir1.into(),
            fir2: self.fir2.map(|f| f.into()),
            fir3: self.fir3.map(|f| f.into()),
        }
    }
}

impl From<FIRConfig<i32>> for maia_json::DDCFIRConfig {
    fn from(config: FIRConfig<i32>) -> maia_json::DDCFIRConfig {
        maia_json::DDCFIRConfig {
            coefficients: config.coefficients,
            decimation: u32::try_from(config.decimation).unwrap(),
        }
    }
}

/// Calculates a DDC design according to some requirements.
///
/// The `design` parameter gives the DDC requirements, such as the decimation
/// rate, and some optional parameters such as the required transition bandwidth
/// and stopband attenuation. The function calculates and returns a DDC
/// configuration that satisfies this requirements and can run with the input
/// sample rate given in the `input_samp_rate` parameter. The function returns
/// an error if the required FIR filters are too long to be realized in the
/// FPGA.
///
/// The following defaults are used for values in `design` that are not
/// specified:
/// - Transition bandwidth: 0.05.
/// - Passband ripple: 0.01.
/// - Stopband attenuation: 60 dB.
/// - Stopband 1/f response: enabled.
pub fn make_design(
    design: &maia_json::PutDDCDesign,
    input_samp_rate: f64,
) -> Result<maia_json::PutDDCConfig> {
    Ok(stages_design(
        usize::try_from(design.decimation).unwrap(),
        input_samp_rate,
        &Config::from_ddc_design(design),
    )?
    .quantize()
    .into_json(design.frequency))
}

fn stages_design(d: usize, input_samp_rate: f64, config: &Config) -> Result<DecimatorConfig<f64>> {
    // Iterator that splits decimation factor d in vectors of up to 3 factors in
    // non-increasing order. Also impose FPGA implementation constraint on max
    // decimation factor per stage.
    let splits = (2..=d.min(constants::MAX_DECIMATION))
        .filter(|&d1| d.is_multiple_of(d1))
        .flat_map(|d1| {
            (1..=(d / d1).min(constants::MAX_DECIMATION))
                .filter(move |&d2| d.is_multiple_of(d1 * d2))
                .filter_map(move |d2| {
                    if d2 > d1 {
                        return None;
                    }
                    let d3 = d / (d1 * d2);
                    if d3 > d2 || d3 > constants::MAX_DECIMATION {
                        return None;
                    }
                    let mut ds = vec![d1];
                    if d2 > 1 {
                        ds.push(d2);
                    }
                    if d3 > 1 {
                        debug_assert!(d2 > 1);
                        ds.push(d3);
                    }
                    Some(ds)
                })
        });

    let Some(best_split) = splits.min_by(|a, b| {
        split_cost_estimate(a, d, input_samp_rate, config)
            .partial_cmp(&split_cost_estimate(b, d, input_samp_rate, config))
            .unwrap()
    }) else {
        // This should only happen if d is very large and cannot be factored
        // into factors smaller than MAX_DECIMATION.
        anyhow::bail!("decimation factor {d} too large");
    };
    split_design(&best_split, d, input_samp_rate, config)
}

fn split_cost_estimate(split: &[usize], d: usize, input_samp_rate: f64, config: &Config) -> f64 {
    assert!((1..=3).contains(&split.len()));
    const THRESHOLD: f64 = 1.1;
    let fp = 0.5 * (1.0 - config.delta_f);

    let d1 = split[0];
    let n1 = pm_estimate(d as f64, fp, d1, config);
    let n1_max = stage_max_coefficients(input_samp_rate, d1, true);
    if n1 as f64 > n1_max as f64 * THRESHOLD {
        // filter is most likely not realizable by FPGA
        return f64::INFINITY;
    }
    let c1 = n1 as f64 / d1 as f64; // multiplies per input

    if split.len() < 2 {
        return c1;
    }

    let d2 = split[1];
    let n2 = pm_estimate((d / d1) as f64, fp, d2, config);
    // If there are only 2 decimation stages, use the first and third stage,
    // both of which have 4 DSPs. If there are 3 decimation stages, we need to
    // use the second stage, which has only 2 DSPs.
    let four_dsp = split.len() == 2;
    let n2_max = stage_max_coefficients(input_samp_rate / d1 as f64, d2, four_dsp);
    if n2 as f64 > n2_max as f64 * THRESHOLD {
        // filter is most likely not realizable by FPGA
        return f64::INFINITY;
    }
    let c2 = n2 as f64 / (d1 * d2) as f64;

    if split.len() < 3 {
        return c1 + c2;
    }

    let d3 = split[2];
    let n3 = pm_estimate(d3 as f64, fp, d3, config);
    let n3_max = stage_max_coefficients(input_samp_rate / (d1 * d2) as f64, d3, true);
    if n3 as f64 > n3_max as f64 * THRESHOLD {
        // filter is most likely not realizable by FPGA
        return f64::INFINITY;
    }
    let c3 = n3 as f64 / d as f64;

    c1 + c2 + c3
}

fn split_design(
    split: &[usize],
    d: usize,
    input_samp_rate: f64,
    config: &Config,
) -> Result<DecimatorConfig<f64>> {
    assert!((1..=3).contains(&split.len()));
    let fp = 0.5 * (1.0 - config.delta_f);

    let d1 = split[0];
    let n1_max = stage_max_coefficients(input_samp_rate, d1, true);
    let design1 = pm_design(d as f64, fp, d1, config, n1_max)?;
    let fir1 = FIRConfig {
        coefficients: design1.impulse_response,
        decimation: d1,
    };

    if split.len() < 2 {
        return Ok(DecimatorConfig {
            fir1,
            fir2: None,
            fir3: None,
        });
    }

    let d2 = split[1];
    let four_dsp = split.len() == 2;
    let n2_max = stage_max_coefficients(input_samp_rate / d1 as f64, d2, four_dsp);
    let design2 = pm_design((d / d1) as f64, fp, d2, config, n2_max)?;
    let fir2 = FIRConfig {
        coefficients: design2.impulse_response,
        decimation: d2,
    };

    if split.len() < 3 {
        return Ok(DecimatorConfig {
            fir1,
            // with only 2 stages, the 2nd stage is bypassed and the third stage
            // is used as the second
            fir2: None,
            fir3: Some(fir2),
        });
    }

    let d3 = split[2];
    let n3_max = stage_max_coefficients(input_samp_rate / (d1 * d2) as f64, d3, true);
    let design3 = pm_design(d3 as f64, fp, d3, config, n3_max)?;
    let fir3 = FIRConfig {
        coefficients: design3.impulse_response,
        decimation: d3,
    };

    Ok(DecimatorConfig {
        fir1,
        fir2: Some(fir2),
        fir3: Some(fir3),
    })
}

fn stage_max_coefficients(input_samp_rate: f64, decimation: usize, four_dsp: bool) -> usize {
    let clocks_per_input = (constants::CLOCK_FREQUENCY / input_samp_rate).floor() as usize;
    let coefficients_per_clock = if four_dsp { 2 } else { 1 };
    let max_operations = clocks_per_input.min(constants::MAX_OPERATIONS);
    let max_coeffs = max_operations * coefficients_per_clock * decimation;
    let max_coeffs_ram = if four_dsp {
        constants::MAX_COEFFICIENTS_4DSP
    } else {
        constants::MAX_COEFFICIENTS_2DSP
    };
    // The number of coefficients actually stored in the RAM must be divisible
    // by the decimation factor.
    let max_coeffs_ram = (max_coeffs_ram / decimation) * decimation;
    max_coeffs.min(max_coeffs_ram)
}

fn pm_design(
    fs: f64,
    fp: f64,
    d: usize,
    config: &Config,
    max_taps: usize,
) -> Result<PMDesign<f64>> {
    let passband_end = fp / fs;
    let stopband_start = 1.0 / d as f64 - passband_end;
    let stopband_weight = config.delta_p / config.delta_s;
    let stopband_weight = if config.one_over_f {
        linear(stopband_weight, stopband_weight * 0.5 / stopband_start)
    } else {
        constant(stopband_weight)
    };
    let bands = [
        BandSetting::new(0.0, passband_end, constant(1.0)).unwrap(),
        BandSetting::with_weight(stopband_start, 0.5, constant(0.0), stopband_weight).unwrap(),
    ];

    // Initial estimate for number of taps
    let mut num_taps = ichige(
        passband_end,
        stopband_start - passband_end,
        config.delta_p,
        config.delta_s,
    );

    let parameters = pm_parameters(num_taps, &bands).unwrap();
    let mut design = pm_remez(&parameters)?;

    if design.weighted_error < config.delta_p {
        // Initial estimate was an overestimate. Back off the number of taps
        // until we no longer meet the estimate.
        loop {
            num_taps -= 1;
            let parameters = pm_parameters(num_taps, &bands).unwrap();
            let new_design = pm_remez(&parameters)?;
            if new_design.weighted_error > config.delta_p {
                return Ok(design);
            }
            design = new_design;
        }
    } else {
        // Initial estimate was an underestimate. Increase the number of taps
        // until the estimate is met.
        while design.weighted_error > config.delta_p {
            num_taps += 1;
            if num_taps > max_taps {
                anyhow::bail!("FIR filter would need more taps than is realizable by FPGA");
            }
            let parameters = pm_parameters(num_taps, &bands).unwrap();
            design = pm_remez(&parameters)?;
        }
        Ok(design)
    }
}

fn pm_estimate(fs: f64, fp: f64, d: usize, config: &Config) -> usize {
    let passband_end = fp / fs;
    let stopband_start = 1.0 / d as f64 - passband_end;
    ichige(
        passband_end,
        stopband_start - passband_end,
        config.delta_p,
        config.delta_s,
    )
}

fn zero_pack<T: Default + Clone>(x: &[T], n: usize) -> Vec<T> {
    let mut out = vec![T::default(); x.len() * n];
    for (dst, src) in out.iter_mut().step_by(n).zip(x.iter()) {
        dst.clone_from(src);
    }
    out
}

// full convolution of two vectors
fn convolve(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len();
    let m = y.len();
    let conv_size = n + m - 1;
    (0..conv_size)
        .map(|j| {
            let a = if j >= m { j - m + 1 } else { 0 };
            let b = j.min(n - 1);
            let c = if j >= n { j - n + 1 } else { 0 };
            let d = j.min(m - 1);
            debug_assert!(b >= a);
            debug_assert!(d >= c);
            debug_assert_eq!(b - a, d - c);
            x[a..=b]
                .iter()
                .zip(y[c..=d].iter().rev())
                .map(|(&u, &v)| u * v)
                .sum::<f64>()
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    fn example_config() -> Config {
        Config {
            delta_f: 0.05,
            delta_p: 0.01,
            delta_s: 0.001,
            one_over_f: true,
        }
    }

    #[test]
    fn stages() {
        let config = example_config();
        let stages = stages_design(1280, 61.44e6, &config).unwrap();
        let stages_quant = stages.quantize();
        assert_eq!(stages_quant.fir1.decimation, 32);
        assert_eq!(stages_quant.fir2.unwrap().decimation, 20);
        assert_eq!(stages_quant.fir3.unwrap().decimation, 2);
    }

    #[test]
    fn pm_design_example() {
        let config = example_config();
        let estimate = pm_estimate(1.0, 0.1, 4, &config);
        assert_eq!(estimate, 54);
        let design = pm_design(1.0, 0.1, 4, &config, 57).unwrap();
        assert_eq!(design.impulse_response.len(), 57);
        assert!(design.weighted_error <= config.delta_p);
        // cannot design with 56 coefficients max
        assert!(pm_design(1.0, 0.1, 4, &config, 56).is_err());
    }

    #[test]
    fn ichige() {
        assert_eq!(super::ichige(0.1, 0.05, 0.01, 0.001), 54);
        assert_eq!(super::ichige(0.05, 0.05, 0.01, 0.001), 55);
        assert_eq!(super::ichige(0.025, 0.05, 0.01, 0.001), 57);
        assert_eq!(super::ichige(0.1, 0.1, 0.01, 0.001), 28);
        assert_eq!(super::ichige(0.01, 0.01, 0.01, 0.001), 271);
    }

    #[test]
    fn zero_pack() {
        assert_eq!(
            super::zero_pack::<f64>(&[1.0, 2.0, 3.0], 2),
            vec![1.0, 0.0, 2.0, 0.0, 3.0, 0.0]
        );
    }

    #[test]
    fn convolve() {
        let v = [1.0, 2.0, 3.0];
        let expected = [1.0, 4.0, 10.0, 12.0, 9.0];
        assert_eq!(&super::convolve(&v, &v), &expected);
    }
}
