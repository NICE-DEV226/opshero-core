//! Integer-only statistics for evidence about commands that may be flaky
//! (RFC-0005 §3).
//!
//! Everything here is exact fixed-point arithmetic on integers, so the result
//! is identical on every platform: no floating point, no libm, no clock, no
//! randomness. Rounding always errs on the conservative side (more runs
//! required, never fewer).

/// Fixed-point scale: probabilities are carried as `value / SCALE`.
const SCALE: u128 = 1_000_000_000_000;
/// Inputs are given in parts per million.
const PPM: u128 = 1_000_000;
/// Hard cap on the answer; beyond this the request is not meaningful.
pub const MAX_RUNS: u32 = 1_000_000;

/// Smallest `n` such that, if every one of `n` independent runs passed, a
/// command whose true failure probability is at least `p1` would have
/// produced that result with probability at most `alpha`:
///
/// `n = min { n : (1 - p1)^n <= alpha }`.
///
/// Equivalently: after `n` consecutive passes we may claim, at confidence
/// `1 - alpha`, that the failure rate is below `p1`. Both arguments are in
/// parts per million. Returns `None` for degenerate inputs (`p1 = 0`,
/// `p1 >= 1`, `alpha` outside `(0, 1)`) or when `n` would exceed [`MAX_RUNS`].
pub fn min_passes(alpha_ppm: u32, p1_ppm: u32) -> Option<u32> {
    let (alpha, p1) = (u128::from(alpha_ppm), u128::from(p1_ppm));
    if alpha == 0 || alpha >= PPM || p1 == 0 || p1 >= PPM {
        return None;
    }
    let q = SCALE - p1 * (SCALE / PPM); // 1 - p1, scaled
    let limit = alpha * (SCALE / PPM);
    let mut pow = SCALE; // (1 - p1)^0
    for n in 1..=MAX_RUNS {
        // Round up so the computed power never underestimates the true one.
        pow = (pow * q).div_ceil(SCALE);
        if pow <= limit {
            return Some(n);
        }
    }
    None
}

