# The sag-tension problem

*Carlos Kleber C. Arruda*

> **Abstract.** Just sketching the problem in some pretty equations...

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
- **Higher-fidelity coupling.** When ruling-span assumptions break down (strongly
  uneven/inclined spans, longitudinal coupling, dynamics), the complete models use
  the **finite element method** (Bertrand 2020/2022; Sugiyama 2003). That is the
  later FEM track (ADR-0003), validated against the analytic core where they
  overlap.

The analytic core therefore works in 3D coordinates from the start, reduces each
span to a horizontal distance `S` and elevation difference `h`, and solves the
inclined catenary rather than assuming level supports.

## Model

**Premises:**

- Span depth ratio > 1/8, or inclined supports: the exact solution must be used.
- Span depth ratio < 1/8 and aligned supports: the parabolic approximation can be used.

Cable equation for a level span:

$$
\begin{aligned}
\frac{\partial}{\partial s} \left( T \frac{dx}{ds}\right) &= 0 \\
\frac{\partial}{\partial s} \left( T \frac{dy}{ds}\right) &= -mg
\end{aligned}
$$

For small sags, it is approximated by the parabola:

$$
y = \frac{m g l^2}{2 H} \left[ \frac{x}{l} - \left( \frac{x}{l} \right)^2 \right]
$$

### Inclined (uneven) span — exact catenary

With horizontal distance $S$, elevation difference $h$ between the supports, load
per unit length $w$, and horizontal tension $H$, the catenary constant is
$c = H/w$. Placing support 1 at $(0,0)$ and support 2 at $(S,h)$, the curve is

$$
y(x) = c\,\cosh\!\frac{x-a}{c} - c\,\cosh\!\frac{a}{c},
\qquad
a = \frac{S}{2} - c\,\operatorname{arcsinh}\!\frac{h}{2c\,\sinh\!\frac{S}{2c}},
$$

where $a$ is the abscissa of the low point (which falls outside $[0,S]$ for steep
inclines). The conductor length has the closed form

$$
L = \sqrt{\,h^2 + \left(2c\,\sinh\!\frac{S}{2c}\right)^2}\,,
$$

and the tension is $T(x) = H\cosh\!\frac{x-a}{c}$, largest at the higher support.
For $h = 0$ these reduce to the level-span results ($a = S/2$,
$L = 2c\sinh\frac{S}{2c}$, sag $= c(\cosh\frac{S}{2c}-1)$, $T_\max = H + w\,\text{sag}$).
This is the form implemented in `atldp.core.catenary`.

### Change of state and ruling span

Across temperature/load states the unstrained length $L_0$ is conserved, with
$L = L_0\,(1 + \sigma/E + \alpha\,(T-T_{\text{ref}}))$. Eliminating $L_0$ between a
reference state and a target state and substituting $L = L(H)$ above gives one
equation for the new horizontal tension $H$. A tension section sharing one $H$ is
reduced to the equivalent **ruling span** $S_r = \sqrt{\sum S_i^3 / \sum S_i}$,
solved once and applied back to every real span.

$$
\begin{aligned}
\frac{\partial}{\partial s} \left[ (T + \tau) \left( \frac{dx}{ds} + \frac{\partial u}{\partial s}\right) \right] &= \rho A \frac{\partial^2 u}{\partial t^2} \\
\frac{\partial}{\partial s} \left[ (T + \tau) \left( \frac{dy}{ds} + \frac{\partial v}{\partial s}\right)\right] &= \rho A \frac{\partial^2 v}{\partial t^2} - \rho A g \\
\frac{\partial}{\partial s} \left[ (T + \tau) \frac{\partial w}{\partial s} \right] &= \rho A \frac{\partial^2 w}{\partial t^2}
\end{aligned}
$$

$$
\ddot{q} + \mu \dot{q} + q + c_2 q^2 + c_3 q^3 = f(t)
$$

where $q$ is the modal coordinate and $f$ the arbitrary external force vector.

Reduced order model (ROM).
