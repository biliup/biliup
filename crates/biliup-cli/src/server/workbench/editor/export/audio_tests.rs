use super::*;
use std::collections::BTreeMap;

#[tokio::test]
#[ignore = "requires Chromium and FFmpeg"]
async fn export_measures_speech_and_ducks_music_in_the_encoded_sound() {
    let temp = tempfile::tempdir().unwrap();
    let speech = temp.path().join("speech.wav");
    let music = temp.path().join("music.wav");
    for (path, source) in [
        (
            &speech,
            r"aevalsrc=if(between(t\,1\,3)\,0.2*sin(2*PI*440*t)\,0):s=16000:d=4",
        ),
        (&music, "sine=frequency=1000:sample_rate=16000:duration=4"),
    ] {
        assert!(
            crate::tools::ffmpeg_command()
                .args(["-v", "error", "-y", "-f", "lavfi", "-i", source])
                .arg(path)
                .status()
                .await
                .unwrap()
                .success()
        );
    }
    let project = json!({"format":"biliup.timeline/1","name":"Audio ducking","canvas":{"width":320,"height":180,"fps":30},
        "media":[{"id":"speech","name":"Speech","kind":"audio","path":"speech.wav","probe":{"frames":120,"hasAudio":true}},{"id":"music","name":"Music","kind":"audio","path":"music.wav","probe":{"frames":120,"hasAudio":true}}],
        "tracks":[{"id":"main","name":"Main","kind":"video","role":"main","items":[]},
        {"id":"voice","name":"Voice","kind":"audio","items":[{"id":"voice-clip","type":"audio","media":"speech","start":0,"duration":120}]},
        {"id":"music-track","name":"Music","kind":"audio","mixer":{"effects":[{"module":"audio.duck","params":{"under":["voice"],"amountDb":-18,"attackMs":100,"releaseMs":100}}]},"items":[{"id":"music-clip","type":"audio","media":"music","start":0,"duration":120}]}]});
    let paths = BTreeMap::from([
        ("speech".into(), speech.to_string_lossy().into_owned()),
        ("music".into(), music.to_string_lossy().into_owned()),
    ]);
    let output = temp.path().join("ducked.mp4");
    let cancel = CancellationToken::new();
    let mut measured = false;
    let options = serde_json::from_value(
        json!({"quality":"draft","video":{"codec":"h264","encoder":"software"}}),
    )
    .unwrap();
    let result = render(
        project,
        paths,
        options,
        temp.path(),
        &output,
        &cancel,
        |progress| {
            if matches!(progress, Progress::Sound { done: 1, total: 1 }) {
                measured = true;
            }
        },
    )
    .await
    .unwrap();
    assert!(measured);
    assert!(
        result
            .warnings
            .iter()
            .all(|warning| !warning.contains("not analysed")),
        "{:?}",
        result.warnings
    );
    let decoded = crate::tools::ffmpeg_command()
        .args(["-v", "error", "-i"])
        .arg(&output)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-f", "f32le", "pipe:1"])
        .output()
        .await
        .unwrap();
    assert!(decoded.status.success());
    let samples: Vec<f64> = decoded
        .stdout
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()) as f64)
        .collect();
    // Isolate the 1 kHz music from the 440 Hz speech, away from attack/release boundaries.
    let level = |start: f64| {
        let offset = (start * 16000.) as usize;
        let n = 4000;
        let (mut real, mut imag) = (0., 0.);
        for (i, &sample) in samples[offset..offset + n].iter().enumerate() {
            let phase = 2. * std::f64::consts::PI * 1000. * i as f64 / 16000.;
            real += sample * phase.cos();
            imag += sample * phase.sin();
        }
        (real * real + imag * imag).sqrt() / n as f64
    };
    let quiet = level(0.3);
    let ducked = level(1.5);
    let restored = level(3.5);
    let change = 20. * (ducked / quiet).log10();
    eprintln!("Encoded background music change during speech: {change:.2} dB");
    assert!((-21. ..-15.).contains(&change), "ducking = {change} dB");
    assert!(
        (20. * (restored / quiet).log10()).abs() < 1.,
        "music restores after speech"
    );
    let key = super::super::analysis::fingerprint(&speech, 4.)
        .await
        .unwrap();
    assert!(
        super::super::analysis::read(temp.path(), &key)
            .await
            .unwrap()["envelope"]["data"]
            .is_string()
    );
}
