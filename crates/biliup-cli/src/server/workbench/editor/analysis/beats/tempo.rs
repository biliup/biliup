//! Uncut analysis/beats/tempo.ts, MIT: autocorrelation, metrical level and dynamic programming.
use tokio_util::sync::CancellationToken;
pub const RATE: f64 = 11025. / 128.;
pub fn check(cancel: &CancellationToken) -> Result<(), String> {
    if cancel.is_cancelled() {
        Err("分析已取消".into())
    } else {
        Ok(())
    }
}
pub fn vertex(a: f64, b: f64, c: f64) -> f64 {
    let d = a - 2. * b + c;
    if d < 0. {
        (0.5 * (a - c) / d).clamp(-0.5, 0.5)
    } else {
        0.
    }
}
pub fn quantile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((q * (sorted.len() - 1) as f64).floor() as usize).min(sorted.len() - 1)]
}
fn at(a: &[f64], x: f64) -> f64 {
    if x < 0. {
        return 0.;
    }
    let i = x.floor() as usize;
    if i >= a.len() {
        0.
    } else if i + 1 == a.len() {
        a[i]
    } else {
        a[i] + (a[i + 1] - a[i]) * (x - i as f64)
    }
}
fn peak_near(ac: &[f64], lag: f64) -> (f64, f64) {
    let (mut top, mut position) = (0., lag);
    for i in
        ((lag.floor() as isize - 2).max(1) as usize)..=((lag.ceil() as usize + 2).min(ac.len() - 2))
    {
        if ac[i] > top && ac[i] >= ac[i - 1] && ac[i] >= ac[i + 1] {
            top = ac[i];
            position = i as f64 + vertex(ac[i - 1], ac[i], ac[i + 1]);
        }
    }
    (top, position)
}
pub fn estimate(env: &[f32], cancel: &CancellationToken) -> Result<Option<(f64, f64)>, String> {
    if env.is_empty() {
        return Ok(None);
    }
    let n = env.len();
    let mean = env.iter().map(|v| *v as f64).sum::<f64>() / n as f64;
    let x: Vec<_> = env.iter().map(|v| *v as f64 - mean).collect();
    let min_lag = (60. / 200. * RATE).floor() as usize;
    let max_lag = RATE.ceil() as usize;
    let mut ac = vec![0.; max_lag * 4 + 2];
    for lag in 0..ac.len().min(n) {
        check(cancel)?;
        ac[lag] = x[lag..].iter().zip(x.iter()).map(|(a, b)| a * b).sum();
    }
    if ac[0] <= 0. {
        return Ok(None);
    }
    let mut score = vec![0.; max_lag + 2];
    for (lag, score) in score.iter_mut().enumerate().skip(min_lag) {
        let s = (1..=4)
            .map(|k| at(&ac, (lag * k) as f64).max(0.) / k as f64)
            .sum::<f64>();
        let octaves = (60. * RATE / lag as f64 / 120.).log2();
        *score = s * (-0.5 * octaves * octaves).exp();
    }
    let mut best = min_lag;
    for lag in min_lag..=max_lag {
        if score[lag] > score[best] {
            best = lag;
        }
    }
    if score[best] <= 0. {
        return Ok(None);
    }
    let mut period = best as f64
        + if best > min_lag && best < max_lag {
            vertex(score[best - 1], score[best], score[best + 1])
        } else {
            0.
        };
    let distance = |lag: f64| (60. * RATE / lag / 120.).log2().abs();
    let half = peak_near(&ac, period / 2.);
    if 60. * RATE / half.1 <= 200.
        && distance(half.1) < distance(period)
        && half.0 >= 0.6 * peak_near(&ac, period).0
    {
        period = half.1;
    }
    let peak = peak_near(&ac, period).0.max(at(&ac, period));
    Ok(Some((period, (peak / ac[0]).clamp(0., 1.))))
}
pub fn track(env: &[f32], period: f64, cancel: &CancellationToken) -> Result<Vec<f64>, String> {
    let n = env.len();
    if n == 0 {
        return Ok(vec![]);
    }
    let mean = env.iter().map(|v| *v as f64).sum::<f64>() / n as f64;
    let deviation = (env.iter().map(|v| (*v as f64 - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    let deviation = if deviation > 0. { deviation } else { 1. };
    let sigma = (period / 32.).max(0.5);
    let half = (sigma * 3.).ceil() as isize;
    let kernel: Vec<_> = (-half..=half)
        .map(|i| (-0.5 * (i as f64 / sigma).powi(2)).exp())
        .collect();
    let sum = kernel.iter().sum::<f64>();
    let mut local = vec![0.; n];
    for (i, local) in local.iter_mut().enumerate() {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let mut s = 0.;
        for j in -half..=half {
            let k = i as isize + j;
            if k >= 0 && (k as usize) < n {
                s += env[k as usize] as f64 * kernel[(j + half) as usize];
            }
        }
        *local = s / sum / deviation;
    }
    let lo = (period / 2.).round() as usize;
    let hi = (period * 2.).round() as usize;
    let penalty: Vec<_> = (0..=hi)
        .map(|gap| -100. * (gap as f64 / period).ln().powi(2))
        .collect();
    let mut cum = vec![0.; n];
    let mut back = vec![None; n];
    let mut first = true;
    let threshold = 0.01 * local[..n.min(hi * 4)].iter().copied().fold(0., f64::max);
    for i in 0..n {
        if i % 4096 == 0 {
            check(cancel)?;
        }
        let mut best = f64::NEG_INFINITY;
        let mut from = None;
        for gap in lo..=hi.min(i) {
            let value = cum[i - gap] + penalty[gap];
            if value > best {
                best = value;
                from = Some(i - gap);
            }
        }
        if from.is_none() || best <= 0. || (first && local[i] < threshold) {
            cum[i] = local[i];
        } else {
            cum[i] = local[i] + best;
            back[i] = from;
        }
        if local[i] >= threshold {
            first = false;
        }
    }
    let maxima: Vec<_> = (1..n.saturating_sub(1))
        .filter(|&i| cum[i] > cum[i - 1] && cum[i] >= cum[i + 1])
        .collect();
    let Some(&last) = maxima.last() else {
        return Ok(vec![]);
    };
    let med = quantile(&maxima.iter().map(|&i| cum[i]).collect::<Vec<_>>(), 0.5);
    let last = maxima
        .iter()
        .rev()
        .copied()
        .find(|&i| cum[i] >= 0.5 * med)
        .unwrap_or(last);
    let mut beats = vec![];
    let mut position = Some(last);
    while let Some(i) = position {
        beats.push(i);
        position = back[i];
    }
    beats.reverse();
    let rms = (beats.iter().map(|&b| local[b].powi(2)).sum::<f64>() / beats.len() as f64).sqrt();
    let (mut a, mut b) = (0, beats.len());
    while a < b && local[beats[a]] < 0.5 * rms {
        a += 1;
    }
    while b > a && local[beats[b - 1]] < 0.5 * rms {
        b -= 1;
    }
    Ok(beats[a..b]
        .iter()
        .map(|&f| {
            f as f64
                + if f > 0 && f + 1 < n {
                    vertex(local[f - 1], local[f], local[f + 1])
                } else {
                    0.
                }
        })
        .collect())
}
pub fn bpm(beats: &[f64], fallback: f64) -> f64 {
    if beats.len() < 4 {
        return fallback;
    }
    let gaps: Vec<_> = beats.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let med = quantile(&gaps, 0.5);
    if med <= 0. {
        return fallback;
    }
    let mut indices = vec![0.];
    for gap in gaps {
        indices.push(indices.last().unwrap() + (gap / med).round().max(1.));
    }
    let mx = indices.iter().sum::<f64>() / indices.len() as f64;
    let my = beats.iter().sum::<f64>() / beats.len() as f64;
    let sxy = indices
        .iter()
        .zip(beats)
        .map(|(x, y)| (x - mx) * (y - my))
        .sum::<f64>();
    let sxx = indices.iter().map(|x| (x - mx).powi(2)).sum::<f64>();
    let slope = if sxx > 0. { sxy / sxx } else { med };
    if slope > 0. { 60. / slope } else { fallback }
}
