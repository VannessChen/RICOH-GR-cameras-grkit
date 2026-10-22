//! The solvers of the fit: banded Cholesky, curve least squares, and Levenberg-Marquardt.

// cube_fit.py: 2nd-difference smoothing of the curves, gray and chart weights.
const SMOOTHING: f64 = 3000.0;
pub(super) const STEP: f64 = 1e-6;

/// Solves a symmetric positive definite system whose nonzeros lie within `band` of the diagonal.
fn solve(n: usize, band: usize, mut a: Vec<f64>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    for j in 0..n {
        let first = j.saturating_sub(band);
        let d = a[j * n + j] - (first..j).map(|k| a[j * n + k].powi(2)).sum::<f64>();
        if d.is_nan() || d <= 0.0 {
            return None;
        }
        let d = d.sqrt();
        a[j * n + j] = d;
        for i in j + 1..n.min(j + band + 1) {
            let start = i.saturating_sub(band).max(first);
            let s: f64 = (start..j).map(|k| a[i * n + k] * a[j * n + k]).sum();
            a[i * n + j] = (a[i * n + j] - s) / d;
        }
    }
    for i in 0..n {
        let s: f64 = (i.saturating_sub(band)..i)
            .map(|k| a[i * n + k] * b[k])
            .sum();
        b[i] = (b[i] - s) / a[i * n + i];
    }
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n.min(i + band + 1))
            .map(|k| a[k * n + i] * b[k])
            .sum();
        b[i] = (b[i] - s) / a[i * n + i];
    }
    b.iter().all(|v| v.is_finite()).then_some(b)
}

/// cube_fit.py `fit_curve`: weighted least squares of a 256-knot curve over g / 64 with
/// 2nd-difference smoothing, then clipped and made non-decreasing.
pub(super) fn fit_curve(levels: impl Iterator<Item = (f64, f64, f64)>) -> [f64; 256] {
    let mut a = vec![0.0; 256 * 256];
    let mut b = vec![0.0; 256];
    for (level, target, weight) in levels {
        let x = (level / 64.0).clamp(0.0, 255.0);
        let low = (x as usize).min(254);
        let t = x - low as f64;
        let pairs = [(low, 1.0 - t), (low + 1, t)];
        for (i, u) in pairs {
            for (j, v) in pairs {
                a[i * 256 + j] += weight * u * v;
            }
            b[i] += weight * u * target * 16384.0;
        }
    }
    for row in 0..254 {
        let d = [(row, 1.0), (row + 1, -2.0), (row + 2, 1.0)];
        for (i, u) in d {
            for (j, v) in d {
                a[i * 256 + j] += SMOOTHING * u * v;
            }
        }
    }
    for i in 0..256 {
        a[i * 257] += 1e-6;
    }
    let solution = solve(256, 2, a, b).expect("the regularized curve system is positive definite");
    let mut result = [0.0; 256];
    let mut floor = 0.0f64;
    for (slot, value) in result.iter_mut().zip(solution) {
        floor = floor.max(value.clamp(0.0, 16383.0));
        *slot = floor;
    }
    result
}

fn sum_squares(values: &[f64]) -> f64 {
    values.iter().map(|v| v * v).sum()
}

/// Levenberg-Marquardt with a forward-difference Jacobian.
pub(super) fn minimize(
    start: &[f64],
    iterations: usize,
    residuals: &dyn Fn(&[f64], &mut Vec<f64>),
) -> Vec<f64> {
    minimize_with(start, iterations, residuals, &|x, r, columns| {
        for (j, column) in columns.iter_mut().enumerate() {
            let mut y = x.to_vec();
            y[j] += STEP;
            residuals(&y, column);
            for (value, base) in column.iter_mut().zip(r) {
                *value = (*value - base) / STEP;
            }
        }
    })
}

/// `jacobian(x, r, columns)` fills one column per parameter at `x`, where the residuals are `r`.
type Jacobian<'a> = dyn Fn(&[f64], &[f64], &mut [Vec<f64>]) + 'a;

/// Levenberg-Marquardt with the Jacobian of `jacobian`.
pub(super) fn minimize_with(
    start: &[f64],
    iterations: usize,
    residuals: &dyn Fn(&[f64], &mut Vec<f64>),
    jacobian: &Jacobian,
) -> Vec<f64> {
    let n = start.len();
    let mut x = start.to_vec();
    let (mut r, mut trial) = (Vec::new(), Vec::new());
    residuals(&x, &mut r);
    let mut cost = sum_squares(&r);
    let mut lambda = 1e-3;
    let mut columns = vec![Vec::new(); n];
    for _ in 0..iterations {
        jacobian(&x, &r, &mut columns);
        // A column of a sector coefficient is zero outside its sector. A product over the
        // nonzero rows of the sparser column, in row order, adds the same terms as a dense one.
        let nonzero: Vec<Vec<usize>> = columns
            .iter()
            .map(|c| (0..c.len()).filter(|&i| c[i] != 0.0).collect())
            .collect();
        let dot =
            |a: usize, b: &[f64]| -> f64 { nonzero[a].iter().map(|&i| columns[a][i] * b[i]).sum() };
        let mut normal = vec![0.0; n * n];
        let mut gradient = vec![0.0; n];
        for a in 0..n {
            gradient[a] = -dot(a, &r);
            for b in 0..=a {
                let (sparse, other) = if nonzero[a].len() <= nonzero[b].len() {
                    (a, b)
                } else {
                    (b, a)
                };
                let value = dot(sparse, &columns[other]);
                normal[a * n + b] = value;
                normal[b * n + a] = value;
            }
        }
        let mut improved = false;
        while lambda < 1e10 {
            let mut damped = normal.clone();
            for a in 0..n {
                damped[a * n + a] += lambda * normal[a * n + a].max(1e-12);
            }
            if let Some(delta) = solve(n, n, damped, gradient.clone()) {
                let y: Vec<f64> = x.iter().zip(&delta).map(|(x, d)| x + d).collect();
                residuals(&y, &mut trial);
                let next = sum_squares(&trial);
                if next < cost {
                    let gain = (cost - next) / cost;
                    (x, cost) = (y, next);
                    std::mem::swap(&mut r, &mut trial);
                    lambda = (lambda / 3.0).max(1e-9);
                    improved = gain > 1e-10;
                    break;
                }
            }
            lambda *= 4.0;
        }
        if !improved {
            break;
        }
    }
    x
}
