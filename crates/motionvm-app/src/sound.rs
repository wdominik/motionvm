//! Sound output: the device, the audio thread, and the seam they meet at.
//!
//! The window owns exactly the platform half: pick the default output device,
//! build a stream for whatever sample format it settled on, and fan the
//! source's interleaved stereo out to however many channels the device has.
//! What plays into the stream is an [`AudioSource`] the game's family built —
//! file reading, codecs and synthesis all live behind its `fill`, on the far
//! side of the contract.
//!
//! **Nothing here is allowed to stop the game.** No device, no supported
//! format, no driver: a line on stderr and play on in silence.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use motionvm_playable::AudioSource;

/// Holds the stream open. Dropping it stops the music, so it has to live as
/// long as the game does.
pub struct Sound {
    _stream: cpal::Stream,
}

/// What the output device settled on — a diagnostic, so it speaks only
/// under `MOTIONVM_PERF`, the same switch the frame timing answers to.
fn tell_output(rate: u32, channels: usize, format: cpal::SampleFormat) {
    if std::env::var_os("MOTIONVM_PERF").is_some() {
        eprintln!("sound: {rate} Hz, {channels} channel(s), {format:?}");
    }
}

/// The opened output device, with what it wants to be fed.
///
/// One value instead of loose pieces, because the channel count and the rate
/// are the config's own facts: carried separately they could disagree with
/// it, and the accessors make that impossible.
pub struct Output {
    device: cpal::Device,
    config: cpal::StreamConfig,
    format: cpal::SampleFormat,
}

impl Output {
    /// The device's channel count.
    fn channels(&self) -> usize {
        self.config.channels as usize
    }

    /// The device's sample rate, which the game's music is built for.
    pub fn rate(&self) -> u32 {
        self.config.sample_rate
    }
}

/// The default output device and what it wants to be fed.
pub fn output() -> Result<Output, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("no output format: {e}"))?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    Ok(Output {
        device,
        config,
        format,
    })
}

/// Builds and starts the stream for whichever sample format the device
/// settled on, wraps it in the guard that keeps it alive, and reports what
/// the device settled on.
pub fn spawn(out: &Output, source: Box<dyn AudioSource>) -> Result<Sound, String> {
    let (channels, rate) = (out.channels(), out.rate());
    let stream = match out.format {
        cpal::SampleFormat::F32 => build(&out.device, out.config, source, channels, |s| {
            s as f32 / 32768.0
        }),
        cpal::SampleFormat::I16 => build(&out.device, out.config, source, channels, |s| s),
        other => return Err(format!("unsupported sample format {other:?}")),
    }?;
    stream
        .play()
        .map_err(|e| format!("the stream would not start: {e}"))?;
    tell_output(rate, channels, out.format);
    Ok(Sound { _stream: stream })
}

/// One builder for both sample formats: the synthesis is identical, only the
/// last step out of `i16` differs.
fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut source: Box<dyn AudioSource>,
    channels: usize,
    convert: fn(i16) -> T,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + Send + 'static,
{
    // Generous enough that the callback never reaches the allocator after the
    // first block, whatever buffer size the device settles on.
    let mut stereo = vec![0i16; 8192];
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels.max(1);
                if stereo.len() < frames * 2 {
                    stereo.resize(frames * 2, 0);
                }
                source.fill(&mut stereo[..frames * 2]);
                for (frame, out) in stereo
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .zip(data.chunks_mut(channels.max(1)))
                {
                    match out.len() {
                        // One speaker gets the two sides mixed rather than the
                        // left one only — a hard-panned voice would otherwise
                        // vanish.
                        1 => out[0] = convert(((frame[0] as i32 + frame[1] as i32) / 2) as i16),
                        _ => {
                            out[0] = convert(frame[0]);
                            out[1] = convert(frame[1]);
                            // Anything past the second is not ours to fill.
                            for extra in &mut out[2..] {
                                *extra = convert(0);
                            }
                        }
                    }
                }
            },
            |e| eprintln!("sound: {e}"),
            None,
        )
        .map_err(|e| format!("the stream would not open: {e}"))
}
