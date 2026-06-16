//! Conductor constitutive model and a small built-in library.
//!
//! Phase 1 uses a **linear-elastic + thermal** constitutive law: the conductor
//! strain is the sum of an elastic part `sigma/E` and a thermal part
//! `alpha*(T - T_ref)`, optionally plus a constant creep strain offset. This is
//! the standard "single-modulus" sag-tension model and is enough to validate the
//! change-of-state machinery (ADR-0003).
//!
//! The full nonlinear stress-strain behaviour with separate initial/final
//! aluminium and steel curves and time/temperature creep (CIGRE TB 324; Aluminum
//! Association handbook) is deferred: it refines [`Conductor::strain`] without
//! changing any caller. The hooks (`modulus_initial`, `creep_strain`) are present
//! so that refinement is additive.
//!
//! All quantities are SI: areas m^2, diameters m, weights N/m, moduli Pa,
//! strengths N, thermal coefficients 1/degC, temperatures degC.
//!
//! Mirror of the Python `atldp.core.conductor` oracle (ADR-0014).

use crate::G;

/// A conductor's geometry and linear-elastic + thermal constitutive constants.
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
    pub fn strain(&self, horizontal_tension: f64, temperature: f64, creep_strain: f64) -> f64 {
        let elastic = self.stress(horizontal_tension) / self.modulus_final;
        let thermal = self.thermal_coeff * (temperature - self.reference_temperature);
        elastic + thermal + creep_strain
    }
}

/// ACSR Drake 26/7 — the canonical worked-example conductor in the Aluminum
/// Association / IEEE sag-tension literature. Nominal published values (Aluminum
/// Electrical Conductor Handbook; Southwire). The composite final modulus and
/// thermal coefficient are the single-modulus values appropriate to the Phase 1
/// model; the full bimetallic stress-strain polynomial is a later refinement
/// (CIGRE TB 324).
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
}
