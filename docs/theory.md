# The sag-tension problem

*Carlos Kleber C. Arruda*

## Notation

All quantities are SI. Symbols are defined once here and used consistently
throughout; equations below are not re-annotated.

### Geometry and coordinates

| Symbol | Meaning |
| --- | --- |
| $s$ | arc-length coordinate along the conductor |
| $x,\,y$ | horizontal / vertical coordinates in the span plane (support 1 at the origin) |
| $S$ | horizontal span (chord projection); $S_i$ an individual span; $S_r$ the ruling (equivalent) span |
| $h$ | elevation difference between the two supports |
| $a$ | abscissa of the catenary low point |
| $c$ | catenary constant, $c = H/w$ |
| $L$ | conductor (arc) length; $L_0$ unstrained length; $L_e$ effective cable length (Irvine) |

### Loads

| Symbol | Meaning |
| --- | --- |
| $w$ | resultant transverse load per unit length |
| $w_c,\,w_i,\,w_w$ | self-weight ($=\rho A g$), ice, and wind load per unit length |
| $g$ | gravitational acceleration |
| $\psi$ | conductor swing (blow-out) angle |

### Material and constitutive

| Symbol | Meaning |
| --- | --- |
| $A,\,\rho$ | conductor cross-sectional area and material density ($\rho A$ = mass per unit length) |
| $A_{\text{al}},\,A_{\text{st}}$ | aluminium and steel-core cross-sectional areas ($A = A_{\text{al}}+A_{\text{st}}$) |
| $E,\,E_{\text{final}}$ | modulus of elasticity; final (settled) modulus |
| $\sigma$ | mean axial stress, $\sigma = H/A$; $\sigma_{\text{al}},\,\sigma_{\text{st}}$ per-component stress |
| $\varepsilon,\,\varepsilon_{\text{creep}}$ | total strain and permanent creep strain |
| $a_{j,k},\,b_{j,k}$ | initial / final stress–strain polynomial coefficients ($j\in\{\text{al},\text{st}\}$, degree $k=0\ldots4$) |
| $K,\,m,\,\beta,\,n$ | creep-predictor coefficient and stress / temperature / time exponents |
| $\alpha,\,\alpha_{\text{al}},\,\alpha_{\text{st}}$ | coefficient of linear thermal expansion (composite; per component) |
| $\theta,\,\theta_{\text{ref}},\,\theta_k$ | conductor temperature; reference temperature; knee-point temperature |
| RTS | rated tensile strength |

### Tension

| Symbol | Meaning |
| --- | --- |
| $H$ | horizontal tension component (constant along a span) |
| $T$ | conductor tension, $T(x)$ along the curve; $T_{\max}$ the maximum (at the higher support) |
| $\tau$ | dynamic (vibratory) tension increment |

### Dynamics and vibration

| Symbol | Meaning |
| --- | --- |
| $u_x,\,u_y,\,u_z$ | dynamic displacement components about equilibrium (in-plane horizontal, in-plane vertical, out-of-plane) |
| $\eta$ | modal displacement field; $\phi_k$ mode shape; $q,\,q_k$ modal coordinate |
| $\omega,\,\mu$ | natural (modal) frequency and modal damping coefficient |
| $c_2,\,c_3$ | quadratic / cubic modal coupling coefficients |
| $f(t)$ | projected external (wind / aeolian) force |
| $\lambda^2$ | Irvine parameter |
| $t$ | time (also elapsed time under load, in the creep predictor) |

## A 2D picture of a 3D problem

Sag-tension is almost always *drawn* in 2D — a single cable hanging in a vertical
plane between two equal-height supports. The real problem is 3D:

- **Uneven spans.** The two attachment points are generally at different
  elevations (sloping terrain, different structure heights). The cable then hangs
  as an *inclined* catenary, and the maximum tension is at the higher support.
- **Angle towers.** The line changes plan direction at angle structures. This does
  not change the within-span mechanics — the cable between two points still hangs
  in the vertical plane through those two points — but it produces a transverse
  load on the structure (the bisector load) handled at the structure-modeling
  stage.
