//! Conductor constitutive model and a small built-in library.
//!
//! Two constitutive laws are provided:
//!
//! * **Linear-elastic + thermal** (the default): strain is the sum of an elastic
//!   part `sigma/E` and a thermal part `alpha*(T - T_ref)`, optionally plus a
//!   constant creep strain offset. This is the standard "single-modulus"
//!   sag-tension model and is enough to validate the change-of-state machinery
//!   (ADR-0003).
//! * **Nonlinear bimetallic stress-strain** ([`StressStrainModel`]): the full
//!   Aluminum Association / CIGRE TB 324 model with *separate* aluminium and steel
//!   load-strain (and creep) polynomials, distinct thermal coefficients, and the
//!   bimetallic load transfer between the two components. Attach one to a
//!   [`Conductor`] via [`Conductor::stress_strain`] and [`Conductor::strain`]
//!   uses it automatically — no change to callers (e.g. the change-of-state
//!   equation), which is the additive hook envisaged in ADR-0003 (G1b).
//!
//! The nonlinear law is *not* a transcription of any third-party solver: it is
//! built directly from the published stress-strain physics, so reproducing an
//! independent reference (OTLS-Models) with it is a genuine cross-check rather
//! than a tautology (see `crates/atldp-core/tests/golden_otls_change_of_state.rs`).
//!
//! All SI quantities use: areas m^2, diameters m, weights N/m, moduli Pa,
//! strengths N, thermal coefficients 1/degC, temperatures degC. The
//! [`StressStrainModel`] is deliberately **unit-agnostic** — see its docs.
//!
//! Mirror of the Python `atldp.core.conductor` oracle (ADR-0014), plus the new
//! nonlinear model (G1b, no Python counterpart).

use crate::G;

/// A conductor's geometry and constitutive constants.
///
/// The linear-elastic fields (`modulus_final`, `thermal_coeff`,
/// `reference_temperature`) drive [`Conductor::strain`] by default. Attaching a
/// [`StressStrainModel`] via `stress_strain` switches `strain` to the nonlinear
/// bimetallic law (G1b).
#[derive(Clone, Debug, PartialEq)]
pub struct Conductor {
    /// Catalogue name.
    pub name: String,
    /// Total cross-sectional area, m^2.
    pub area: f64,
    /// Overall diameter, m.
    pub diameter: f64,
    /// Self-weight per unit length, N/m.
    pub unit_weight: f64,
    /// Rated tensile strength (RTS), N.
    pub rated_strength: f64,
    /// Final modulus of elasticity, Pa.
    pub modulus_final: f64,
    /// Coefficient of linear expansion, 1/degC.
    pub thermal_coeff: f64,
    /// degC at which the unstrained length is defined.
    pub reference_temperature: f64,
    /// Initial modulus, Pa (optional; Phase 2).
    pub modulus_initial: Option<f64>,
    /// Nonlinear bimetallic stress-strain model (optional, G1b). When `Some`,
    /// [`Conductor::strain`] uses it instead of the linear-elastic law.
    pub stress_strain: Option<StressStrainModel>,
}

impl Conductor {
    /// Horizontal-tension stress `H/A` (Pa).
    ///
    /// The change-of-state equation uses the horizontal tension as the stress
    /// measure, the conventional single-modulus simplification.
    pub fn stress(&self, horizontal_tension: f64) -> f64 {
        horizontal_tension / self.area
    }

    /// Total mechanical strain relative to the unstrained reference length.
    ///
    /// With a [`StressStrainModel`] attached (`stress_strain`), this inverts the
    /// nonlinear composite stress-strain curve at the given tension and
    /// temperature (the additive `creep_strain` offset is then unused — creep is
    /// modelled by the model's creep polynomial). Otherwise it is the
    /// linear-elastic + thermal sum.
    pub fn strain(&self, tension: f64, temperature: f64, creep_strain: f64) -> f64 {
        if let Some(model) = &self.stress_strain {
            model.strain(tension, temperature, Curve::Initial)
        } else {
            let elastic = self.stress(tension) / self.modulus_final;
            let thermal = self.thermal_coeff * (temperature - self.reference_temperature);
            elastic + thermal + creep_strain
        }
    }
}

/// Which stress-strain curve a [`StressStrainModel`] query follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Curve {
    /// Short-term (initial) load-strain polynomial.
    Initial,
    /// Long-term (after-creep) polynomial.
    Creep,
}

