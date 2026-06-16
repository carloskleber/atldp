"""Conductor constitutive model and a small built-in library.

Phase 1 uses a **linear-elastic + thermal** constitutive law: the conductor
strain is the sum of an elastic part ``sigma/E`` and a thermal part
``alpha*(T - T_ref)``, optionally plus a constant creep strain offset. This is
the standard "single-modulus" sag-tension model and is enough to validate the
change-of-state machinery (ADR-0003).

The full nonlinear stress-strain behaviour with separate initial/final aluminium
and steel curves and time/temperature creep (CIGRE TB 324; Aluminum
Association handbook) is deferred: it refines :meth:`Conductor.strain` without
changing any caller. The hooks (``modulus_initial``, ``creep_strain``) are
present so that refinement is additive.

All quantities are SI: areas m^2, diameters m, weights N/m, moduli Pa, strengths
N, thermal coefficients 1/degC, temperatures degC.
"""

from __future__ import annotations

from dataclasses import dataclass

from atldp.core import G


@dataclass(frozen=True)
class Conductor:
    name: str
    area: float  # total cross-sectional area, m^2
    diameter: float  # overall diameter, m
    unit_weight: float  # self-weight per unit length, N/m
    rated_strength: float  # rated tensile strength (RTS), N
    modulus_final: float  # final modulus of elasticity, Pa
    thermal_coeff: float  # coefficient of linear expansion, 1/degC
    reference_temperature: float = 20.0  # degC at which the unstrained length is defined
    modulus_initial: float | None = None  # initial modulus, Pa (optional; Phase 2)

    @classmethod
    def from_mass(cls, mass_per_length: float, **kwargs) -> "Conductor":
        """Build a conductor from mass per length (kg/m) instead of weight."""
        return cls(unit_weight=mass_per_length * G, **kwargs)

    def stress(self, horizontal_tension: float) -> float:
        """Horizontal-tension stress ``H/A`` (Pa).

        The change-of-state equation uses the horizontal tension as the stress
        measure, the conventional single-modulus simplification.
        """
        return horizontal_tension / self.area

    def strain(self, horizontal_tension: float, temperature: float, creep_strain: float = 0.0) -> float:
        """Total mechanical strain relative to the unstrained reference length."""
        elastic = self.stress(horizontal_tension) / self.modulus_final
        thermal = self.thermal_coeff * (temperature - self.reference_temperature)
        return elastic + thermal + creep_strain


# --- Built-in library --------------------------------------------------------
#
# ACSR Drake 26/7 — the canonical worked-example conductor in the Aluminum
# Association / IEEE sag-tension literature. Nominal published values (Aluminum
# Electrical Conductor Handbook; Southwire). The composite final modulus and
# thermal coefficient are the single-modulus values appropriate to the Phase 1
# model; the full bimetallic stress-strain polynomial is a later refinement
# (CIGRE TB 324).
DRAKE_ACSR = Conductor.from_mass(
    name="ACSR Drake 26/7",
    area=468.5e-6,
    diameter=28.11e-3,
    mass_per_length=1.628,  # kg/m  -> 15.97 N/m
    rated_strength=140.1e3,
    modulus_final=74.0e9,
    thermal_coeff=18.9e-6,
    reference_temperature=20.0,
    modulus_initial=64.0e9,
)

LIBRARY = {DRAKE_ACSR.name: DRAKE_ACSR}
