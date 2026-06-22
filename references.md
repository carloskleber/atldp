# References

## Theory

* IRVINE, H. Max; CAUGHEY, Ti K. The linear theory of free vibrations of a suspended cable. Proceedings of the Royal Society of London. A. Mathematical and Physical Sciences, v. 341, n. 1626, p. 299-315, 1974.
* HAGEDORN, P.; SCHÄFER, B. On non-linear free vibrations of an elastic cable. International Journal of Non-Linear Mechanics, v. 15, n. 4-5, p. 333-340, 1980.
* JAFARI, M.; HOU, F.; ABDELKEFI, A. Wind-induced vibration of structural cables. Nonlinear Dynamics, v. 100, p. 351-421, 2020.
* IRVINE, H. Max. Cable Structures. Cambridge: MIT Press, 1981. (Classic monograph on the static and dynamic behavior of cables.)
* <https://mpewsey.github.io/2021/12/17/sag-tension-algorithm.html>

## Standards

* IEEE Std 738 — IEEE Standard for Calculating the Current-Temperature Relationship of Bare Overhead Conductors (ampacity / thermal rating).
* IEEE Std 1283 — IEEE Guide for Determining the Effects of High-Temperature Operation on Conductors, Connectors, and Accessories.
* CIGRE Technical Brochure 601 — Guide for thermal rating calculations of overhead lines, 2014.
* CIGRE Technical Brochure 324 — Sag-tension calculation methods for overhead lines, 2007. (Conductor stress-strain and creep behavior.)
* IEC 60826 — Design criteria of overhead transmission lines (load cases: extreme wind, construction, broken-wire/unbalanced longitudinal).
* IEEE Std 524 — IEEE Guide for the Installation of Overhead Transmission Line Conductors (stringing/sagging practice; field stringing tables).
* ABNT NBR 5422 — Projeto de linhas aéreas de transmissão de energia elétrica (Brazilian standard: loads, wind, clearances, right-of-way).
* The Aluminum Association. Sag and Tension Calculation Methods for Overhead Transmission Lines (Aluminum Electrical Conductor Handbook).
* EPRI. Transmission Line Reference Book — 200 kV and Above ("Red Book") and Wind-Induced Conductor Motion ("Orange Book").

## Commercial/ close source software

* POWER LINE SYSTEMS. *PLS-CADD — Power Line Systems / CADD* (user manual, v. 19). Madison, WI: Power Line Systems (Bentley Systems), 2021. The reference commercial overhead-line design tool. Its **Level 1–4 wire modeling** (Level 1 = ruling span; Levels 2–4 = real-span finite element via the **SAPS** engine, adding inter-phase coupling through structure flexibility matrices and, at Level 4, a full system solve — Peyrot & Goulois flexibility-matrix method) is the industry articulation of ATLDP's ruling-span-core + uneven-span-FEM staging; its initial / final-after-creep / final-after-load ("common point") cable model matches the experimental stress-strain/creep treatment above. <https://www.powerlinesystems.com/plscadd>
* <https://moosevalley.github.io/overhead_cable_sag_calculator.html>
* Southwire Sag10 — conductor sag-tension: <https://www.sag10.com/>

## Sag calculation - Finite element method

* BERTRAND, Charlélie et al. A robust and efficient numerical finite element method for cables. International Journal for Numerical Methods in Engineering, v. 121, n. 18, p. 4157-4186, 2020.
* BERTRAND, Charlélie et al. Reduced-Order Model for the Nonlinear Dynamics of Cables. Journal of Engineering Mechanics, v. 148, n. 9, p. 04022052, 2022.
* SUGIYAMA, Hiroyuki; MIKKOLA, Aki M.; SHABANA, Ahmed A. A non-incremental nonlinear finite element solution for cable problems. In: International Design Engineering Technical Conferences and Computers and Information in Engineering Conference. 2003. p. 171-181.

## Sag calculation - Other methods

* WINKELMAN, P. F. Sag-Tension Computations and Field Measurements of Bonneville Power Administration. Transactions of the AIEE, Part III, v. 78, n. 3, p. 1532-1547, 1959. (Classic change-of-state / ruling-span method.)
* MOTLIS, Y. et al. Limitations of the ruling span method for overhead line conductors at high operating temperatures. IEEE Transactions on Power Delivery, v. 14, n. 2, p. 549-560, 1999.
* CIGRE Technical Brochure 324 — Sag-tension calculation methods for overhead lines, 2007. (Change-of-state equation, experimental vs. predictor methods; see Standards.)
* Catenary / change-of-state worked example: <https://mpewsey.github.io/2021/12/17/sag-tension-algorithm.html>

## Conductor stress-strain and creep (high-temperature behaviour)

* HARVEY, J. R.; LARSON, R. E. Use of Elevated-Temperature Creep Data in Sag-Tension Calculations. IEEE Transactions on Power Apparatus and Systems, v. PAS-89, n. 3, p. 380-386, 1970.
* HARVEY, J. R.; LARSON, R. E. Creep equations of conductors for sag-tension calculations. IEEE Winter Power Meeting, paper C72 190-2, 1972.
* CIGRE Working Group 22.05. Permanent elongation of conductors — predictor equations and evaluation methods. Electra, n. 75, p. 63-98, 1981.
* BRADBURY, J.; DEY, P.; ORAWSKI, G.; PICKUP, K. H. Long-term-creep assessment for overhead-line conductors. Proceedings of the IEE, v. 122, n. 10, p. 1146-1152, 1975.
* The Aluminum Association. A Method of Stress-Strain Testing of Aluminum Conductor and ACSR / Graphic Method for Sag-Tension Calculations (fourth-degree initial/final stress-strain polynomials A0–A4, B0–B4, C0–C4, D0–D4).
* CIGRE Technical Brochure 324 — Sag-tension calculation methods for overhead lines, 2007 (experimental plastic elongation method; aluminium-steel load transfer and knee point; see Standards).

## Repos

* SSTC — sag-tension calculation: <https://github.com/e-pear/SSTC>
* GitHub topic index for sag-tension: <https://github.com/topics/sag-tension>
* OnSag (Overhead Transmission Line Software): <https://github.com/OverheadTransmissionLineSoftware/OnSag> — wxWidgets GUI for stringing/transit sag; consumes a precomputed tension table. Its numeric engine is OTLS-Models (below), which is what we cross-check against.
* Transmission line simulation (MATLAB): <https://github.com/LukeYoung3000/Transmission_Line_Simulation_MATLAB>
* Finding the sag in transmission lines: <https://github.com/MichaelRzadki/Finding-the-Sag-in-Transmission-Lines>
* Maximum sag calculation: <https://github.com/Khwab-kalra/Maximum_Sag_calculation>
* libwires / conductor library (Overhead Transmission Line Software): <https://github.com/OverheadTransmissionLineSoftware/Models> — **adopted as the independent third-party numeric oracle** (ADR-0008/0014 gate 3): vendored as the `third_party/Models` submodule (@ `c270d48`); its `catenary_test.cc` numbers are cross-checked by `crates/atldp-core/tests/golden_otls_models.rs`. See `crates/atldp-core/tests/ORACLES.md`.