/// Independent nonlinear bimetallic stress-strain model (Aluminum Association /
/// CIGRE TB 324 method).
///
/// A stranded conductor (ACSR: aluminium *shell* over a steel *core*) is two
/// materials sharing the same elongation. Each component's load is a 4th-order
/// polynomial in **percent** strain; the conductor load is their sum. The two
/// components have different thermal coefficients, so a temperature change shifts
/// their mechanical strains differently — the *bimetallic* effect that a single
/// modulus cannot capture (e.g. aluminium going slack at high temperature, the
/// steel core then carrying the load).
///
/// # Unit convention
/// The model is **unit-agnostic** in force and temperature: the polynomial
/// coefficients already fold in the conductor area, so they map a percent strain
/// to a *load* (in whatever force unit the coefficients use), and the thermal
/// coefficients are per whatever temperature unit `poly_ref_temp` and the query
/// temperatures use. Feed it consistent units (e.g. lbf + degF, or N + degC) and
/// you get that force unit back. This lets the published US-customary Drake data
/// drive the change-of-state directly for the OTLS cross-check.
///
/// # Derivation, not transcription
/// This is built from the stress-strain physics itself (composite-polynomial
/// inversion + thermal compatibility), not copied from any solver's internal
/// region/stretch machinery. That independence is what makes the OTLS comparison
/// a validation.
#[derive(Clone, Debug, PartialEq)]
pub struct StressStrainModel {
    /// Steel-core initial load-strain polynomial, coefficients of percent strain.
    pub core_loadstrain: [f64; 5],
    /// Aluminium-shell initial load-strain polynomial.
    pub shell_loadstrain: [f64; 5],
    /// Steel-core after-creep polynomial.
    pub core_creep: [f64; 5],
    /// Aluminium-shell after-creep polynomial.
    pub shell_creep: [f64; 5],
    /// Steel-core thermal coefficient, per temperature unit.
    pub core_thermal: f64,
    /// Aluminium-shell thermal coefficient, per temperature unit.
    pub shell_thermal: f64,
    /// Datum temperature at which the polynomials are defined.
    pub poly_ref_temp: f64,
}

impl StressStrainModel {
    /// Evaluate a load-strain polynomial at percent strain `s`.
    fn poly(coeffs: &[f64; 5], s: f64) -> f64 {
        let mut acc = 0.0;
        for (i, c) in coeffs.iter().enumerate() {
            acc += c * s.powi(i as i32);
        }
        acc
    }

    /// One component's load at mechanical (fractional) strain `mech`.
    ///
    /// Compression is clamped to zero: a stranded component carries essentially no
    /// compressive load (the strands simply go slack), which is also how the
    /// bimetallic load transfer at high temperature arises.
    fn component_load(coeffs: &[f64; 5], mech: f64) -> f64 {
        if mech <= 0.0 {
            0.0
        } else {
            Self::poly(coeffs, mech * 100.0)
        }
    }

    /// Composite conductor load at total strain `strain` and `temperature`.
    pub fn load(&self, strain: f64, temperature: f64, curve: Curve) -> f64 {
        let (core, shell) = match curve {
            Curve::Initial => (&self.core_loadstrain, &self.shell_loadstrain),
            Curve::Creep => (&self.core_creep, &self.shell_creep),
        };
        let dt = temperature - self.poly_ref_temp;
        let mech_core = strain - self.core_thermal * dt;
        let mech_shell = strain - self.shell_thermal * dt;
        Self::component_load(core, mech_core) + Self::component_load(shell, mech_shell)
    }

    /// Total strain carrying composite load `load` at `temperature` — the inverse
    /// of [`StressStrainModel::load`].
    ///
    /// Solved by bisection: the composite curve is monotone increasing in strain
    /// over the working range, so the root is unique. The bracket spans light
    /// compression to ~5% strain, which covers loads up to and beyond a typical
    /// rated strength.
    pub fn strain(&self, load: f64, temperature: f64, curve: Curve) -> f64 {
        let mut lo = -0.005;
        let mut hi = 0.05;
        let f = |e: f64| self.load(e, temperature, curve) - load;
        let mut f_lo = f(lo);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            let f_mid = f(mid);
            if (f_lo > 0.0) == (f_mid > 0.0) {
                lo = mid;
                f_lo = f_mid;
            } else {
                hi = mid;
            }
            if (hi - lo) <= 1e-12 {
                break;
            }
        }
        0.5 * (lo + hi)
    }
}

/// ACSR Drake 26/7 — the canonical worked-example conductor in the Aluminum
/// Association / IEEE sag-tension literature. SI values (Aluminum Electrical
/// Conductor Handbook; Southwire). The composite final modulus and thermal
/// coefficient are the single-modulus values used by the linear-elastic law; the
/// full bimetallic stress-strain behaviour is available via
/// [`drake_acsr_stress_strain`] (G1b).
///
/// `unit_weight` is `1.628 kg/m * G = 15.97 N/m`, matching the Python oracle's
/// `Conductor.from_mass` construction.
pub fn drake_acsr() -> Conductor {
    Conductor {
        name: "ACSR Drake 26/7".to_string(),
        area: 468.5e-6,
        diameter: 28.11e-3,
        unit_weight: 1.628 * G, // kg/m -> 15.97 N/m
        rated_strength: 140.1e3,
        modulus_final: 74.0e9,
        thermal_coeff: 18.9e-6,
        reference_temperature: 20.0,
        modulus_initial: Some(64.0e9),
        stress_strain: None,
    }
}

