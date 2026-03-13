pub fn gaussian_kernel_1d(sigma: f32) -> Vec<f32> {
    if sigma <= 0.0 {
        return vec![1.0];
    }
    let radius = (sigma * 3.0).ceil() as i32;
    let mut kernel = Vec::with_capacity((2 * radius + 1) as usize);
    let mut sum = 0.0f32;
    for i in -radius..=radius {
        let x = i as f32;
        let v = (-0.5 * (x / sigma).powi(2)).exp();
        kernel.push(v);
        sum += v;
    }
    for v in &mut kernel {
        *v /= sum;
    }
    kernel
}

fn clamp_i32(v: i32, min: i32, max: i32) -> i32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

pub fn gaussian_blur_grid(input: &[f32], width: usize, height: usize, sigma: f32) -> Vec<f32> {
    if width == 0 || height == 0 {
        return Vec::new();
    }
    if sigma <= 0.0 {
        return input.to_vec();
    }
    let kernel = gaussian_kernel_1d(sigma);
    let radius = (kernel.len() as i32 - 1) / 2;
    let mut tmp = vec![0.0f32; input.len()];
    let mut out = vec![0.0f32; input.len()];

    // Horizontal pass
    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let mut acc = 0.0f32;
            for (k, weight) in kernel.iter().enumerate() {
                let dx = k as i32 - radius;
                let sx = clamp_i32(x as i32 + dx, 0, width as i32 - 1) as usize;
                acc += input[row + sx] * weight;
            }
            tmp[row + x] = acc;
        }
    }

    // Vertical pass
    for y in 0..height {
        for x in 0..width {
            let mut acc = 0.0f32;
            for (k, weight) in kernel.iter().enumerate() {
                let dy = k as i32 - radius;
                let sy = clamp_i32(y as i32 + dy, 0, height as i32 - 1) as usize;
                acc += tmp[sy * width + x] * weight;
            }
            out[y * width + x] = acc;
        }
    }

    out
}
