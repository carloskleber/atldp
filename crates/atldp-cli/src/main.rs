//! ATLDP command-line interface — thin presentation over `atldp-core` (ADR-0011).
//!
//! Mirrors today's Python `atldp` CLI (`catenary`, `cos`, …) so scripting and
//! validation workflows survive the port. Subcommands land in phase G1 alongside
//! the core modules they wrap.

fn main() {
    println!("atldp {} (skeleton — phase G0)", atldp_core::VERSION);
}