- **Wind blow-out / swing.** Transverse wind tilts the load plane out of vertical,
  turning the in-plane catenary into a swung 3D curve. This is a load-case concern
  (Phase 2): the static shape in the swung plane is still a catenary under the
  *resultant* load per unit length.
- **Higher-fidelity coupling.** When ruling-span assumptions break down, the complete
  models use the **finite element method** [10, 11, 12]. Because each structure can
  attach the conductors at its **own points** (different family, height and crossarm
  geometry), spans between mixed structures are **genuinely uneven** — the common case,
  not an edge case — so the **static** uneven-span FEM solve is **brought forward**
  (ADR-0021, phase G11) as an alternative section kernel, validated against the analytic
  core in their overlap. The **dynamic** FEM track (longitudinal coupling, vibration)
  stays behind the static core (ADR-0003).

The analytic core therefore works in 3D coordinates from the start, reduces each
span to a horizontal distance $S$ and elevation difference $h$, and solves the
inclined catenary rather than assuming level supports.

## Model

**Premises:**

- Span depth ratio > 1/8, or inclined supports: the exact solution must be used.
- Span depth ratio < 1/8 and aligned supports: the parabolic approximation can be used.

Cable equilibrium for a level span [7]:

$$
\begin{aligned}
\frac{\partial}{\partial s} \left( T \frac{dx}{ds}\right) &= 0 \\
\frac{\partial}{\partial s} \left( T \frac{dy}{ds}\right) &= -w
\end{aligned}
$$

For small sags it is approximated by the parabola [5]:

$$
y = \frac{w\,S^2}{2 H} \left[ \frac{x}{S} - \left( \frac{x}{S} \right)^2 \right]
$$

### Inclined (uneven) span — exact catenary

With the catenary constant $c = H/w$, support 1 at $(0,0)$ and support 2 at
$(S,h)$, the curve is [4, 5]

$$
y(x) = c\,\cosh\!\frac{x-a}{c} - c\,\cosh\!\frac{a}{c},
\qquad
a = \frac{S}{2} - c\,\operatorname{arcsinh}\!\frac{h}{2c\,\sinh\!\frac{S}{2c}},
$$

where the low-point abscissa $a$ falls outside $[0,S]$ for steep inclines. The
conductor length has the closed form

$$
L = \sqrt{\,h^2 + \left(2c\,\sinh\!\frac{S}{2c}\right)^2}\,,
$$

and the tension is $T(x) = H\cosh\!\frac{x-a}{c}$, largest at the higher support.
For $h = 0$ these reduce to the level-span results ($a = S/2$,
$L = 2c\sinh\frac{S}{2c}$, sag $= c(\cosh\frac{S}{2c}-1)$, $T_{\max} = H + w\,\text{sag}$).
This is the form implemented in `atldp.core.catenary`.

### Change of state and ruling span

Across temperature/load states the unstrained length $L_0$ is conserved [1]. With
the strain decomposed into an elastic and a thermal part,

$$
\varepsilon(H, \theta) = \frac{H}{A\,E} + \alpha\,(\theta - \theta_{\text{ref}}) + \varepsilon_{\text{creep}},
\qquad
\frac{L_2}{1+\varepsilon(H_2,\theta_2)} = \frac{L_1}{1+\varepsilon(H_1,\theta_1)},
$$

with $L_i = L(H_i)$ from the closed form above. Eliminating $L_0$ between a
reference and a target state thus gives one equation for the new horizontal
tension $H_2$. Because the geometric length $L(H)$ decreases with $H$ (tighter
conductor) while the elastic stretch increases with $H$, the root $H_2$ is unique
and found by a robust bracketed solve. This is implemented in
`atldp.core.change_of_state`. The Phase 1 constitutive law is the single-modulus
linear-elastic + thermal model; the full bilinear initial/final aluminium–steel
behaviour and time/temperature **creep** [3, 4] refine $\varepsilon(H,\theta)$
without changing any caller.

