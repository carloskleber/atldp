"""Thin CLI over the sag-tension core (ADR-0002: presentation is a thin layer).

Examples
--------
Solve one span for a fixed horizontal tension::

    atldp catenary --span 400 --rise 30 --weight 15.97 --tension 30000

Change of state for the built-in Drake ACSR conductor::

    atldp cos --span 400 --rise 0 --ref-H 31500 --ref-temp 15 \
        --target-temp 75 --target-weight 15.97
"""

from __future__ import annotations

import argparse

from atldp.core.catenary import solve_span
from atldp.core.change_of_state import StateCase, change_of_state
from atldp.core.conductor import DRAKE_ACSR, LIBRARY


def _print_solution(sol) -> None:
    print(f"  method            : {sol.method}")
    print(f"  horizontal tension: {sol.H:,.1f} N")
    print(f"  conductor length  : {sol.conductor_length:,.4f} m")
    print(f"  sag               : {sol.sag:,.4f} m  (at x = {sol.sag_position:,.2f} m)")
    print(f"  tension @ support : {sol.tension_start:,.1f} / {sol.tension_end:,.1f} N")
    print(f"  max tension       : {sol.max_tension:,.1f} N")


def _cmd_catenary(args: argparse.Namespace) -> int:
    sol = solve_span(args.span, args.rise, args.weight, args.tension, method=args.method)
    print(f"Span {args.span} m, rise {args.rise} m, w {args.weight} N/m:")
    _print_solution(sol)
    return 0


def _cmd_cos(args: argparse.Namespace) -> int:
    conductor = LIBRARY.get(args.conductor, DRAKE_ACSR)
    ref = StateCase("reference", args.ref_temp, args.ref_weight)
    target = StateCase("target", args.target_temp, args.target_weight)
    sol = change_of_state(
        conductor, args.span, args.rise, args.ref_H, ref, target, method=args.method
    )
    pct = 100.0 * sol.max_tension / conductor.rated_strength
    print(f"{conductor.name}: span {args.span} m, rise {args.rise} m")
    print(f"  reference: H={args.ref_H:,.1f} N @ {args.ref_temp} degC, w={args.ref_weight} N/m")
    print(f"  target   : {args.target_temp} degC, w={args.target_weight} N/m")
    _print_solution(sol)
    print(f"  max tension       : {pct:.1f}% of RTS")
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="atldp", description="ATLDP sag-tension core")
    sub = p.add_subparsers(dest="command", required=True)

    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--span", type=float, required=True, help="horizontal distance, m")
    common.add_argument("--rise", type=float, default=0.0, help="elevation difference end-start, m")
    common.add_argument(
        "--method", choices=["auto", "catenary", "parabola"], default="auto"
    )

    c = sub.add_parser("catenary", parents=[common], help="solve a span at fixed tension")
    c.add_argument("--weight", type=float, required=True, help="resultant load, N/m")
    c.add_argument("--tension", type=float, required=True, help="horizontal tension, N")
    c.set_defaults(func=_cmd_catenary)

    s = sub.add_parser("cos", parents=[common], help="change of state")
    s.add_argument("--conductor", default=DRAKE_ACSR.name)
    s.add_argument("--ref-H", type=float, required=True, help="reference horizontal tension, N")
    s.add_argument("--ref-temp", type=float, required=True, help="reference temperature, degC")
    s.add_argument("--ref-weight", type=float, default=DRAKE_ACSR.unit_weight)
    s.add_argument("--target-temp", type=float, required=True, help="target temperature, degC")
    s.add_argument("--target-weight", type=float, default=DRAKE_ACSR.unit_weight)
    s.set_defaults(func=_cmd_cos)

    return p


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
