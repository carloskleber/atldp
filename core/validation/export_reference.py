"""Export Python-oracle outputs over a parameter sweep as CSV fixtures.

These fixtures are the frozen reference the Rust `atldp-core` cross-check tests
read (ADR-0014 gate 2: cross-implementation agreement over a parameter sweep).
Regenerate them from the validated Python core whenever the oracle changes, then
re-run `cargo test -p atldp-core` to confirm the Rust port still agrees within
tolerance. Running this is *not* required to build or test the Rust workspace —
the CSVs are committed so CI needs no Python.

Usage (from the repo root, with the core venv active)::

    python core/validation/export_reference.py

Writes:
    crates/atldp-core/tests/fixtures/oracle_spans.csv   (solve_span sweep)
    crates/atldp-core/tests/fixtures/oracle_cos.csv      (change_of_state sweep)
"""

from __future__ import annotations

import csv
from pathlib import Path

from atldp.core.catenary import solve_span
from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.conductor import DRAKE_ACSR

FIXTURES = Path(__file__).resolve().parents[2] / "crates" / "atldp-core" / "tests" / "fixtures"

W = DRAKE_ACSR.unit_weight  # 15.97 N/m


def _spans_sweep():
    rows = []
    for method in ("auto", "catenary", "parabola"):
        for S in (100.0, 250.0, 400.0, 600.0, 800.0):
            for h in (-120.0, -40.0, 0.0, 40.0, 120.0):
                for w in (W, 2.0 * W):
                    for H in (15000.0, 30000.0, 50000.0):
                        sol = solve_span(S, h, w, H, method=method)
                        rows.append(
                            {
                                "method_in": method,
                                "S": S,
                                "h": h,
                                "w": w,
                                "H": H,
                                "method_used": sol.method,
                                "length": repr(sol.conductor_length),
                                "sag": repr(sol.sag),
                                "sag_position": repr(sol.sag_position),
                                "low_point_x": repr(sol.low_point_x),
                                "tension_start": repr(sol.tension_start),
                                "tension_end": repr(sol.tension_end),
                            }
                        )
    return rows


def _cos_sweep():
    rows = []
    for method in ("auto", "catenary"):
        for S in (200.0, 400.0, 600.0):
            for h in (-40.0, 0.0, 40.0):
                for ref_H in (25000.0, 31500.0):
                    for ref_temp in (0.0, 15.0):
                        for tgt_temp in (-10.0, 50.0, 75.0):
                            for tgt_w in (W, 2.0 * W):
                                ref = StateCase("ref", ref_temp, W)
                                tgt = StateCase("tgt", tgt_temp, tgt_w)
                                sol = change_of_state(
                                    DRAKE_ACSR, S, h, ref_H, ref, tgt, method=method
                                )
                                rows.append(
                                    {
                                        "method_in": method,
                                        "S": S,
                                        "h": h,
                                        "ref_H": ref_H,
                                        "ref_temp": ref_temp,
                                        "tgt_temp": tgt_temp,
                                        "tgt_w": tgt_w,
                                        "H2": repr(sol.H),
                                        "length": repr(sol.conductor_length),
                                        "sag": repr(sol.sag),
                                        "tension_start": repr(sol.tension_start),
                                        "tension_end": repr(sol.tension_end),
                                    }
                                )
    return rows


def _write(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)
    print(f"wrote {len(rows)} rows -> {path}")


def main() -> None:
    _write(FIXTURES / "oracle_spans.csv", _spans_sweep())
    _write(FIXTURES / "oracle_cos.csv", _cos_sweep())


if __name__ == "__main__":
    main()
