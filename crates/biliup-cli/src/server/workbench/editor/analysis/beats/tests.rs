use super::*;

fn clicks(bpm: f64, seconds: f64) -> (Vec<f64>, Vec<f64>) {
    let mut samples = vec![0.; (seconds * SAMPLE_RATE as f64).round() as usize];
    let mut truth = vec![];
    let mut time = 0.37;
    while time < seconds - 0.05 {
        truth.push(time);
        let start = (time * SAMPLE_RATE as f64).round() as usize;
        let n = 0.01 * SAMPLE_RATE as f64;
        for i in 0..n.ceil() as usize {
            if start + i < samples.len() {
                samples[start + i] += 0.8
                    * (2. * std::f64::consts::PI * 1500. * i as f64 / SAMPLE_RATE as f64).sin()
                    * (-(i as f64) / (n / 4.)).exp();
            }
        }
        time += 60. / bpm;
    }
    (samples, truth)
}
fn samples_result(samples: &[f64]) -> Value {
    let mut env = Envelope::new();
    for &sample in samples {
        env.sample(sample);
    }
    let (env, power, seconds) = env.finish();
    result(&env, &power, seconds, &CancellationToken::new()).unwrap()
}
#[test]
fn beats_clicks_find_each_beat_without_octave_errors() {
    for bpm in [70., 90., 120., 128., 150., 190.] {
        let (samples, truth) = clicks(bpm, 20.);
        let found = samples_result(&samples);
        assert!(valid(&found));
        assert!((found["bpm"].as_f64().unwrap() - bpm).abs() < 1., "{found}");
        let beats: Vec<_> = found["beats"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(beats.len(), truth.len(), "BPM {bpm}");
        for (a, b) in beats.iter().zip(truth) {
            assert!((a - b).abs() < 0.03, "{bpm}: {a} vs {b}");
        }
        assert_eq!(found["beatLevels"].as_array().unwrap().len(), beats.len());
    }
}
#[test]
fn beats_silence_noise_chunking_and_cancellation() {
    let silence = samples_result(&vec![0.; SAMPLE_RATE * 5]);
    assert!(silence["bpm"].is_null());
    assert_eq!(silence["beats"], json!([]));
    let mut state = 3u32;
    let noise: Vec<_> = (0..SAMPLE_RATE * 30)
        .map(|_| {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            (state as f64 / u32::MAX as f64 * 2. - 1.) * 0.3
        })
        .collect();
    let hiss = samples_result(&noise);
    assert!(hiss["bpm"].is_null(), "{hiss}");
    let (samples, _) = clicks(120., 8.);
    let pcm: Vec<_> = samples
        .iter()
        .map(|v| (v * 32767.).round() as i16)
        .flat_map(i16::to_le_bytes)
        .collect();
    let mut whole = Envelope::new();
    whole.push(&pcm);
    let mut chunked = Envelope::new();
    for chunk in pcm.chunks(103) {
        chunked.push(chunk);
    }
    let (a, ap, sa) = whole.finish();
    let (b, bp, sb) = chunked.finish();
    assert_eq!(a, b);
    assert_eq!(ap, bp);
    assert_eq!(sa, sb);
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(result(&a, &ap, sa, &cancel).is_err());
}
#[test]
fn beat_levels_capture_bass_energy_changes() {
    let beats = vec![0.003, 0.503, 1.003, 1.503];
    let power: Vec<_> = (0..180)
        .map(|frame| if frame < 86 { 1. } else { 16. })
        .collect();
    let values = levels(&beats, &power);
    assert_eq!(values.len(), 4);
    assert_eq!(values[0], 0.25);
    assert_eq!(values[2], 1.);
}
#[tokio::test]
#[ignore = "requires FFmpeg and BILIUP_BEAT_FIXTURES generated from the upstream algorithm"]
async fn beats_match_upstream_fixtures_through_ffmpeg() {
    let root = std::env::var_os("BILIUP_BEAT_FIXTURES").expect("fixture folder");
    let root = Path::new(&root);
    let cases: Vec<Value> =
        serde_json::from_slice(&std::fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let expected: Value =
            serde_json::from_slice(&std::fs::read(root.join(format!("{name}.json"))).unwrap())
                .unwrap();
        let found = analyze(
            &root.join(format!("{name}.wav")),
            case["seconds"].as_f64().unwrap(),
            &CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        assert!(valid(&found));
        assert_eq!(
            found["bpm"].as_f64(),
            expected["bpm"].as_f64(),
            "{name} BPM"
        );
        assert_eq!(
            found["confidence"].as_f64(),
            expected["confidence"].as_f64(),
            "{name} confidence"
        );
        for key in ["beats", "beatLevels"] {
            assert_eq!(
                found[key].as_array().map(Vec::len),
                expected[key].as_array().map(Vec::len),
                "{name} {key} count"
            );
            if let Some(items) = found[key].as_array() {
                for (actual, expected) in items.iter().zip(expected[key].as_array().unwrap()) {
                    assert!(
                        (actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() <= 0.0011,
                        "{name} {key}: {actual} {expected}"
                    );
                }
            }
        }
        let actual = found["onsets"].as_array().unwrap();
        let expected = expected["onsets"].as_array().unwrap();
        assert_eq!(actual.len(), expected.len(), "{name} onset count");
        for (actual, expected) in actual.iter().zip(expected) {
            for key in ["t", "strength"] {
                assert!(
                    (actual[key].as_f64().unwrap() - expected[key].as_f64().unwrap()).abs() < 1e-8,
                    "{name} onset {key}: {actual} {expected}"
                );
            }
        }
        if let Some(bpm) = case["bpm"].as_f64() {
            assert!((found["bpm"].as_f64().unwrap() - bpm).abs() < 1., "{name}");
        }
        eprintln!(
            "{name}: {} BPM, {} beats, matches upstream",
            found["bpm"],
            found["beats"].as_array().unwrap().len()
        );
    }
}