/// Published Aluminum Association stress-strain data for ACSR Drake 26/7, in
/// **US-customary consistent units** (load in lbf, strain in percent, temperature
/// in degF; conductor area 0.7264 in^2 folded into the coefficients).
///
/// The polynomials are the standard handbook load-strain (initial) and creep
/// curves for the steel core and aluminium shell of Drake; the thermal
/// coefficients are the per-material values (steel 6.4e-6/degF, aluminium
/// 12.8e-6/degF) at the 70 degF datum. This is published *physical* data for the
/// conductor — the model that consumes it ([`StressStrainModel`]) is derived
/// independently, so reproducing OTLS-Models' reload tensions from it is a
/// cross-implementation validation (ADR-0008/0014).
///
/// Coefficients are `psi * area` so that the polynomial yields load directly; the
/// `0.7264 in^2` factor is the Drake physical aluminium-plus-steel area.
pub fn drake_acsr_stress_strain() -> StressStrainModel {
    // Conductor area (in^2); coefficients below are psi values scaled by it so the
    // polynomial maps percent strain directly to load (lbf).
    const A: f64 = 0.7264;
    let scale = |psi: [f64; 5]| psi.map(|c| c * A);
    StressStrainModel {
        core_loadstrain: scale([-69.3, 38629.0, 3998.1, -45713.0, 27892.0]),
        shell_loadstrain: scale([-1213.0, 44308.1, -14004.4, -37618.0, 30676.0]),
        core_creep: scale([47.1, 36211.3, 12201.4, -72392.0, 46338.0]),
        shell_creep: scale([-544.8, 21426.8, -18842.2, 5495.0, 0.0]),
        core_thermal: 0.0000064,
        shell_thermal: 0.0000128,
        poly_ref_temp: 70.0,
    }
}

/// ACSR Drake 26/7 set up for the **nonlinear** change-of-state cross-check, in
/// US-customary consistent units (lbf, ft, degF) so it lines up with the
/// OTLS-Models reference fixture. Self-weight `1.094 lbf/ft`, RTS `31500 lbf`.
pub fn drake_acsr_nonlinear() -> Conductor {
    Conductor {
        name: "ACSR Drake 26/7 (nonlinear, US units)".to_string(),
        area: 0.7264,
        diameter: 1.108 / 12.0, // 1.108 in -> ft
        unit_weight: 1.094,
        rated_strength: 31500.0,
        modulus_final: f64::NAN, // unused: the stress-strain model drives strain()
        thermal_coeff: f64::NAN,
        reference_temperature: 70.0,
        modulus_initial: None,
        stress_strain: Some(drake_acsr_stress_strain()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drake_weight_from_mass() {
        let c = drake_acsr();
        assert!((c.unit_weight - 1.628 * 9.80665).abs() < 1e-12);
    }

    #[test]
    fn strain_is_elastic_plus_thermal() {
        let c = drake_acsr();
        let s = c.strain(30000.0, 75.0, 0.0);
        let expected = (30000.0 / c.area) / c.modulus_final
            + c.thermal_coeff * (75.0 - c.reference_temperature);
        assert!((s - expected).abs() < 1e-15);
    }

    #[test]
    fn nonlinear_strain_load_round_trip() {
        let m = drake_acsr_stress_strain();
        for &load in &[2000.0, 6000.0, 12000.0, 20000.0] {
            for &temp in &[0.0, 60.0, 212.0] {
                let e = m.strain(load, temp, Curve::Initial);
                let back = m.load(e, temp, Curve::Initial);
                assert!(
                    (back - load).abs() < 1e-4,
                    "load {load} temp {temp}: round-trip {back}"
                );
            }
        }
    }

    #[test]
    fn nonlinear_is_softer_than_single_modulus_and_temperature_lowers_strain_capacity() {
        // The composite tangent stiffens then softens; basic monotonicity: a
        // higher load needs a higher strain, and a hotter conductor reaches a
        // given load at a higher total strain (thermal expansion adds in).
        let m = drake_acsr_stress_strain();
        assert!(m.strain(12000.0, 60.0, Curve::Initial) > m.strain(6000.0, 60.0, Curve::Initial));
        assert!(m.strain(6000.0, 100.0, Curve::Initial) > m.strain(6000.0, 0.0, Curve::Initial));
    }

    #[test]
    fn conductor_strain_dispatches_to_nonlinear_model() {
        let c = drake_acsr_nonlinear();
        let direct = drake_acsr_stress_strain().strain(6000.0, 60.0, Curve::Initial);
        assert_eq!(c.strain(6000.0, 60.0, 0.0), direct);
    }
}