/// Probability, in parts per million rounded *up*, that `n` consecutive
/// passes occur although the true failure probability is `p1`:
/// `(1 - p1)^n`. Used by explanations ("a 5 % flaky command passes 29 times in
/// a row with probability 4.7 %").
pub fn all_pass_probability_ppm(n: u32, p1_ppm: u32) -> u32 {
    let p1 = u128::from(p1_ppm.min(PPM as u32));
    let q = SCALE - p1 * (SCALE / PPM);
    let mut pow = SCALE;
    for _ in 0..n {
        pow = (pow * q).div_ceil(SCALE);
        if pow == 0 {
            break;
        }
    }
    u32::try_from(pow.div_ceil(SCALE / PPM)).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Floating-point oracle, independent of the implementation under test.
    fn oracle(alpha: f64, p1: f64) -> u32 {
        let mut n = 0u32;
        let mut pow = 1.0f64;
        while pow > alpha {
            pow *= 1.0 - p1;
            n += 1;
        }
        n
    }

    #[test]
    fn known_values_from_the_literature() {
        // Rule of three family: 95 % confidence that the failure rate < 5 % needs 59 passes.
        assert_eq!(min_passes(50_000, 50_000), Some(59));
        // 95 % / 10 % -> 29 ; 99 % / 10 % -> 44 ; 99 % / 1 % -> 459 ; 95 % / 1 % -> 299.
        assert_eq!(min_passes(50_000, 100_000), Some(29));
        assert_eq!(min_passes(10_000, 100_000), Some(44));
        assert_eq!(min_passes(10_000, 10_000), Some(459));
        assert_eq!(min_passes(50_000, 10_000), Some(299));
    }

    #[test]
    fn exact_boundary_is_respected() {
        // (1 - 0.5)^1 = 0.5 <= 0.5: one run suffices when alpha equals the miss probability.
        assert_eq!(min_passes(500_000, 500_000), Some(1));
        // Just below: needs two.
        assert_eq!(min_passes(499_999, 500_000), Some(2));
    }

    #[test]
    fn degenerate_inputs_are_rejected() {
        assert_eq!(min_passes(0, 100_000), None);
        assert_eq!(min_passes(1_000_000, 100_000), None);
        assert_eq!(min_passes(50_000, 0), None);
        assert_eq!(min_passes(50_000, 1_000_000), None);
        // p1 = 1 ppm and alpha = 1 ppm would need ~13.8 million runs.
        assert_eq!(min_passes(1, 1), None);
    }

    #[test]
    fn explanation_probability_matches_the_threshold() {
        let n = min_passes(50_000, 100_000).unwrap();
        assert!(all_pass_probability_ppm(n, 100_000) <= 50_000);
        assert!(all_pass_probability_ppm(n - 1, 100_000) > 50_000);
        assert_eq!(all_pass_probability_ppm(0, 100_000), 1_000_000);
    }

    /// Exact integer oracle on a permille grid: (1000-p)^n <= alpha * 1000^(n-1).
    /// Fits in u128 for n <= 12. The implementation must never answer fewer
    /// runs than the exact answer (the conservative guarantee) and at most one more.
    #[test]
    fn never_fewer_runs_than_the_exact_answer() {
        let mut checked = 0u32;
        for p in 300u128..=900 {
            for a in 1u128..=500 {
                let mut exact = None;
                for n in 1u32..=12 {
                    if (1000 - p).pow(n) <= a * 1000u128.pow(n - 1) {
                        exact = Some(n);
                        break;
                    }
                }
                let Some(exact) = exact else { continue };
                let got = min_passes(
                    u32::try_from(a * 1000).unwrap(),
                    u32::try_from(p * 1000).unwrap(),
                )
                .unwrap();
                assert!(
                    got >= exact,
                    "alpha={a}‰ p1={p}‰: got {got} < exact {exact}"
                );
                assert!(
                    got <= exact + 1,
                    "alpha={a}‰ p1={p}‰: got {got} > exact {exact} + 1"
                );
                checked += 1;
            }
        }
        assert!(
            checked > 100_000,
            "grid too small to be meaningful: {checked}"
        );
    }

    proptest! {
        /// Agreement with the float oracle: never fewer runs than the exact
        /// answer (conservative), and at most one more (rounding slack).
        #[test]
        fn conservative_and_tight_against_the_oracle(
            alpha in 1_000u32..=500_000,
            p1 in 5_000u32..=900_000,
        ) {
            let got = min_passes(alpha, p1).unwrap();
            let want = oracle(f64::from(alpha) / 1e6, f64::from(p1) / 1e6);
            prop_assert!(got >= want, "got {got} < exact {want}");
            prop_assert!(got <= want + 1, "got {got} > exact {want} + 1");
        }

        /// A stricter requirement never needs fewer runs.
        #[test]
        fn monotone_in_alpha_and_p1(
            a1 in 1_000u32..=400_000,
            da in 0u32..=100_000,
            p1 in 5_000u32..=500_000,
            dp in 0u32..=400_000,
        ) {
            let looser_alpha = min_passes(a1 + da, p1).unwrap();
            let stricter_alpha = min_passes(a1, p1).unwrap();
            prop_assert!(stricter_alpha >= looser_alpha);
            let easier_p = min_passes(a1, p1 + dp).unwrap();
            let harder_p = min_passes(a1, p1).unwrap();
            prop_assert!(harder_p >= easier_p);
        }
    }

    /// splitmix64: tiny, fixed, documented PRNG so the simulation is reproducible.
    struct SplitMix(u64);
    impl SplitMix {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }
        fn uniform(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// Monte-Carlo check of the *meaning* of the threshold: for a command that
    /// really fails with probability p1, the fraction of trials in which n
    /// consecutive runs all pass must not exceed alpha beyond sampling error.
    /// Fixed seed: the test is deterministic. Tolerance is 5 standard errors
    /// of a binomial proportion (false alarm probability about 6e-7).
    #[test]
    fn simulation_false_accept_rate_is_bounded_by_alpha() {
        for (alpha_ppm, p1_ppm, seed) in [
            (50_000u32, 100_000u32, 1u64),
            (50_000, 50_000, 2),
            (10_000, 100_000, 3),
        ] {
            let n = min_passes(alpha_ppm, p1_ppm).unwrap();
            let (alpha, p1) = (f64::from(alpha_ppm) / 1e6, f64::from(p1_ppm) / 1e6);
            let trials = 400_000u32;
            let mut rng = SplitMix(seed);
            let mut accepted = 0u32;
            for _ in 0..trials {
                if (0..n).all(|_| rng.uniform() >= p1) {
                    accepted += 1;
                }
            }
            let rate = f64::from(accepted) / f64::from(trials);
            let se = (alpha * (1.0 - alpha) / f64::from(trials)).sqrt();
            assert!(
                rate <= alpha + 5.0 * se,
                "alpha={alpha} p1={p1} n={n}: empirical false-accept {rate} exceeds the bound"
            );
            // And the bound is not vacuous: n-1 runs must exceed alpha (tightness).
            let mut rng = SplitMix(seed + 100);
            let mut accepted = 0u32;
            for _ in 0..trials {
                if (0..n - 1).all(|_| rng.uniform() >= p1) {
                    accepted += 1;
                }
            }
            let rate = f64::from(accepted) / f64::from(trials);
            assert!(
                rate > alpha - 5.0 * se,
                "threshold is not tight: n-1 runs give {rate} <= {alpha}"
            );
        }
    }
}
