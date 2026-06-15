# ADR-0007 — Prototype isolation and repository hygiene

- Status: Accepted (implemented in Phase 0, 2026-06-15)
- Date: 2026-06-15

## Context

`tests/` is intentionally a sandbox of throwaway prototypes in any language. The
terrain prototype currently carries a committed-by-accident risk: its virtual
environment `tests/terrain/.venv/` is **676 MB and is not git-ignored**. DEM
rasters and LaTeX build artifacts are similarly large/derived. Without hygiene
rules the repository will bloat and prototypes will interfere with each other.

## Decision

- Each prototype lives in its own folder under `tests/<name>/` with its **own
  isolated environment** and a local `requirements.txt`/lockfile. `uv` or the
  stdlib `venv` are both acceptable.
- **Never commit** virtual environments, DEM/raster data, LaTeX build artifacts
  (`*.aux`, `*.log`, `*.synctex.gz`), or other large/derived files. Add them to
  `.gitignore`.
- Prototype code is not a dependency of the core. Promotion of a prototype into a
  production module requires tests and validation (ADR-0008).

## Consequences

- Small, clonable repository; reproducible per-prototype environments.
- Requires a `.gitignore` update (e.g. `**/.venv/`, `*.tif`, `docs/theory.aux`
  et al.) and removing the already-present `.venv` from any future commit.
- Contributors must set up an environment per prototype rather than one global one.
