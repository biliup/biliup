//! Uncut video/phaseCorrelator.ts and fft.ts (MIT): Hann-windowed 2D phase correlation.
const W: usize = 128;
const H: usize = 64;
const N: usize = W * H;
#[derive(Clone, Copy)]
pub struct Motion {
    pub dx: f64,
    pub dy: f64,
    pub peak: f64,
}
struct Fft {
    n: usize,
    cos: Vec<f64>,
    sin: Vec<f64>,
}
impl Fft {
    fn new(n: usize) -> Self {
        let phases: Vec<_> = (0..n / 2)
            .map(|k| 2. * std::f64::consts::PI * k as f64 / n as f64)
            .collect();
        Self {
            n,
            cos: phases.iter().map(|p| p.cos()).collect(),
            sin: phases.iter().map(|p| p.sin()).collect(),
        }
    }
    fn run(&self, re: &mut [f64], im: &mut [f64], offset: usize, stride: usize, inverse: bool) {
        let mut j = 0;
        for i in 1..self.n {
            let mut bit = self.n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                re.swap(offset + i * stride, offset + j * stride);
                im.swap(offset + i * stride, offset + j * stride);
            }
        }
        let mut size = 2;
        while size <= self.n {
            let half = size / 2;
            let step = self.n / size;
            for from in (0..self.n).step_by(size) {
                for k in 0..half {
                    let wr = self.cos[k * step];
                    let wi = if inverse {
                        self.sin[k * step]
                    } else {
                        -self.sin[k * step]
                    };
                    let a = offset + (from + k) * stride;
                    let b = offset + (from + k + half) * stride;
                    let r = wr * re[b] - wi * im[b];
                    let i = wr * im[b] + wi * re[b];
                    re[b] = re[a] - r;
                    im[b] = im[a] - i;
                    re[a] += r;
                    im[a] += i;
                }
            }
            size *= 2;
        }
    }
}
pub struct PhaseCorrelator {
    window: Vec<f64>,
    width: Fft,
    height: Fft,
    previous: Option<(Vec<f64>, Vec<f64>)>,
    re: Vec<f64>,
    im: Vec<f64>,
}
impl PhaseCorrelator {
    pub fn new() -> Self {
        Self {
            window: (0..N)
                .map(|i| {
                    let x = i % W;
                    let y = i / W;
                    (0.5 - 0.5 * (2. * std::f64::consts::PI * (x as f64 + 0.5) / W as f64).cos())
                        * (0.5
                            - 0.5 * (2. * std::f64::consts::PI * (y as f64 + 0.5) / H as f64).cos())
                })
                .collect(),
            width: Fft::new(W),
            height: Fft::new(H),
            previous: None,
            re: vec![0.; N],
            im: vec![0.; N],
        }
    }
    fn fft2(&self, re: &mut [f64], im: &mut [f64], inverse: bool) {
        for y in 0..H {
            self.width.run(re, im, y * W, 1, inverse)
        }
        for x in 0..W {
            self.height.run(re, im, x, W, inverse)
        }
    }
    pub fn push(
        &mut self,
        pixels: &[u8],
        frame_width: usize,
        frame_height: usize,
    ) -> Option<Motion> {
        let ox = frame_width.saturating_sub(W) / 2;
        let oy = frame_height.saturating_sub(H) / 2;
        let mean = (0..H)
            .flat_map(|y| (0..W).map(move |x| pixels[(y + oy) * frame_width + x + ox] as f64))
            .sum::<f64>()
            / N as f64;
        let mut re = std::mem::take(&mut self.re);
        let mut im = std::mem::take(&mut self.im);
        for y in 0..H {
            for x in 0..W {
                let i = y * W + x;
                re[i] = (pixels[(y + oy) * frame_width + x + ox] as f64 - mean) * self.window[i];
                im[i] = 0.;
            }
        }
        self.fft2(&mut re, &mut im, false);
        let old = self.previous.replace((re, im));
        let Some((mut pr, mut pi)) = old else {
            self.re = vec![0.; N];
            self.im = vec![0.; N];
            return None;
        };
        let (re, im) = self.previous.as_ref().unwrap();
        for i in 0..N {
            let r = re[i] * pr[i] + im[i] * pi[i];
            let v = im[i] * pr[i] - re[i] * pi[i];
            let mag = r.hypot(v);
            pr[i] = if mag > 1e-9 { r / mag } else { 0. };
            pi[i] = if mag > 1e-9 { v / mag } else { 0. };
        }
        self.fft2(&mut pr, &mut pi, true);
        let mut best = 0;
        for i in 1..N {
            if pr[i] > pr[best] {
                best = i;
            }
        }
        let px = (best % W) as isize;
        let py = (best / W) as isize;
        let at = |x: isize, y: isize| {
            pr[y.rem_euclid(H as isize) as usize * W + x.rem_euclid(W as isize) as usize] / N as f64
        };
        let peak = at(px, py);
        let sub = |l: f64, r: f64| {
            let (s, dir) = if r >= l { (r, 1.) } else { (l, -1.) };
            if s <= 0. || peak <= 0. {
                0.
            } else {
                dir * (s / (s + peak)).min(0.5)
            }
        };
        let mut dx = px as f64 + sub(at(px - 1, py), at(px + 1, py));
        let mut dy = py as f64 + sub(at(px, py - 1), at(px, py + 1));
        if dx > W as f64 / 2. {
            dx -= W as f64;
        }
        if dy > H as f64 / 2. {
            dy -= H as f64;
        }
        self.re = pr;
        self.im = pi;
        Some(Motion { dx, dy, peak })
    }
}
