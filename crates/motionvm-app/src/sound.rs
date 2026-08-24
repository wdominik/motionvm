//! Sound output: the audio thread, and the engine's way onto it.
//!
//! ```text
//! game thread                            audio thread (cpal)
//!   Engine ─ MusicSink ──── mpsc ────▶  Player::fill
//!            Song::parse here             nothing but arithmetic
//! ```
//!
//! The split is where it is on purpose. Parsing a song means reading, checking
//! and allocating, and none of that belongs in an audio callback — so
//! [`Music::start`] does it on the game thread and sends the finished song
//! across. What is left in the callback is a `try_recv` and the synthesis.
//!
//! (The old song is dropped on the audio thread when a new one replaces it.
//! Handing it back over a second channel would avoid even that, but a location
//! change happens every few minutes, and a returning channel is more machinery
//! than the problem is worth.)
//!
//! **Nothing here is allowed to stop the game.** No device, no supported
//! format, no driver: a line on stderr and play on in silence.
//!
//! Both games come through here with the same shape and their own player:
//! Dunkle Schatten 2's HMI songs through the rebuilt MIDI driver
//! ([`open`]), Die Enviro-Kids greifen ein's PSM 2 tunes through the
//! rebuilt `MUSADL.DRV` sequencer ([`open_enviro`]).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use motionvm_audio::{Player, psm};
use motionvm_engine::MusicSink;
use motionvm_formats::m16::psm::Plx;
use motionvm_formats::m32::DriverArchive;
use motionvm_formats::m32::bnk::Bank as InstrumentBank;
use motionvm_formats::m32::hmi::Song;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

enum Command {
    Start(Box<Song>),
    Stop,
}

enum PsmCommand {
    Start(Box<Plx>, i16),
    Stop,
}

/// One side of the audio callback: drain the command channel, then render.
/// The two games differ only in what stands behind this.
trait Backend: Send + 'static {
    fn pump(&mut self, out: &mut [i16]);
}

struct HmiBackend {
    player: Player,
    rx: Receiver<Command>,
}

impl Backend for HmiBackend {
    fn pump(&mut self, out: &mut [i16]) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Start(song) => self.player.start(*song),
                Command::Stop => self.player.stop(),
            }
        }
        self.player.fill(out);
    }
}

struct PsmBackend {
    player: psm::Player,
    rx: Receiver<PsmCommand>,
}

impl Backend for PsmBackend {
    fn pump(&mut self, out: &mut [i16]) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                PsmCommand::Start(song, loops) => self.player.start(*song, loops),
                PsmCommand::Stop => self.player.stop(),
            }
        }
        self.player.fill(out);
    }
}

/// Holds the stream open. Dropping it stops the music, so it has to live as
/// long as the game does.
pub struct Sound {
    _stream: cpal::Stream,
}

/// What the engine talks to. Everything it is handed goes over the channel.
pub struct Music {
    tx: Sender<Command>,
}

impl MusicSink for Music {
    fn start(&mut self, _handle: i32, tune: i32, _looping: bool, song: &[u8]) {
        match Song::parse(song) {
            Ok(song) => {
                let _ = self.tx.send(Command::Start(Box::new(song)));
            }
            // A song that will not parse is worth saying out loud — it means a
            // block the decoder does not understand — but not worth stopping
            // for.
            Err(e) => eprintln!("tune {tune} did not parse: {e}"),
        }
    }

    fn stop(&mut self, _handle: i32) {
        let _ = self.tx.send(Command::Stop);
    }
}

/// The 16-bit game's sink. `STARTTUNE`'s loop count is `-1` at every call
/// site in the game — endless — and the driver reads it unsigned, so the
/// bool comes back out as the count it stands for.
pub struct PsmMusic {
    tx: Sender<PsmCommand>,
}

impl MusicSink for PsmMusic {
    fn start(&mut self, _handle: i32, tune: i32, looping: bool, song: &[u8]) {
        match Plx::parse(song) {
            Ok(song) => {
                let loops = if looping { -1 } else { 0 };
                let _ = self.tx.send(PsmCommand::Start(Box::new(song), loops));
            }
            Err(e) => eprintln!("tune {tune} did not parse: {e}"),
        }
    }

    fn stop(&mut self, _handle: i32) {
        let _ = self.tx.send(PsmCommand::Stop);
    }
}

