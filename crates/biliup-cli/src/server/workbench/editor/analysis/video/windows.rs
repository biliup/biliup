//! Uncut video/windows.ts (MIT): Gaussian-weighted quadratic camera smoothing.
use serde::Serialize;
use tokio_util::sync::CancellationToken;
#[derive(Clone, Copy)]
pub struct Sample {
    pub time: f64,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
    pub blur: Option<f64>,
    pub brightness: f64,
    pub dark: f64,
    pub bright: f64,
}
#[derive(Serialize)]
pub struct Window {
    start: usize,
    end: f64,
    frames: usize,
    jitter: Option<f64>,
    motion: Option<f64>,
    tracked: f64,
    blur: Option<f64>,
    brightness: f64,
    dark: f64,
    bright: f64,
    frozen: f64,
}
pub fn round(value: f64, digits: i32) -> f64 {
    let f = 10f64.powi(digits);
    (value * f).round() / f
}
fn smooth(values: &[f64], sigma: f64, cancel: &CancellationToken) -> Result<Vec<f64>, String> {
    if sigma <= 0. || values.len() < 4 {
        return Ok(values.to_vec());
    }
    let n = values.len();
    let radius = (sigma * 3.).ceil() as isize;
    if radius > 10000 {
        return Err("素材帧率超出分析范围".into());
    }
    let kernel: Vec<_> = (-radius..=radius)
        .map(|k| (-(k * k) as f64 / (2. * sigma * sigma)).exp())
        .collect();
    let mut out = vec![0.; n];
    for i in 0..n {
        if i % 1024 == 0 {
            super::check(cancel)?;
        }
        let (mut s0, mut s1, mut s2, mut s3, mut s4, mut t0, mut t1, mut t2) =
            (0., 0., 0., 0., 0., 0., 0., 0.);
        for k in (-radius).max(-(i as isize))..=radius.min((n - 1 - i) as isize) {
            let w = kernel[(k + radius) as usize];
            let y = values[(i as isize + k) as usize];
            let k = k as f64;
            let k2 = k * k;
            s0 += w;
            s1 += w * k;
            s2 += w * k2;
            s3 += w * k2 * k;
            s4 += w * k2 * k2;
            t0 += w * y;
            t1 += w * k * y;
            t2 += w * k2 * y;
        }
        let det = s0 * (s2 * s4 - s3 * s3) - s1 * (s1 * s4 - s3 * s2) + s2 * (s1 * s3 - s2 * s2);
        let da = t0 * (s2 * s4 - s3 * s3) - s1 * (t1 * s4 - s3 * t2) + s2 * (t1 * s3 - s2 * t2);
        out[i] = if det.abs() > 1e-9 * (s0 * s2 * s4).max(1.) {
            da / det
        } else {
            t0 / s0
        };
    }
    Ok(out)
}
pub fn windows(
    samples: &[Sample],
    frozen: &[(f64, f64)],
    patch_width: f64,
    end: f64,
    cancel: &CancellationToken,
) -> Result<Vec<Window>, String> {
    let n = samples.len();
    let fps = if n > 1 && end > 0. {
        n as f64 / end
    } else {
        30.
    };
    let sigma = 0.15 * fps;
    let mut residual = vec![f64::NAN; n];
    let mut speed = vec![f64::NAN; n];
    let mut i = 0;
    while i < n {
        super::check(cancel)?;
        if samples.get(i + 1).is_none_or(|s| s.dx.is_none()) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < n && samples[j].dx.is_some() {
            j += 1;
        }
        let len = j - i;
        let mut px = vec![0.; len];
        let mut py = vec![0.; len];
        for k in 1..len {
            px[k] = px[k - 1] + samples[i + k].dx.unwrap();
            py[k] = py[k - 1] + samples[i + k].dy.unwrap();
        }
        let sx = smooth(&px, sigma, cancel)?;
        let sy = smooth(&py, sigma, cancel)?;
        for k in 0..len {
            residual[i + k] = (px[k] - sx[k]).hypot(py[k] - sy[k]) / patch_width;
            if k > 0 {
                speed[i + k] = (sx[k] - sx[k - 1]).hypot(sy[k] - sy[k - 1])
                    / patch_width
                    / (samples[i + k].time - samples[i + k - 1].time).max(1e-6);
            }
        }
        i = j;
    }
    let seconds = (end - 1e-6).ceil().max(1.) as usize;
    let mut out = vec![];
    let mut f = 0;
    let mut frozen_at = 0;
    for second in 0..seconds {
        if second % 64 == 0 {
            super::check(cancel)?;
        }
        let w_end = end.min(second as f64 + 1.);
        let (mut frames, mut tracked, mut jit_n, mut mot_n, mut blur_n, mut froz) =
            (0, 0, 0, 0, 0, 0);
        let (mut jit, mut mot, mut blur, mut bri, mut dark, mut bright) = (0., 0., 0., 0., 0., 0.);
        while f < n && samples[f].time < w_end {
            let s = &samples[f];
            frames += 1;
            if s.dx.is_some() {
                tracked += 1;
            }
            if residual[f].is_finite() {
                jit += residual[f].powi(2);
                jit_n += 1;
            }
            if speed[f].is_finite() {
                mot += speed[f];
                mot_n += 1;
            }
            if let Some(b) = s.blur.filter(|b| b.is_finite()) {
                blur += b;
                blur_n += 1;
            }
            bri += s.brightness;
            dark += s.dark;
            bright += s.bright;
            while frozen_at < frozen.len() && frozen[frozen_at].1 <= s.time {
                frozen_at += 1;
            }
            if frozen
                .get(frozen_at)
                .is_some_and(|&(start, end)| s.time >= start && s.time < end)
            {
                froz += 1;
            }
            f += 1;
        }
        if frames == 0 {
            continue;
        }
        out.push(Window {
            start: second,
            end: round(w_end, 3),
            frames,
            jitter: (jit_n > 0).then(|| round((jit / jit_n as f64).sqrt(), 5)),
            motion: (mot_n > 0).then(|| round(mot / mot_n as f64, 4)),
            tracked: round(
                tracked as f64 / (frames - usize::from(second == 0)).max(1) as f64,
                3,
            ),
            blur: (blur_n > 0).then(|| round(blur / blur_n as f64, 3)),
            brightness: round(bri / frames as f64, 3),
            dark: round(dark / frames as f64, 4),
            bright: round(bright / frames as f64, 4),
            frozen: round(froz as f64 / frames as f64, 3),
        });
    }
    Ok(out)
}
