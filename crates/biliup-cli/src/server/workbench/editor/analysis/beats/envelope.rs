//! Uncut's streamed 512-point FFT / mel-band spectral flux and low-band energy (MIT).
pub const SAMPLE_RATE: usize = 11025;
const WINDOW: usize = 512;
const HOP: usize = 128;
pub struct Envelope {
    ring: [f64; WINDOW],
    written: usize,
    since: isize,
    hann: [f64; WINDOW],
    re: [f64; WINDOW],
    im: [f64; WINDOW],
    cos: [f64; WINDOW / 2],
    sin: [f64; WINDOW / 2],
    band_of: [isize; WINDOW / 2 + 1],
    bands: usize,
    low_bands: usize,
    prev: Vec<f64>,
    cur: Vec<f64>,
    peak: f64,
    env: Vec<f32>,
    low: Vec<f32>,
    power: Vec<f32>,
    carry: Option<u8>,
    pub samples: usize,
}
fn deviation(values: &[f32]) -> f64 {
    let n = values.len().max(1) as f64;
    let mean = values.iter().map(|v| *v as f64).sum::<f64>() / n;
    (values
        .iter()
        .map(|v| (*v as f64 - mean).powi(2))
        .sum::<f64>()
        / n)
        .sqrt()
}
impl Envelope {
    pub fn new() -> Self {
        let mut s = Self {
            ring: [0.; WINDOW],
            written: WINDOW / 2,
            since: HOP as isize - WINDOW as isize / 2,
            hann: [0.; WINDOW],
            re: [0.; WINDOW],
            im: [0.; WINDOW],
            cos: [0.; WINDOW / 2],
            sin: [0.; WINDOW / 2],
            band_of: [-1; WINDOW / 2 + 1],
            bands: 0,
            low_bands: 0,
            prev: vec![],
            cur: vec![],
            peak: 0.,
            env: vec![],
            low: vec![],
            power: vec![],
            carry: None,
            samples: 0,
        };
        for i in 0..WINDOW {
            s.hann[i] = 0.5 - 0.5 * (2. * std::f64::consts::PI * i as f64 / WINDOW as f64).cos();
        }
        for k in 0..WINDOW / 2 {
            let phase = 2. * std::f64::consts::PI * k as f64 / WINDOW as f64;
            s.cos[k] = phase.cos();
            s.sin[k] = phase.sin();
        }
        let mel = |f: f64| 2595. * (1. + f / 700.).log10();
        let lo = mel(30.);
        let hi = mel(5000.);
        let mut last = -1;
        for k in 0..=WINDOW / 2 {
            let f = k as f64 * SAMPLE_RATE as f64 / WINDOW as f64;
            if !(30. ..=5000.).contains(&f) {
                continue;
            }
            let band = (((mel(f) - lo) / (hi - lo) * 40.).floor() as isize).min(39);
            if band != last {
                last = band;
                s.bands += 1;
            }
            s.band_of[k] = s.bands as isize - 1;
        }
        for k in 0..=WINDOW / 2 {
            if s.band_of[k] >= 0 && k as f64 * SAMPLE_RATE as f64 / (WINDOW as f64) < 200. {
                s.low_bands = s.band_of[k] as usize + 1;
            }
        }
        s.prev = vec![0.; s.bands];
        s.cur = vec![0.; s.bands];
        s
    }
    pub fn push(&mut self, bytes: &[u8]) {
        let mut at = 0;
        if let (Some(lo), Some(hi)) = (self.carry, bytes.first()) {
            self.carry = None;
            self.sample(i16::from_le_bytes([lo, *hi]) as f64 / 32768.);
            at = 1;
        }
        while at + 1 < bytes.len() {
            self.sample(i16::from_le_bytes([bytes[at], bytes[at + 1]]) as f64 / 32768.);
            at += 2;
        }
        if at < bytes.len() {
            self.carry = Some(bytes[at]);
        }
    }
    pub fn sample(&mut self, value: f64) {
        self.ring[self.written % WINDOW] = value;
        self.written += 1;
        self.samples += 1;
        self.since += 1;
        if self.since >= HOP as isize && self.written >= WINDOW {
            self.since = 0;
            self.frame();
        }
    }
    fn fft(&mut self) {
        let mut j = 0;
        for i in 1..WINDOW {
            let mut bit = WINDOW >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                self.re.swap(i, j);
                self.im.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= WINDOW {
            let half = size / 2;
            let step = WINDOW / size;
            for i in (0..WINDOW).step_by(size) {
                for k in 0..half {
                    let wr = self.cos[k * step];
                    let wi = -self.sin[k * step];
                    let a = i + k;
                    let b = a + half;
                    let xr = self.re[b] * wr - self.im[b] * wi;
                    let xi = self.re[b] * wi + self.im[b] * wr;
                    self.re[b] = self.re[a] - xr;
                    self.im[b] = self.im[a] - xi;
                    self.re[a] += xr;
                    self.im[a] += xi;
                }
            }
            size *= 2;
        }
    }
    fn frame(&mut self) {
        let start = self.written % WINDOW;
        for i in 0..WINDOW {
            self.re[i] = self.ring[(start + i) % WINDOW] * self.hann[i];
            self.im[i] = 0.;
        }
        self.fft();
        self.cur.fill(0.);
        let mut power = 0.;
        for k in 1..=WINDOW / 2 {
            let b = self.band_of[k];
            if b < 0 {
                continue;
            }
            let b = b as usize;
            let mag = self.re[k].hypot(self.im[k]);
            self.cur[b] += mag;
            if b < self.low_bands {
                power += mag * mag;
            }
        }
        let top = self.cur.iter().copied().fold(0., f64::max);
        self.peak = top.max(self.peak * 0.999);
        let floor = 0.03_f64.max(self.peak * 10_f64.powf(-35. / 20.));
        let (mut flux, mut low_flux) = (0., 0.);
        for b in 0..self.bands {
            let m = (self.cur[b].max(floor) / floor).ln();
            self.cur[b] = m;
            let d = m - self.prev[b];
            if d > 0. {
                flux += d;
                if b < self.low_bands {
                    low_flux += d;
                }
            }
        }
        if self.env.is_empty() {
            flux = 0.;
            low_flux = 0.;
        }
        std::mem::swap(&mut self.cur, &mut self.prev);
        self.power.push(power as f32);
        self.low.push(low_flux as f32);
        self.env.push(flux as f32);
    }
    pub fn finish(mut self) -> (Vec<f32>, Vec<f32>, f64) {
        for _ in 0..WINDOW / 2 {
            self.ring[self.written % WINDOW] = 0.;
            self.written += 1;
            self.since += 1;
            if self.since >= HOP as isize && self.written >= WINDOW {
                self.since = 0;
                self.frame();
            }
        }
        let se = deviation(&self.env);
        let sl = deviation(&self.low);
        for i in 0..self.env.len() {
            self.env[i] = ((if se > 0. { self.env[i] as f64 / se } else { 0. })
                + (if sl > 0. { self.low[i] as f64 / sl } else { 0. }))
                as f32;
        }
        (
            self.env,
            self.power,
            self.samples as f64 / SAMPLE_RATE as f64,
        )
    }
}