A tension section sharing one $H$ is reduced to the equivalent **ruling span** [1]

$$
S_r = \sqrt{\frac{\sum_i S_i^3}{\sum_i S_i}},
$$

solved once (by `atldp.core.ruling_span`) and applied back to every real span.
This reduction is exact only under its usual assumptions (free-swinging
suspension insulators that equalise $H$, geometrically similar spans); its
accuracy degrades at high operating temperature and for strongly unequal spans
[2]. Because real structures attach the wires at different points, unequal spans
are common, so this is the entry point for the **uneven-span FEM section solver**,
brought forward to phase G11 (ADR-0021) and validated against this analytic
reduction in the equal-span limit (ADR-0003/0008).

The *boundaries* of a tension section are the **anchor (strain / dead-end)
structures**: at a suspension structure the insulator swings freely and the
horizontal tension passes through, so the section continues; at an anchor the
conductor is terminated and a new section — with, in general, its own stringing
tension (**traction**) — begins. A line is therefore a chain of sections, each one
an independent ruling-span problem with its own $S_r$ and its own $H$. Each **wire**
(every phase conductor, and the shield/ground wires, which are normally strung
tighter) likewise has its own tension and load, so the section solve above is run
per (wire, section) pair rather than once for the whole line. This is the model
realised in the product by ADR-0015 (`atldp_core::ruling_span::Section` is the
per-section kernel).

## Wind and load cases

A weather case scales the per-unit load and tilts the load plane. With self-weight
$w_c$, an ice load $w_i$ on the vertical component and a transverse wind load
$w_w$, the **resultant** load per unit length and the **swing (blow-out) angle**
$\psi$ are [13, 14]

$$
w = \sqrt{(w_c + w_i)^2 + w_w^2},
\qquad
\psi = \arctan\!\frac{w_w}{w_c + w_i},
$$

by which the conductor plane tilts out of vertical. The static shape in that swung
plane is still the catenary above, solved with the resultant $w$; clearances are
then checked on the projected geometry (wire-to-ground and between phases). Each
load case (e.g. everyday tension, maximum wind, maximum ice, minimum temperature)
is one change-of-state target sharing the section's ruling span, and the governing
limits are the design tension fractions of RTS (everyday / initial / final) and
the minimum clearances of the applicable standard [13, 14].

A complete design is verified against the **set** of load cases prescribed by a
governing standard, not a single weather. Under **IEC 60826** [14] the core
mechanical cases are: the **everyday (EDS)** long-term tension (creep / fatigue
limit); **extreme wind** (the high-wind reliability case — transverse load, swing,
and the structure transverse load it implies); **construction / maintenance**
(stringing and installation, a safety limit state); the **broken-wire / unbalanced
longitudinal** case (one or more conductors broken, an unbalanced pull that governs
anchors); and **minimum temperature** (the low-temperature high-tension case). Each
case differs in two independent ways — its weather/temperature *state* (a
change-of-state target on the section ruling span, solved by the engine above) and
the *load combination* it imposes on the structure (transverse + vertical +
longitudinal, with per-case factors). ATLDP encodes these as a pluggable **criteria
set** (ADR-0004/0017): IEC 60826 first, with ABNT NBR 5422 and others as additional
sets selectable per project, so a result traces to a named clause rather than a
magic number.

## Stringing table

The field crew strings the conductor at whatever the **ambient temperature** happens
to be on the day, yet the conductor must end up at the design tension. The
**stringing (sagging) table** is the bridge: for one tension section it tabulates,
across a range of conductor temperatures, the **sag and tension** the crew should
read once the conductor has settled at that temperature. It is produced by running
the change-of-state from the design (reference) state to each tabulated temperature
**on the section ruling span**,

$$
H(\theta_k)\ \text{from change-of-state on } S_r,
\qquad
\text{sag}_i(\theta_k) = \text{sag}\big(S_i, h_i; H(\theta_k)\big),
$$

