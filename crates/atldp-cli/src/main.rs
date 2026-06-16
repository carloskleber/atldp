//! ATLDP command-line interface — thin presentation over `atldp-core` (ADR-0011).
//!
//! Mirrors the Python `atldp` CLI (`catenary`, `cos`) so scripting and validation
//! workflows survive the port (ADR-0002: presentation is a thin layer). Argument
//! parsing is hand-rolled to keep the binary dependency-free and small (the
//! < 30 MB target of ADR-0011); the flag surface matches the Python `argparse`
//! one one-for-one.
//!
//! Examples::
//!
//!     atldp catenary --span 400 --rise 30 --weight 15.97 --tension 30000
//!     atldp cos --span 400 --rise 0 --ref-H 31500 --ref-temp 15 \
//!         --target-temp 75 --target-weight 15.97

use std::collections::HashMap;
use std::process::ExitCode;

use atldp_core::catenary::{solve_span, CatenarySolution, Method};
use atldp_core::change_of_state::{change_of_state, StateCase};
use atldp_core::conductor::drake_acsr;

const USAGE: &str = "\
atldp — ATLDP sag-tension core

USAGE:
    atldp catenary --span S --weight w --tension H [--rise h] [--method auto|catenary|parabola]
    atldp cos --span S --ref-H H --ref-temp T --target-temp T [--rise h]
              [--ref-weight w] [--target-weight w] [--conductor NAME] [--method ...]

Only the built-in 'ACSR Drake 26/7' conductor is available in Phase G1.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let (command, rest) = args
        .split_first()
        .ok_or_else(|| format!("missing command\n\n{USAGE}"))?;
    match command.as_str() {
        "catenary" => cmd_catenary(&parse_flags(rest)?),
        "cos" => cmd_cos(&parse_flags(rest)?),
        "-h" | "--help" | "help" => {
            println!("{USAGE}");
            Ok(())
        }
        other => Err(format!("unknown command: {other:?}\n\n{USAGE}")),
    }
}

/// Parse `--flag value` pairs into a map. Mirrors argparse's `--flag value` form.
fn parse_flags(rest: &[String]) -> Result<HashMap<String, String>, String> {
    let mut flags = HashMap::new();
    let mut it = rest.iter();
    while let Some(tok) = it.next() {
        let key = tok
            .strip_prefix("--")
            .ok_or_else(|| format!("expected a --flag, got {tok:?}"))?;
        let value = it
            .next()
            .ok_or_else(|| format!("flag --{key} expects a value"))?;
        flags.insert(key.to_string(), value.clone());
    }
    Ok(flags)
}

fn req_f64(flags: &HashMap<String, String>, key: &str) -> Result<f64, String> {
    let raw = flags
        .get(key)
        .ok_or_else(|| format!("missing required flag --{key}"))?;
    raw.parse()
        .map_err(|_| format!("--{key} must be a number, got {raw:?}"))
}

fn opt_f64(flags: &HashMap<String, String>, key: &str, default: f64) -> Result<f64, String> {
    match flags.get(key) {
        None => Ok(default),
        Some(raw) => raw
            .parse()
            .map_err(|_| format!("--{key} must be a number, got {raw:?}")),
    }
}

fn method_of(flags: &HashMap<String, String>) -> Result<Method, String> {
    let name = flags.get("method").map(String::as_str).unwrap_or("auto");
    Method::parse(name).map_err(|e| e.to_string())
}

fn print_solution(sol: &CatenarySolution) {
    println!("  method            : {}", sol.method.as_str());
    println!("  horizontal tension: {:.1} N", sol.h_tension);
    println!("  conductor length  : {:.4} m", sol.conductor_length);
    println!(
        "  sag               : {:.4} m  (at x = {:.2} m)",
        sol.sag, sol.sag_position
    );
    println!(
        "  tension @ support : {:.1} / {:.1} N",
        sol.tension_start, sol.tension_end
    );
    println!("  max tension       : {:.1} N", sol.max_tension());
}

fn cmd_catenary(flags: &HashMap<String, String>) -> Result<(), String> {
    let span = req_f64(flags, "span")?;
    let rise = opt_f64(flags, "rise", 0.0)?;
    let weight = req_f64(flags, "weight")?;
    let tension = req_f64(flags, "tension")?;
    let sol =
        solve_span(span, rise, weight, tension, method_of(flags)?).map_err(|e| e.to_string())?;
    println!("Span {span} m, rise {rise} m, w {weight} N/m:");
    print_solution(&sol);
    Ok(())
}

fn cmd_cos(flags: &HashMap<String, String>) -> Result<(), String> {
    let conductor = drake_acsr();
    if let Some(name) = flags.get("conductor") {
        if name != &conductor.name {
            return Err(format!(
                "unknown conductor {name:?}; only {:?} is built in",
                conductor.name
            ));
        }
    }
    let span = req_f64(flags, "span")?;
    let rise = opt_f64(flags, "rise", 0.0)?;
    let ref_h = req_f64(flags, "ref-H")?;
    let ref_temp = req_f64(flags, "ref-temp")?;
    let ref_weight = opt_f64(flags, "ref-weight", conductor.unit_weight)?;
    let target_temp = req_f64(flags, "target-temp")?;
    let target_weight = opt_f64(flags, "target-weight", conductor.unit_weight)?;

    let reference = StateCase::new("reference", ref_temp, ref_weight);
    let target = StateCase::new("target", target_temp, target_weight);
    let sol = change_of_state(
        &conductor,
        span,
        rise,
        ref_h,
        &reference,
        &target,
        method_of(flags)?,
    )
    .map_err(|e| e.to_string())?;
    let pct = 100.0 * sol.max_tension() / conductor.rated_strength;

    println!("{}: span {span} m, rise {rise} m", conductor.name);
    println!("  reference: H={ref_h:.1} N @ {ref_temp} degC, w={ref_weight} N/m");
    println!("  target   : {target_temp} degC, w={target_weight} N/m");
    print_solution(&sol);
    println!("  max tension       : {pct:.1}% of RTS");
    Ok(())
}