/// Opens the default output and starts the audio thread.
///
/// `dir` is the game directory: the driver archive and the two instrument banks
/// come from there, the same three files `ENGINE.EXE` hands its MIDI layer.
pub fn open(dir: &Path) -> Result<(Sound, Music), String> {
    // `find_ci` throughout: these three are looked up by name, and a copied
    // install is as likely to spell them in lower case as on the disc.
    let read = |name: &str| -> Result<Vec<u8>, String> {
        let path =
            motionvm_formats::find_ci(dir, name).ok_or_else(|| format!("{name}: not found"))?;
        std::fs::read(path).map_err(|e| format!("{name}: {e}"))
    };
    let archive = read("HMIMDRV.386")?;
    let archive = DriverArchive::parse(&archive).map_err(|e| format!("HMIMDRV.386: {e}"))?;
    let driver = archive
        .device(motionvm_audio::opl::Fm::DEVICE)
        .ok_or("HMIMDRV.386 has no OPL3 driver")?;
    let melodic = read("MELODIC.BNK")?;
    let drums = read("DRUM.BNK")?;
    let melodic = InstrumentBank::parse(&melodic).map_err(|e| format!("MELODIC.BNK: {e}"))?;
    let drums = InstrumentBank::parse(&drums).map_err(|e| format!("DRUM.BNK: {e}"))?;

    let (device, config, format, channels, rate) = output()?;
    let player = Player::new(rate, driver, &melodic, &drums)
        .map_err(|e| format!("the FM driver did not come up: {e}"))?;
    let (tx, rx) = channel();
    let stream = spawn(&device, config, format, channels, HmiBackend { player, rx })?;
    tell_output(rate, channels, format);
    Ok((Sound { _stream: stream }, Music { tx }))
}

/// Opens the default output and starts the audio thread for the 16-bit
/// game.
///
/// `dir` is the game directory: `MUSADL.DRV` comes from there, the same
/// file `ENVIRO.EXE` loads whole and installs — motionvm reads its tables
/// and rebuilds the code around them.
pub fn open_enviro(dir: &Path) -> Result<(Sound, PsmMusic), String> {
    let path = motionvm_formats::find_ci(dir, "MUSADL.DRV").ok_or("MUSADL.DRV: not found")?;
    let driver = std::fs::read(path).map_err(|e| format!("MUSADL.DRV: {e}"))?;

    let (device, config, format, channels, rate) = output()?;
    let player = psm::Player::new(rate, &driver)
        .map_err(|e| format!("the Ad Lib driver did not come up: {e}"))?;
    let (tx, rx) = channel();
    let stream = spawn(&device, config, format, channels, PsmBackend { player, rx })?;
    tell_output(rate, channels, format);
    Ok((Sound { _stream: stream }, PsmMusic { tx }))
}

/// What the output device settled on — a diagnostic, so it speaks only
/// under `MOTIONVM_PERF`, the same switch the frame timing answers to.
fn tell_output(rate: u32, channels: usize, format: cpal::SampleFormat) {
    if std::env::var_os("MOTIONVM_PERF").is_some() {
        eprintln!("sound: {rate} Hz, {channels} channel(s), {format:?}");
    }
}

/// The default output device and what it wants to be fed.
fn output() -> Result<
    (
        cpal::Device,
        cpal::StreamConfig,
        cpal::SampleFormat,
        usize,
        u32,
    ),
    String,
> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output device")?;
    let supported = device
        .default_output_config()
        .map_err(|e| format!("no output format: {e}"))?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let channels = config.channels as usize;
    let rate = config.sample_rate;
    Ok((device, config, format, channels, rate))
}

/// Builds and starts the stream for whichever sample format the device
/// settled on.
fn spawn<B: Backend>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    format: cpal::SampleFormat,
    channels: usize,
    backend: B,
) -> Result<cpal::Stream, String> {
    let stream = match format {
        cpal::SampleFormat::F32 => build(device, config, backend, channels, |s| s as f32 / 32768.0),
        cpal::SampleFormat::I16 => build(device, config, backend, channels, |s| s),
        other => return Err(format!("unsupported sample format {other:?}")),
    }?;
    stream
        .play()
        .map_err(|e| format!("the stream would not start: {e}"))?;
    Ok(stream)
}

/// One builder for both sample formats: the synthesis is identical, only the
/// last step out of `i16` differs.
fn build<T, B>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    mut backend: B,
    channels: usize,
    convert: fn(i16) -> T,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + Send + 'static,
    B: Backend,
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
                backend.pump(&mut stereo[..frames * 2]);
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
