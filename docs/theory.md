# The sag-tension problem

*Carlos Kleber C. Arruda*

> **Abstract.** Just sketching the problem in some pretty equations...

## Model

**Premises:**

- Span depth ratio > 1/8, or inclined supports: the exact solution must be used.
- Span depth ratio < 1/8 and aligned supports: the parabolic approximation can be used.

**Approaches:**

- Stationary
- Eigenfrequency (vibration)
- Time dependent

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