and then projecting that common $H(\theta_k)$ back onto each real span $i$ to give the
per-span sag (and, while pulling through travelers, the offset clipping/pulley sag).
No new mechanics are involved — it is the per-section solve of the previous sections
evaluated at a sweep of temperatures and laid out for use in the field (IEEE 524
stringing practice; ATLDP emits it as a stage-6 field output, plan G14).

## Ground profile precision

Clearance is verified as the vertical gap between the lowest wire and the ground
directly beneath it, so a **metre of error in the ground profile is a metre of error
in the clearance** — directly against the normative minimum. Public DEMs are coarse
for this (≈ 30 m posting for SRTM, sampled nearest-cell), which is adequate to lay
out a route over a wide area but not to certify clearance. The right-of-way profile
under the wires is therefore refined to ~1 m by densifying the corridor and sampling
the DEM with **bilinear/bicubic interpolation** rather than nearest-cell, with
no-data cells flagged rather than zero-filled (ADR-0018). Interpolation makes the
profile continuous and consistent but cannot create accuracy the source lacks; the
ultimate ~1 m truth comes from surveyed LiDAR or contour data, for which this leaves
a clean substitution hook.

## Creep and high-temperature behaviour

The single-modulus law $\varepsilon = \sigma/E + \alpha\,\Delta\theta$ used above is
a Phase-1 simplification. Real stranded conductors — especially composite ACSR
(aluminium strands over a steel core) — are **non-linear, inelastic and
time-dependent**: the aluminium yields and creeps far more than the steel, so the
load shares between the two materials shift with both tension and temperature, and
a permanent elongation accumulates over the line's life. The industry-standard
treatment is the **experimental stress–strain–creep ("graphic") method** of the
Aluminum Association and CIGRE, in which each material is characterised by
laboratory-fitted **fourth-degree polynomials** [3, 4, 17].

### Stress–strain polynomials

With strain $\varepsilon$ expressed in percent, each component $j\in\{\text{al},\text{st}\}$
has an *initial* loading curve and a *final* (post-settlement / 10-year creep)
curve, both fourth degree:

$$
\sigma_j^{\text{init}}(\varepsilon) = \sum_{k=0}^{4} a_{j,k}\,\varepsilon^{k},
\qquad
\sigma_j^{\text{final}}(\varepsilon) = \sum_{k=0}^{4} b_{j,k}\,\varepsilon^{k}.
$$

The conventional coefficient labels are $A_0\!\ldots\!A_4$ / $B_0\!\ldots\!B_4$ for
the aluminium initial / final curves and $C_0\!\ldots\!C_4$ / $D_0\!\ldots\!D_4$ for
the steel-core curves [4]. Above the strain at which the final modulus is reached
the curve continues as the straight **final modulus** $E_{\text{final}}$. The
composite conductor force, and hence the horizontal tension, is the area-weighted
sum of the two component stresses, each evaluated at its own thermally shifted
strain,

$$
H(\varepsilon,\theta) = A_{\text{al}}\,\sigma_{\text{al}}\!\big(\varepsilon - \alpha_{\text{al}}\,\Delta\theta\big)
                       + A_{\text{st}}\,\sigma_{\text{st}}\!\big(\varepsilon - \alpha_{\text{st}}\,\Delta\theta\big),
\qquad \Delta\theta = \theta - \theta_{\text{ref}},
$$

where the operative branch (initial vs. final) is the upper envelope: the largest
stress ever experienced sets the permanent set, and the conductor unloads/reloads
along the final curve thereafter.

### Aluminium–steel load transfer and the knee point

Because $\alpha_{\text{al}} > \alpha_{\text{st}}$, heating elongates the aluminium
faster than the steel and transfers tension from the aluminium onto the core. The
aluminium stress falls with temperature and may reach zero at the **knee-point
temperature** $\theta_k$ defined by $\sigma_{\text{al}}(\cdot)=0$; above $\theta_k$
the steel core carries essentially all the tension and the conductor's effective
thermal-expansion coefficient drops toward the steel's. This knee-point behaviour
is the main reason the single-modulus model is inadequate at high operating
temperature and is central to high-temperature low-sag (HTLS) conductor design
[3, 17].

