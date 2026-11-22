Run this suite after Phase 4 is implemented to expose numerical instabilities, energy/mass leaks, and Soil Taxonomy edge-case failures.

## 1. Conservation & Thermodynamic Invariants

* **Thermal Energy Conservation:** With $q_0 = 0$ (insulated top) and $\partial T / \partial z\vert{}_{z_{\max}} = 0$, initialize a sharp step temperature profile ($20^\circ\text{C}$ top half, $0^\circ\text{C}$ bottom half). Verify total enthalpy $H = \sum_i \left(C_{v, i} T_i + \rho_w L_f \theta_{l, i}\right) \Delta z_i$ is conserved to `< 1e-6` over 1,000 steps.
* **Second Law / Monotonicity Check:** Under constant $T_{\text{air}}$ with no internal heat source, no node may exceed $\max(T_{\text{air}}, \max_i T_i(0))$ or drop below $\min(T_{\text{air}}, \min_i T_i(0))$.
* **Water Mass Balance:** For the prognostic water-table mode, verify:

$$\Delta S_{\text{matrix}} + S_y \Delta h_{\text{wt}} = P - \text{AET} - Q_{\text{lat}} - R_{\text{surf}}$$



to within `1e-9` at every step.

## 2. Analytical & Limiting-Case Degeneracy Tests

* **Carslaw & Jaeger Harmonic Damping Benchmark:** Set uniform $\lambda, C_v$, no phase change, $d_{\text{snow}} = d_O = 0$, $h_c \to \infty$, and sinusoidal $T_{\text{air}}(t) = \bar{T} + A \sin(\omega t)$. Assert numerical $T(z, t)$ matches the exact analytical solution:

$$T(z, t) = \bar{T} + A e^{-z / d} \sin\left(\omega t - \frac{z}{d}\right), \quad d = \sqrt{\frac{2 \lambda}{\omega C_v}}$$



within 1% amplitude and phase error at $z = 0.50\text{ m}$.
* **Classic Newhall Recovery:** Set $z_{\text{wt}} = 100\text{ m}$ (deep water table) and $q_{\text{cap}} = 0$. Assert **100% bit-for-bit or `1e-9` parity** with the Phase 2 classic hydrology engine.
* **Zero-Thickness Insulation Limit:** Assert that $d_{\text{snow}} = 0, d_O = 0$ produces identical temperatures to the bare-soil boundary condition (no division-by-zero in $R_{\text{surf}}$).

## 3. Numerical Stability & Stiffness Torture Tests

* **Extreme Conductivity Contrast:** Stack $1.5\text{ m}$ of low-density snow ($\lambda_{\text{snow}} = 0.05$) over saturated mineral soil ($\lambda = 2.5$, 50× jump) with a $-30^\circ\text{C}$ air-temperature shock. Verify no oscillations, `NaN`, or negative absolute temperatures.
* **Freeze-Thaw Chatter:** Oscillate $T_{\text{air}}$ rapidly between $-2^\circ\text{C}$ and $+2^\circ\text{C}$ daily for 365 days. Verify the apparent heat capacity solver does not "skip over" the latent heat peak (check against a 10× finer $\Delta t$ reference run).
* **Water-Table Step Shock:** Drop $z_{\text{wt}}$ instantaneously from $0.0\text{ m}$ to $3.0\text{ m}$, then back to $0.0\text{ m}$ in consecutive steps. Verify moisture never goes negative, never exceeds `row_awc_max`, and capillary rise never pulls water above saturation.
* **Degenerate Inputs:** Test $\phi = 0$, $\rho_b = 0$, $\text{AWC} = 0$, $K_{\text{sat}} = 0$, $z_{\text{wt}} < 0$ (ponded surface), and `NaN` in a single month of forcing.

## 4. Property-Based Fuzzing (`proptest`) on Soil Taxonomy Boundaries

Use Rust's `proptest` crate to generate 10,000+ randomized climate/soil profiles specifically clustered around Soil Taxonomy regime thresholds:

* **Temperature Boundaries:** MAST near $0^\circ\text{C}$ (Gelic/Cryic), $8^\circ\text{C}$ (Cryic/Frigid/Mesic), $15^\circ\text{C}$ (Mesic/Thermic), $22^\circ\text{C}$ (Thermic/Hyperthermic), and $\text{MSST} - \text{MWST}$ near $6^\circ\text{C}$ (Iso-regimes).
* **Invariants to assert:**
* Increasing $d_{\text{snow}}$ in winter **never lowers** MWST when $T_{\text{air}} < T_{\text{soil}}$.
* Raising $z_{\text{wt}}$ **never increases** cumulative dry days in the MCS.
* Zero panics, zero `NaN` outputs on valid finite inputs, and deterministic output across single-threaded vs. `rayon` parallel execution.



## 5. Empirical Station-Level Reality Checks

Validate against observed 50 cm soil temperature and moisture from **USDA-NRCS SCAN**, **SNOTEL**, and **USCRN** stations across four contrasting stress regimes:

1. **Deep Winter Snowpack (e.g., Sierra Nevada / Rockies SNOTEL):** Verify snow insulation keeps $T_{50}$ near $0^\circ\text{C}$ despite $-15^\circ\text{C}$ air temperatures (where classic $+2.5^\circ\text{C}$ offset fails badly).
2. **Permafrost / Boreal O-Horizon (Alaska SCAN):** Verify O-horizon + latent heat dampens summer thaw depth.
3. **High Spring Water Table (Midwest Mollisol SCAN):** Verify capillary rise and saturation prevent false Ustic/dry-day classifications during spring drawdown.
4. **Arid Bare Soil (Southwest USCRN):** Verify dry low-$\lambda$ surface soil reproduces high summer MAST/MSST without needing an arbitrary offset.
