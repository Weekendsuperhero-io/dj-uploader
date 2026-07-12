use anyhow::{Context, Result};
use std::fs::File;
use std::path::{Path, PathBuf};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

/// Creates preview snippets of an audio file at 30, 60, and 90 seconds
/// Each snippet takes 10-second chunks from intro, middle, and end with fade effects
pub fn create_preview_snippets(file_path: &Path) -> Result<Vec<PathBuf>> {
    let durations = vec![30, 60, 90]; // seconds
    let mut output_files = Vec::new();

    // Get the total duration first
    let total_duration = get_audio_duration(file_path)?;

    for duration in durations {
        let output_path = generate_snippet_path(file_path, duration)?;
        create_snippet(file_path, &output_path, duration, total_duration)?;
        output_files.push(output_path);
    }

    Ok(output_files)
}

/// Generate output path for snippet
fn generate_snippet_path(original: &Path, duration: u64) -> Result<PathBuf> {
    let parent = original.parent().unwrap_or(Path::new("."));
    let stem = original
        .file_stem()
        .and_then(|s| s.to_str())
        .context("Invalid file name")?;

    // Always output as WAV to avoid encoding complexity
    let output_name = format!("{}_preview_{}s.wav", stem, duration);
    Ok(parent.join(output_name))
}

/// Get the duration of an audio file in seconds
fn get_audio_duration(file_path: &Path) -> Result<f64> {
    let file = File::open(file_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("Failed to probe audio file")?;

    let track = format
        .default_track(TrackType::Audio)
        .context("No default audio track found")?;

    let sample_rate = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .and_then(|a| a.sample_rate);

    // Duration in seconds = number of audio frames / sample rate.
    if let (Some(frames), Some(sr)) = (track.num_frames, sample_rate) {
        Ok(frames as f64 / sr as f64)
    } else {
        // Fallback: assume 3 minutes if we can't determine.
        Ok(180.0)
    }
}

/// Create a snippet from the audio file
/// Takes 10-second chunks from intro, middle, and end with fade effects
fn create_snippet(
    input_path: &Path,
    output_path: &Path,
    duration_secs: u64,
    total_duration: f64,
) -> Result<()> {
    let chunk_duration = 10.0; // Always 10 seconds per chunk
    let num_chunks = (duration_secs as f64 / chunk_duration) as usize;

    // Calculate start positions for each chunk
    let mut positions = Vec::new();

    match num_chunks {
        3 => {
            // 30s: intro (0s), middle, end
            positions.push(0.0);
            positions.push((total_duration / 2.0) - (chunk_duration / 2.0));
            positions.push((total_duration - chunk_duration).max(20.0));
        }
        6 => {
            // 60s: 2 chunks from intro, 2 from middle, 2 from end
            positions.push(0.0);
            positions.push(10.0);
            positions.push((total_duration / 2.0) - chunk_duration);
            positions.push(total_duration / 2.0);
            positions.push((total_duration - (2.0 * chunk_duration)).max(40.0));
            positions.push((total_duration - chunk_duration).max(50.0));
        }
        9 => {
            // 90s: 3 chunks from intro, 3 from middle, 3 from end
            positions.push(0.0);
            positions.push(10.0);
            positions.push(20.0);
            positions.push((total_duration / 2.0) - (1.5 * chunk_duration));
            positions.push((total_duration / 2.0) - (0.5 * chunk_duration));
            positions.push((total_duration / 2.0) + (0.5 * chunk_duration));
            positions.push((total_duration - (3.0 * chunk_duration)).max(60.0));
            positions.push((total_duration - (2.0 * chunk_duration)).max(70.0));
            positions.push((total_duration - chunk_duration).max(80.0));
        }
        _ => {
            anyhow::bail!("Unsupported duration: {}s", duration_secs);
        }
    }

    // Extract all chunks
    let mut all_samples = Vec::new();
    let mut sample_rate = 44100;

    for &start_pos in &positions {
        let (samples, sr) = extract_chunk(input_path, start_pos, chunk_duration)?;
        sample_rate = sr;

        // Apply fade in/out
        let faded = apply_fades(samples, sample_rate);
        all_samples.extend(faded);
    }

    // Write to WAV file
    write_wav(output_path, &all_samples, sample_rate)?;

    Ok(())
}

/// Extract a chunk of audio starting at a specific position
fn extract_chunk(
    input_path: &Path,
    start_secs: f64,
    duration_secs: f64,
) -> Result<(Vec<f32>, u32)> {
    let file = File::open(input_path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = input_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("Failed to probe audio file")?;

    let track = format
        .default_track(TrackType::Audio)
        .context("No default audio track found")?;
    let track_id = track.id;
    let audio_params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .context("Track has no audio codec parameters")?;
    let sample_rate = audio_params.sample_rate.unwrap_or(44100);

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_params, &AudioDecoderOptions::default())
        .context("Failed to create decoder")?;

    // Seek to the start position (clamped to >= 0).
    let start = start_secs.max(0.0);
    let time = Time::try_new(
        start.trunc() as i64,
        (start.fract() * 1_000_000_000.0) as u32,
    )
    .unwrap_or(Time::ZERO);
    let _ = format.seek(
        SeekMode::Accurate,
        SeekTo::Time {
            time,
            track_id: Some(track_id),
        },
    );

    let mut samples: Vec<f32> = Vec::new();
    let mut interleaved: Vec<f32> = Vec::new();
    let target_samples = (duration_secs * sample_rate as f64) as usize;

    while samples.len() < target_samples {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(_) => break,
        };

        if packet.track_id != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                // Copy decoded samples as interleaved f32, then downmix to mono.
                let channels = audio_buf.spec().channels().count().max(1);
                interleaved.resize(audio_buf.samples_interleaved(), 0.0);
                audio_buf.copy_to_slice_interleaved(&mut interleaved);
                for frame in interleaved.chunks(channels) {
                    let sum: f32 = frame.iter().sum();
                    samples.push(sum / channels as f32);
                }
            }
            Err(_) => continue,
        }
    }

    // Trim to exact length.
    samples.truncate(target_samples);

    Ok((samples, sample_rate))
}

/// Apply 1-second fade in and fade out
fn apply_fades(mut samples: Vec<f32>, sample_rate: u32) -> Vec<f32> {
    let fade_samples = sample_rate as usize; // 1 second
    let len = samples.len();

    if len <= fade_samples * 2 {
        return samples;
    }

    // Fade in
    for (i, sample) in samples.iter_mut().enumerate().take(fade_samples) {
        let factor = i as f32 / fade_samples as f32;
        *sample *= factor;
    }

    // Fade out
    for (i, sample) in samples.iter_mut().enumerate().take(fade_samples) {
        let factor = 1.0 - (i as f32 / fade_samples as f32);
        *sample *= factor;
    }

    samples
}

/// Write samples to WAV file
fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec).context("Failed to create WAV writer")?;

    for &sample in samples {
        let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;

    Ok(())
}