### Creep predictor

Long-term metallurgical creep of the aluminium is the permanent strain
$\varepsilon_{\text{creep}}$ entering the change-of-state law. Over the primary-creep
regime relevant to conductors it follows a power law in time with stress and
temperature dependence (CIGRE WG 22 / Harvey–Larson form) [17, 18]:

$$
\varepsilon_{\text{creep}}(\sigma,\theta,t) = K\,\sigma^{\,m}\,e^{\,\beta\theta}\,t^{\,n},
\qquad n \approx 0.16,
$$

with $\varepsilon_{\text{creep}}$ in microstrain, $t$ in hours, and $K, m, \beta,
n$ empirical coefficients tabulated per alloy/construction. Equivalently, the
*final* polynomial above is the locus the conductor reaches after this creep has
accumulated (conventionally ≈ 10 years at everyday tension), so design takes the
worse of the two permanent-elongation mechanisms — high-load **settlement** and
long-time **creep** — when predicting final sag [3, 17, 19].

In ATLDP this section is exactly the refinement anticipated in *Change of state*:
it replaces the constant $\varepsilon_{\text{creep}}$ offset and the single modulus
$E$ with the composite polynomial constitutive law, evaluated by the same
change-of-state solver without changing its interface (`atldp.core.conductor`,
Phase 2).

## Dynamics

For aeolian vibration, galloping and the dynamic response to gusts the static
catenary is perturbed. Linearising Newton's law about the catenary gives the
coupled equations of motion for the displacement components $u_x, u_y, u_z$ [6, 7]

$$
\begin{aligned}
\frac{\partial}{\partial s} \left[ (T + \tau) \left( \frac{dx}{ds} + \frac{\partial u_x}{\partial s}\right) \right] &= \rho A \frac{\partial^2 u_x}{\partial t^2} \\
\frac{\partial}{\partial s} \left[ (T + \tau) \left( \frac{dy}{ds} + \frac{\partial u_y}{\partial s}\right)\right] &= \rho A \frac{\partial^2 u_y}{\partial t^2} - \rho A g \\
\frac{\partial}{\partial s} \left[ (T + \tau) \frac{\partial u_z}{\partial s} \right] &= \rho A \frac{\partial^2 u_z}{\partial t^2}
\end{aligned}
$$

The dynamic tension $\tau$ is not independent: stretching the cable couples the
transverse motion back into the axial direction, so $\tau$ depends on an integral
of the displacement field over the span. This geometric coupling is governed by
the dimensionless **Irvine parameter** [6, 7]

$$
\lambda^2 = \left(\frac{w\,S}{H}\right)^{2} \frac{S\,E A}{H\,L_e},
$$

which sets whether the in-plane symmetric modes are taut-string-like or
sag-dominated and produces the crossover frequencies of the linear theory. The
out-of-plane and antisymmetric modes decouple from $\tau$ and reduce to the
taut-string spectrum.

### Reduced-order model (ROM)

The continuous field is projected onto a finite set of vibration modes
$\eta(s,t) = \sum_k \phi_k(s)\, q_k(t)$ (a Galerkin / modal truncation). Retaining
the quadratic and cubic terms from the cable's geometric stretching, each modal
coordinate $q$ obeys a forced, damped Duffing-type equation [8, 9, 11]

$$
\ddot{q} + \mu \dot{q} + \omega^2 q + c_2 q^2 + c_3 q^3 = f(t).
$$

The quadratic term carries the asymmetric stiffening characteristic of cable sag,
and the cubic term the large-amplitude hardening. Reducing the full
finite-element discretisation to a handful of such modal equations is the
**reduced-order model** of [11]; the underlying high-fidelity discretisation is
the robust cable finite element of [10] and the non-incremental formulation of
[12]. The same cable element is what the **static** uneven-span section solver uses
(ADR-0021, brought forward to G11); this **dynamic** ROM track stays later, behind the
static core (ADR-0003), validated against it wherever the two overlap (the static
limit, small oscillations about equilibrium).

