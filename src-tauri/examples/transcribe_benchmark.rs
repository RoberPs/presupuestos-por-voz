//! Mide la transcripción local para la puerta de decisión SC-002 (tarea T006).
//!
//! Uso:
//!   cargo run --release --example transcribe_benchmark -- <modelo.bin> <audio.wav>
//!
//! El audio debe ser WAV PCM de 16 bits, mono y 16 kHz.

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

const SAMPLE_RATE: u32 = 16_000;
const MAX_THREADS: usize = 4;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (model, wav, requested_threads) = match args.as_slice() {
        [model, wav] => (model, wav, None),
        [model, wav, threads] => match threads.parse::<usize>() {
            Ok(n) if n > 0 => (model, wav, Some(n)),
            _ => {
                eprintln!("El número de hilos debe ser un entero positivo");
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("Uso: transcribe_benchmark <modelo.bin> <audio.wav> [hilos]");
            return ExitCode::from(2);
        }
    };

    match run(Path::new(model), Path::new(wav), requested_threads) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(model: &Path, wav: &Path, requested_threads: Option<usize>) -> Result<(), String> {
    let samples = read_wav_16k_mono(wav)?;
    let audio_secs = samples.len() as f64 / f64::from(SAMPLE_RATE);
    let threads = requested_threads.unwrap_or_else(|| {
        std::thread::available_parallelism().map_or(MAX_THREADS, |n| n.get().min(MAX_THREADS))
    });

    let started = Instant::now();
    let model_path = model.to_str().ok_or("la ruta del modelo no es válida")?;
    let context = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| format!("no se pudo cargar el modelo: {e}"))?;
    let mut state = context
        .create_state()
        .map_err(|e| format!("no se pudo preparar el motor: {e}"))?;
    let load_secs = started.elapsed().as_secs_f64();

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("es"));
    params.set_n_threads(threads as i32);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    let transcribe_started = Instant::now();
    state
        .full(params, &samples)
        .map_err(|e| format!("la transcripción falló: {e}"))?;
    let transcribe_secs = transcribe_started.elapsed().as_secs_f64();
    let total_secs = started.elapsed().as_secs_f64();

    let mut text = String::new();
    for segment in state.as_iter() {
        let piece = segment
            .to_str_lossy()
            .map_err(|e| format!("no se pudo leer el texto: {e}"))?;
        text.push_str(&piece);
    }

    println!("modelo:            {}", model.display());
    println!("hilos:             {threads}");
    println!(
        "motor:             {}",
        whisper_rs::print_system_info().trim()
    );
    println!("audio:             {audio_secs:.1} s");
    println!("carga del modelo:  {load_secs:.1} s");
    println!("transcripción:     {transcribe_secs:.1} s");
    println!("tiempo total:      {total_secs:.1} s");
    println!("factor (total/audio): {:.2}", total_secs / audio_secs);
    match peak_memory_mib() {
        Some(mib) => println!("pico de memoria:   {mib:.0} MiB"),
        None => println!("pico de memoria:   no disponible"),
    }
    println!("texto:\n{}", text.trim());
    Ok(())
}

/// Lee un WAV PCM de 16 bits, mono y 16 kHz y devuelve muestras en el rango [-1, 1].
fn read_wav_16k_mono(path: &Path) -> Result<Vec<f32>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("no se pudo abrir el audio: {e}"))?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("el archivo no es un WAV".into());
    }

    let mut format_checked = false;
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]) as usize;
        let body = offset + 8;

        if id == b"fmt " {
            if body + 16 > bytes.len() {
                return Err("la cabecera del WAV está incompleta".into());
            }
            let format = u16::from_le_bytes([bytes[body], bytes[body + 1]]);
            let channels = u16::from_le_bytes([bytes[body + 2], bytes[body + 3]]);
            let rate = u32::from_le_bytes([
                bytes[body + 4],
                bytes[body + 5],
                bytes[body + 6],
                bytes[body + 7],
            ]);
            let bits = u16::from_le_bytes([bytes[body + 14], bytes[body + 15]]);
            if format != 1 || channels != 1 || rate != SAMPLE_RATE || bits != 16 {
                return Err(format!(
                    "se necesita PCM de 16 bits, mono y 16 kHz (formato {format}, \
                     {channels} canales, {rate} Hz, {bits} bits)"
                ));
            }
            format_checked = true;
        } else if id == b"data" {
            if !format_checked {
                return Err("el WAV no declara su formato antes de los datos".into());
            }
            let end = body.saturating_add(size).min(bytes.len());
            return Ok(bytes[body..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|sample| f32::from(i16::from_le_bytes(*sample)) / 32768.0)
                .collect());
        }
        // Los bloques RIFF se alinean a un número par de bytes.
        offset = body.saturating_add(size).saturating_add(size % 2);
    }
    Err("el WAV no contiene datos de audio".into())
}

#[cfg(windows)]
fn peak_memory_mib() -> Option<f64> {
    use std::ffi::c_void;

    #[repr(C)]
    #[derive(Default)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let size = std::mem::size_of::<ProcessMemoryCounters>() as u32;
    let mut counters = ProcessMemoryCounters {
        cb: size,
        ..Default::default()
    };
    // SAFETY: `counters` es una estructura válida del tamaño indicado y el
    // pseudo-identificador del proceso actual no necesita cerrarse.
    let ok = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
    (ok != 0).then(|| counters.peak_working_set_size as f64 / (1024.0 * 1024.0))
}

#[cfg(not(windows))]
fn peak_memory_mib() -> Option<f64> {
    None
}
