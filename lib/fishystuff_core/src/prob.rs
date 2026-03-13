pub fn js_divergence(p: &[f64], q: &[f64]) -> f64 {
    assert_eq!(p.len(), q.len(), "length mismatch");
    let mut m = Vec::with_capacity(p.len());
    for (&pi, &qi) in p.iter().zip(q.iter()) {
        m.push(0.5 * (pi + qi));
    }
    0.5 * kl_divergence(p, &m) + 0.5 * kl_divergence(q, &m)
}

fn kl_divergence(p: &[f64], q: &[f64]) -> f64 {
    p.iter()
        .zip(q.iter())
        .filter(|(&pi, _)| pi > 0.0)
        .map(|(&pi, &qi)| {
            let ratio = pi / qi;
            pi * ratio.ln()
        })
        .sum()
}

pub fn dirichlet_posterior_mean(alpha0: f64, p0: &[f64], counts: &[f64]) -> Vec<f64> {
    assert_eq!(p0.len(), counts.len(), "length mismatch");
    let sum_p0: f64 = p0.iter().sum();
    let norm_p0: Vec<f64> = if sum_p0 > 0.0 {
        p0.iter().map(|v| v / sum_p0).collect()
    } else {
        vec![1.0 / p0.len() as f64; p0.len()]
    };
    let sum_counts: f64 = counts.iter().sum();
    let total = alpha0 + sum_counts;
    norm_p0
        .iter()
        .zip(counts.iter())
        .map(|(&p0_i, &c_i)| (alpha0 * p0_i + c_i) / total)
        .collect()
}
