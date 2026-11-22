# Part 1: Phase 4 Technical Specification (`PHASE4_PHYSICS_SPEC.md`)

## 1. 1D Soil Thermal Model (`ThermalModel`)

### 1.1 Governing Equation

Replace the static `soilAirOffset` (+2.5 °C) with a 1D vertical heat conduction equation with phase change on a fixed, stack-allocated node grid ($N = 16$ nodes from $z = 0\text{ m}$ to $z_{\max} = 5.0\text{ m}$, stretched geometrically so resolution is finest in the top $1.0\text{ m}$):

$$C_{\text{app}}(z, t) \frac{\partial T}{\partial t} = \frac{\partial}{\partial z} \left( \lambda(z, t) \frac{\partial T}{\partial z} \right)$$

### 1.2 Thermal Properties (Coupled to Moisture & Texture)

For each node $i$ with porosity $\phi_i$, organic matter volume fraction $f_{\text{om}, i}$, liquid water content $\theta_{l, i}$, and ice content $\theta_{i, i}$:

1. **Volumetric Heat Capacity** (de Vries, 1963):

$$C_v = C_m (1 - \phi - f_{\text{om}}) + C_{\text{om}} f_{\text{om}} + C_w \theta_l + C_i \theta_i$$



Constants ($\text{J m}^{-3}\text{ K}^{-1}$): $C_m = 1.94 \times 10^6$, $C_{\text{om}} = 2.50 \times 10^6$, $C_w = 4.18 \times 10^6$, $C_i = 1.93 \times 10^6$.
2. **Apparent Heat Capacity (Latent Heat of Fusion):**
To avoid discontinuities at $T_f = 0^\circ\text{C}$, use a smooth unfrozen water curve $\theta_l(T) = \theta_{\text{tot}} \cdot \frac{1}{2}\left(1 + \tanh\left(\frac{T - T_f}{\Delta T_f}\right)\right)$ with smoothing width $\Delta T_f = 0.5\text{ K}$:

$$C_{\text{app}} = C_v + \rho_w L_f \frac{\partial \theta_l}{\partial T}, \quad L_f = 3.34 \times 10^5\text{ J kg}^{-1}$$


3. **Thermal Conductivity** (Johansen, 1975 / Côté & Konrad, 2005):

$$\lambda = (\lambda_{\text{sat}} - \lambda_{\text{dry}}) K_e + \lambda_{\text{dry}}$$


* $\lambda_{\text{dry}} = \frac{0.135 \rho_b + 64.7}{2700 - 0.947 \rho_b}$ (mineral soils, $\rho_b$ in $\text{kg m}^{-3}$); for O-horizons, use $\lambda_{\text{dry}, O} = 0.05\text{ W m}^{-1}\text{ K}^{-1}$.
* $\lambda_{\text{sat}} = \lambda_s^{1-\phi} \lambda_w^{\theta_l} \lambda_i^{\theta_i}$ ($\lambda_w = 0.57$, $\lambda_i = 2.2$, $\lambda_s$ from quartz fraction).
* Normalized Kersten number $K_e = \frac{\kappa S_r}{1 + (\kappa - 1) S_r}$ where $S_r = (\theta_l + \theta_i)/\phi$ and $\kappa$ is texture-dependent (e.g., $4.6$ coarse, $3.55$ medium, $1.9$ fine, $0.6$ organic).



### 1.3 Boundary Conditions

* **Top Boundary ($z = 0$, Robin / Thermal Resistance):**
Model snowpack ($d_{\text{snow}}, \lambda_{\text{snow}}$) and surface O-horizon ($d_O, \lambda_O$) as series thermal resistors between air temperature $T_{\text{air}}(t)$ and the top mineral node $T_0$:

$$R_{\text{surf}} = \frac{d_{\text{snow}}}{\lambda_{\text{snow}}} + \frac{d_O}{\lambda_O} + \frac{1}{h_c}, \qquad q_0 = \frac{T_{\text{air}}(t) - T_0}{R_{\text{surf}}}$$



where $h_c \approx 15\text{ W m}^{-2}\text{ K}^{-1}$ is the surface convective transfer coefficient.
* **Bottom Boundary ($z = z_{\max} = 5.0\text{ m}$):**
Zero heat flux (Neumann): $\left.\frac{\partial T}{\partial z}\right\vert{}_{z_{\max}} = 0$.

