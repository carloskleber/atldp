# Terrain prototype

Throwaway prototype that fetches elevation data and renders a 3D terrain surface
with Plotly. It currently pulls elevation from the OpenElevation API; per
[ADR-0005](../../docs/adr/0005-local-dem-as-geospatial-source-of-truth.md), online
elevation APIs are prototype-only — local DEMs are the intended source of truth.

## Isolated environment (ADR-0007)

This prototype keeps its own environment and dependencies; it is not wired into
the core.

```bash
cd tests/terrain
python -m venv .venv        # or: uv venv
source .venv/bin/activate
pip install -r requirements.txt
python terrain_navigation.py
```

## What is and isn't committed

Generated artifacts are **git-ignored** and must not be committed:

- `*.html` — Plotly outputs (`map_with_elevation.html`, `terrain_model.html`,
  `elevation_profile.html`)
- `dem.tif` and any other raster / DEM cache
- `.venv/`
