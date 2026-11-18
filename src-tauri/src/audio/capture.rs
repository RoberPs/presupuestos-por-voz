//! Captura del micrófono con `cpal` en un hilo propio (research.md, R3).
//!
//! El flujo de audio no se puede mover entre hilos, así que vive entero en el hilo de grabación.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::audio::quality::rms;
use crate::audio::resample::StreamResampler;
use crate::audio::wav::{to_i16, WavWriter};
use crate::audio::{
    ActiveRecording, Recorder, RecordingEvent, RecordingSink, StopReason, MAX_DURATION_MS,
    WARNING_AT_MS,
};
use crate::error::{AppError, MicrophoneCode};

/// Unas 10 actualizaciones de nivel por segundo.
const LEVEL_INTERVAL_MS: i64 = 100;
/// Volcado a disco: se pierde como máximo un segundo ante un cierre inesperado (R5).
const FLUSH_INTERVAL_MS: i64 = 1_000;
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// Amplifica el nivel para que la voz normal se vea en el indicador.
const LEVEL_GAIN: f32 = 4.0;

enum Message {
    Samples(Vec<f32>),
    DeviceLost,
}

/// Micrófono predeterminado del sistema.
#[derive(Debug, Default, Clone, Copy)]
pub struct MicRecorder;

struct MicRecording {
    stop: Arc<AtomicBool>,
    duration_ms: Arc<AtomicI64>,
    thread: JoinHandle<()>,
}

impl ActiveRecording for MicRecording {
    fn stop(self: Box<Self>) -> Result<i64, AppError> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.join().map_err(|_| {
            log::error!("El hilo de grabación ha terminado de forma inesperada");
            AppError::microphone(MicrophoneCode::MicUnavailable)
        })?;
        Ok(self.duration_ms.load(Ordering::SeqCst))
    }
}

impl Recorder for MicRecorder {
    fn start(
        &self,
        part_path: &Path,
        events: RecordingSink,
    ) -> Result<Box<dyn ActiveRecording>, AppError> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let duration_ms = Arc::new(AtomicI64::new(0));
        let path = part_path.to_path_buf();

        let thread = {
            let stop = stop.clone();
            let duration_ms = duration_ms.clone();
            std::thread::Builder::new()
                .name("grabacion".into())
                .spawn(move || record(path, events, stop, duration_ms, ready_tx))
                .map_err(|e| {
                    log::error!("No se ha podido iniciar la grabación: {e}");
                    AppError::microphone(MicrophoneCode::MicUnavailable)
                })?
        };

        ready_rx
            .recv()
            .map_err(|_| AppError::microphone(MicrophoneCode::MicUnavailable))??;
        Ok(Box::new(MicRecording {
            stop,
            duration_ms,
            thread,
        }))
    }
}

fn microphone_error(kind: ErrorKind) -> AppError {
    AppError::microphone(match kind {
        ErrorKind::PermissionDenied => MicrophoneCode::MicAccessDenied,
        ErrorKind::DeviceNotAvailable => MicrophoneCode::MicNotFound,
        _ => MicrophoneCode::MicUnavailable,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    tx: Sender<Message>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let errors = tx.clone();
    device.build_input_stream(
        config,
        move |data: &[T], _: &_| {
            let samples = data.iter().map(|s| f32::from_sample_(*s)).collect();
            let _ = tx.send(Message::Samples(samples));
        },
        move |error: cpal::Error| {
            log::warn!("Error del micrófono: {error}");
            if matches!(
                error.kind(),
                ErrorKind::DeviceNotAvailable
                    | ErrorKind::DeviceChanged
                    | ErrorKind::StreamInvalidated
            ) {
                let _ = errors.send(Message::DeviceLost);
            }
        },
        None,
    )
}

struct OpenedMic {
    stream: Stream,
    rate: u32,
    channels: usize,
    rx: Receiver<Message>,
}

fn open_microphone() -> Result<OpenedMic, AppError> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AppError::microphone(MicrophoneCode::MicNotFound))?;
    let supported = device.default_input_config().map_err(|e| {
        log::warn!("No se puede leer la configuración del micrófono: {e}");
        microphone_error(e.kind())
    })?;

    let rate = supported.sample_rate();
    let channels = usize::from(supported.channels());
    let format = supported.sample_format();
    let config = supported.config();
    let (tx, rx) = mpsc::channel();

    let stream = match format {
        SampleFormat::I8 => build_stream::<i8>(&device, config, tx),
        SampleFormat::I16 => build_stream::<i16>(&device, config, tx),
        SampleFormat::I32 => build_stream::<i32>(&device, config, tx),
        SampleFormat::U8 => build_stream::<u8>(&device, config, tx),
        SampleFormat::U16 => build_stream::<u16>(&device, config, tx),
        SampleFormat::U32 => build_stream::<u32>(&device, config, tx),
        SampleFormat::F32 => build_stream::<f32>(&device, config, tx),
        SampleFormat::F64 => build_stream::<f64>(&device, config, tx),
        other => {
            log::warn!("Formato de micrófono no admitido: {other}");
            return Err(AppError::microphone(MicrophoneCode::MicUnavailable));
        }
    }
    .map_err(|e| {
        log::warn!("No se puede abrir el micrófono: {e}");
        microphone_error(e.kind())
    })?;

    stream.play().map_err(|e| {
        log::warn!("No se puede iniciar el micrófono: {e}");
        microphone_error(e.kind())
    })?;

    Ok(OpenedMic {
        stream,
        rate,
        channels,
        rx,
    })
}