## References

[1]&nbsp; WINKELMAN, P. F. *Sag-Tension Computations and Field Measurements of
Bonneville Power Administration.* Transactions of the AIEE, Part III, v. 78, n. 3,
p. 1532–1547, 1959.

[2]&nbsp; MOTLIS, Y. et al. *Limitations of the ruling span method for overhead
line conductors at high operating temperatures.* IEEE Transactions on Power
Delivery, v. 14, n. 2, p. 549–560, 1999.

[3]&nbsp; CIGRE Technical Brochure 324 — *Sag-tension calculation methods for
overhead lines*, 2007.

[4]&nbsp; The Aluminum Association. *Sag and Tension Calculation Methods for
Overhead Transmission Lines* (Aluminum Electrical Conductor Handbook).

[5]&nbsp; mpewsey. *Catenary / change-of-state worked example.*
<https://mpewsey.github.io/2021/12/17/sag-tension-algorithm.html>

[6]&nbsp; IRVINE, H. Max; CAUGHEY, T. K. *The linear theory of free vibrations of
a suspended cable.* Proceedings of the Royal Society of London A, v. 341, n. 1626,
p. 299–315, 1974.

[7]&nbsp; IRVINE, H. Max. *Cable Structures.* Cambridge: MIT Press, 1981.

[8]&nbsp; HAGEDORN, P.; SCHÄFER, B. *On non-linear free vibrations of an elastic
cable.* International Journal of Non-Linear Mechanics, v. 15, n. 4–5, p. 333–340,
1980.

[9]&nbsp; JAFARI, M.; HOU, F.; ABDELKEFI, A. *Wind-induced vibration of
structural cables.* Nonlinear Dynamics, v. 100, p. 351–421, 2020.

[10]&nbsp; BERTRAND, Charlélie et al. *A robust and efficient numerical finite
element method for cables.* International Journal for Numerical Methods in
Engineering, v. 121, n. 18, p. 4157–4186, 2020.

[11]&nbsp; BERTRAND, Charlélie et al. *Reduced-Order Model for the Nonlinear
Dynamics of Cables.* Journal of Engineering Mechanics, v. 148, n. 9, p. 04022052,
2022.

[12]&nbsp; SUGIYAMA, Hiroyuki; MIKKOLA, Aki M.; SHABANA, Ahmed A. *A
non-incremental nonlinear finite element solution for cable problems.* Proc. ASME
IDETC/CIE, 2003, p. 171–181.

[13]&nbsp; ABNT NBR 5422 — *Projeto de linhas aéreas de transmissão de energia
elétrica.*

[14]&nbsp; IEC 60826 — *Design criteria of overhead transmission lines.*

[15]&nbsp; IEEE Std 738 — *Calculating the Current-Temperature Relationship of
Bare Overhead Conductors.*

[16]&nbsp; CIGRE Technical Brochure 601 — *Guide for thermal rating calculations
of overhead lines*, 2014.

[17]&nbsp; HARVEY, J. R.; LARSON, R. E. *Use of Elevated-Temperature Creep Data in
Sag-Tension Calculations.* IEEE Transactions on Power Apparatus and Systems,
v. PAS-89, n. 3, p. 380–386, 1970.

[18]&nbsp; CIGRE Working Group 22.05. *Permanent elongation of conductors —
predictor equations and evaluation methods.* Electra, n. 75, p. 63–98, 1981.

[19]&nbsp; BRADBURY, J.; DEY, P.; ORAWSKI, G.; PICKUP, K. H. *Long-term-creep
assessment for overhead-line conductors.* Proceedings of the IEE, v. 122, n. 10,
p. 1146–1152, 1975.

A fuller bibliography (including software and reference repositories) is kept in
[`references.md`](../references.md).
