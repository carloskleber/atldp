"""Ruling-span (equivalent-span) section model.

A run of spans between two strain (dead-end) structures shares a single
horizontal tension, because the suspension insulators swing freely and equalise
``H`` along the section. The classic ruling-span method replaces the section with
one fictitious level span

    RS = sqrt( sum(S_i^3) / sum(S_i) )

solves the change-of-state on that single span to get the common horizontal
tension for each weather state, and then applies that tension back to every real
span (each with its own length and elevation difference) to get per-span sag and
tension.

This is exact only under the usual ruling-span assumptions (free-swinging
suspensions, similar spans); its limits at high temperature are documented by
Motlis et al. 1999, and the FEM track (ADR-0003, Phase 6) is the escape hatch
when those assumptions break.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from atldp.core.catenary import CatenarySolution, solve_span
from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.conductor import Conductor
from atldp.core.geometry import Span


@dataclass(frozen=True)
class RulingSpanResult:
    ruling_span: float  # equivalent span length, m
    H: float  # common horizontal tension at the target state, N
    ruling_solution: CatenarySolution  # solution of the fictitious ruling span
    spans: list[CatenarySolution]  # per-span solutions at the common H


@dataclass(frozen=True)
class Section:
    """A tension section: a conductor and the ordered list of its spans."""

    conductor: Conductor
    spans: list[Span]

    @property
    def ruling_span(self) -> float:
        lengths = [s.horizontal_distance for s in self.spans]
        return math.sqrt(sum(l ** 3 for l in lengths) / sum(lengths))

    def solve(
        self,
        reference_H: float,
        reference: StateCase,
        target: StateCase,
        method: str = "auto",
    ) -> RulingSpanResult:
        """Solve the section at ``target`` given a ``reference`` state.

        ``reference_H`` is the common horizontal tension at the reference state
        (e.g. the stringing tension). The change-of-state runs on the ruling
        span; the resulting tension is applied to every real span.
        """
        rs = self.ruling_span
        ruling_solution = change_of_state(
            self.conductor, rs, 0.0, reference_H, reference, target, method=method
        )
        H = ruling_solution.H
        per_span = [
            solve_span(s.horizontal_distance, s.elevation_difference, target.w, H, method=method)
            for s in self.spans
        ]
        return RulingSpanResult(
            ruling_span=rs,
            H=H,
            ruling_solution=ruling_solution,
            spans=per_span,
        )