/// Avisos y volcados periódicos de la grabación.
struct Progress {
    events: RecordingSink,
    next_level_ms: i64,
    next_flush_ms: i64,
    warned: bool,
}

impl Progress {
    fn update(&mut self, writer: &mut WavWriter, level: f32) {
        let elapsed_ms = writer.duration_ms();
        if elapsed_ms >= self.next_level_ms {
            self.next_level_ms = elapsed_ms + LEVEL_INTERVAL_MS;
            (self.events)(RecordingEvent::Level {
                elapsed_ms,
                level: (level * LEVEL_GAIN).clamp(0.0, 1.0),
            });
        }
        if !self.warned && elapsed_ms >= WARNING_AT_MS {
            self.warned = true;
            (self.events)(RecordingEvent::LimitWarning {
                remaining_ms: MAX_DURATION_MS - elapsed_ms,
            });
        }
        if elapsed_ms >= self.next_flush_ms {
            self.next_flush_ms = elapsed_ms + FLUSH_INTERVAL_MS;
            if let Err(e) = writer.flush() {
                log::warn!("No se ha podido volcar la grabación a disco: {e}");
            }
        }
    }
}

fn store(samples: &[f32], writer: &mut WavWriter) {
    let pcm: Vec<i16> = samples.iter().map(|s| to_i16(*s)).collect();
    if let Err(e) = writer.write(&pcm) {
        log::error!("No se ha podido guardar la grabación: {e}");
    }
}

fn record(
    path: PathBuf,
    events: RecordingSink,
    stop: Arc<AtomicBool>,
    duration_ms: Arc<AtomicI64>,
    ready: Sender<Result<(), AppError>>,
) {
    let opened = open_microphone().and_then(|mic| {
        let resampler = StreamResampler::new(mic.rate, mic.channels).map_err(|e| {
            log::warn!("Formato de micrófono no admitido: {e}");
            AppError::microphone(MicrophoneCode::MicUnavailable)
        })?;
        let writer = WavWriter::create(&path)?;
        Ok((mic, resampler, writer))
    });
    let (mic, mut resampler, mut writer) = match opened {
        Ok(opened) => opened,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let _ = ready.send(Ok(()));

    let mut progress = Progress {
        events: events.clone(),
        next_level_ms: 0,
        next_flush_ms: FLUSH_INTERVAL_MS,
        warned: false,
    };
    let mut converted = Vec::new();
    let mut stop_reason = None;

    while !stop.load(Ordering::SeqCst) {
        match mic.rx.recv_timeout(POLL_INTERVAL) {
            Ok(Message::Samples(samples)) => {
                converted.clear();
                if let Err(e) = resampler.push(&samples, &mut converted) {
                    log::error!("No se ha podido convertir la grabación: {e}");
                    continue;
                }
                store(&converted, &mut writer);
                progress.update(&mut writer, rms(&samples));
                if writer.is_full() {
                    stop_reason = Some(StopReason::LimitReached);
                    break;
                }
            }
            Ok(Message::DeviceLost) | Err(RecvTimeoutError::Disconnected) => {
                stop_reason = Some(StopReason::DeviceLost);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }

    drop(mic.stream);
    converted.clear();
    if resampler.finish(&mut converted).is_ok() {
        store(&converted, &mut writer);
    }
    let duration = writer.finalize().unwrap_or_else(|e| {
        log::error!("No se ha podido cerrar la grabación: {e}");
        0
    });
    duration_ms.store(duration, Ordering::SeqCst);

    if let Some(reason) = stop_reason {
        events(RecordingEvent::Stopped {
            reason,
            duration_ms: duration,
        });
    }
}