### 1.4 Numerical Solver (Zero-Allocation)

* **Discretization:** Fully implicit **Backward Euler** in time (unconditionally stable for any $\Delta t$, preventing Crank-Nicolson oscillations under sharp snow/freeze transitions).
* **Linear Solver:** Tridiagonal **Thomas Algorithm** operating on stack-allocated arrays (`[f64; 16]`), $O(N)$ per step, zero heap allocations.
* **Spin-Up:** Run 3–5 annual cycles (or until $\max_i \vert{}T_i^{(y)} - T_i^{(y-1)}\vert{} < 10^{-3}\text{ K}$) before recording Soil Taxonomy metrics.
* **Taxonomy Extraction:** Linearly interpolate $T(z = 0.50\text{ m})$ at each step to compute MAST, MSST, MWST, and biological-zero ($5^\circ\text{C}$) windows.

---

## 2. Fluctuating Water-Table Boundary Condition (`HydrologyStep`)

### 2.1 Depth Mapping to the Newhall 8×8 Matrix

Map each row $r \in \{0, \dots, 7\}$ of the Newhall 8×8 profile to a physical depth interval $[z_r, z_{r+1}]$ based on total profile/rooting depth $Z_{\text{prof}}$ (default $1.0\text{ m}$ or user-supplied restrictive-layer depth).

### 2.2 Water-Table Forcing $z_{\text{wt}}(t)$

Support two modes via an enum on `BoundaryCondition`:

1. **Prescribed:** Monthly or daily $z_{\text{wt}}(t)$ from input arrays or a seasonal harmonic:

$$z_{\text{wt}}(t) = \bar{z}_{\text{wt}} - A_{\text{wt}} \cos\left(\frac{2\pi (t - t_{\text{peak}})}{365}\right)$$


2. **Prognostic (Perched / Restricted Drainage):** Excess drainage $D_{\text{excess}}$ from the bottom of the 8×8 matrix accumulates above an impermeable boundary $z_{\text{imp}}$ with drainable porosity $S_y$ and lateral recession constant $k_{\text{lat}}$:

$$\frac{d h_{\text{wt}}}{dt} = \frac{D_{\text{excess}} - q_{\text{cap}}}{S_y} - k_{\text{lat}} h_{\text{wt}}, \qquad z_{\text{wt}} = z_{\text{imp}} - h_{\text{wt}}$$



### 2.3 Saturated Zone & Capillary Rise

At each time step, after standard Newhall infiltration/extraction:

1. **Saturation Override ($z_{r+1} \ge z_{\text{wt}}$):**
* For any row fully below $z_{\text{wt}}$, set all 8 columns to full AWC (`row_awc_max`) and mark the row as saturated.
* For a row straddling $z_{\text{wt}}$, linearly scale the minimum moisture floor by the submerged fraction $f_{\text{sub}} = \frac{z_{r+1} - z_{\text{wt}}}{z_{r+1} - z_r}$.


2. **Upward Capillary Flux ($z_r < z_{\text{wt}}$):**
Compute maximum steady-state upward flux from the water table to the unsaturated row centroid $\bar{z}_r$ using the Gardner (1958) relation:

$$q_{\text{cap}, \max}(\Delta z) = \frac{K_{\text{sat}} \cdot a}{\Delta z^n + b}, \qquad \Delta z = z_{\text{wt}} - \bar{z}_r$$



where $a, b, n$ are texture-class parameters (coarse, medium, fine).
* Actual recharge added to row $r$ is bounded by the row's moisture deficit and remaining atmospheric demand:

$$\Delta W_r = \min\left( q_{\text{cap}, \max}(\Delta z) \cdot \Delta t, \; W_{\text{max}, r} - W_r \right)$$


* Distribute $\Delta W_r$ across the row's columns from driest to wettest (or uniformly) before evaluating moisture control section (MCS) dry/moist states.



### 2.4 Aquic & Redox Diagnostics

Track two additional counters in `SiteOutput`:

* `sat_days_mcs`: Cumulative days where $z_{\text{wt}}$ is within the MCS.
* `aquic_bio_days`: Consecutive days where $z_{\text{wt}} \le 0.50\text{ m}$ **and** $T(0.50\text{ m}) > 5.0^\circ\text{C}$.

