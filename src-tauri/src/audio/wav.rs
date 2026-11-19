//! WAV de 16 kHz, mono y 16 bits: escritura incremental y reparación tras un cierre inesperado (R5).

use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, ErrorKind, Seek, SeekFrom, Write};
use std::path::Path;

use crate::audio::{MAX_DURATION_MS, SAMPLE_RATE};

const HEADER_LEN: u64 = 44;
const BYTES_PER_SAMPLE: u64 = 2;

/// Muestras que caben en la duración máxima de un dictado.
pub const MAX_SAMPLES: u64 = MAX_DURATION_MS as u64 * SAMPLE_RATE as u64 / 1_000;

fn header(data_len: u32) -> [u8; HEADER_LEN as usize] {
    let mut h = [0u8; HEADER_LEN as usize];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&data_len.saturating_add(36).to_le_bytes());
    h[8..16].copy_from_slice(b"WAVEfmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&1u16.to_le_bytes());
    h[24..28].copy_from_slice(&SAMPLE_RATE.to_le_bytes());
    h[28..32].copy_from_slice(&(SAMPLE_RATE * BYTES_PER_SAMPLE as u32).to_le_bytes());
    h[32..34].copy_from_slice(&(BYTES_PER_SAMPLE as u16).to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_len.to_le_bytes());
    h
}

fn samples_to_ms(samples: u64) -> i64 {
    (samples * 1_000 / u64::from(SAMPLE_RATE)) as i64
}

/// Escribe el audio poco a poco. Nunca supera la duración máxima de un dictado.
pub struct WavWriter {
    file: BufWriter<File>,
    samples: u64,
}

impl WavWriter {
    pub fn create(path: &Path) -> io::Result<Self> {
        let mut file = BufWriter::new(File::create(path)?);
        file.write_all(&header(0))?;
        Ok(Self { file, samples: 0 })
    }

    /// Añade muestras hasta el límite y devuelve cuántas se han escrito.
    pub fn write(&mut self, samples: &[i16]) -> io::Result<usize> {
        let room = MAX_SAMPLES.saturating_sub(self.samples);
        let count = samples
            .len()
            .min(usize::try_from(room).unwrap_or(usize::MAX));
        for sample in &samples[..count] {
            self.file.write_all(&sample.to_le_bytes())?;
        }
        self.samples += count as u64;
        Ok(count)
    }

    pub fn is_full(&self) -> bool {
        self.samples >= MAX_SAMPLES
    }

    pub fn duration_ms(&self) -> i64 {
        samples_to_ms(self.samples)
    }

    /// Vuelca a disco lo escrito hasta ahora.
    pub fn flush(&mut self) -> io::Result<()> {
        self.file.flush()?;
        self.file.get_ref().sync_data()
    }

    /// Completa la cabecera y devuelve la duración en milisegundos.
    pub fn finalize(mut self) -> io::Result<i64> {
        self.file.flush()?;
        let data_len = (self.samples * BYTES_PER_SAMPLE) as u32;
        let file = self.file.get_mut();
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&header(data_len))?;
        file.sync_all()?;
        Ok(samples_to_ms(self.samples))
    }
}

/// Repara la cabecera a partir del tamaño del archivo y devuelve la duración en milisegundos.
pub fn repair(path: &Path) -> io::Result<i64> {
    let size = std::fs::metadata(path)?.len();
    if size <= HEADER_LEN {
        return Ok(0);
    }
    let samples = ((size - HEADER_LEN) / BYTES_PER_SAMPLE).min(MAX_SAMPLES);
    let data_len = samples * BYTES_PER_SAMPLE;

    let mut file = OpenOptions::new().write(true).open(path)?;
    file.set_len(HEADER_LEN + data_len)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&header(data_len as u32))?;
    file.sync_all()?;
    Ok(samples_to_ms(samples))
}

/// Duración de una grabación sin reparar, según el tamaño del archivo.
pub fn duration_from_size(path: &Path) -> io::Result<i64> {
    let size = std::fs::metadata(path)?.len();
    let samples = (size.saturating_sub(HEADER_LEN) / BYTES_PER_SAMPLE).min(MAX_SAMPLES);
    Ok(samples_to_ms(samples))
}

/// Lee un audio propio como muestras en el rango [-1, 1].
pub fn read_samples(path: &Path) -> io::Result<Vec<f32>> {
    let invalid = || io::Error::new(ErrorKind::InvalidData, "el archivo no es un WAV válido");
    let bytes = std::fs::read(path)?;
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(invalid());
    }

    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let size_bytes: [u8; 4] = bytes[offset + 4..offset + 8]
            .try_into()
            .map_err(|_| invalid())?;
        let size = u32::from_le_bytes(size_bytes) as usize;
        let body = offset + 8;
        if &bytes[offset..offset + 4] == b"data" {
            let end = body.saturating_add(size).min(bytes.len());
            return Ok(bytes[body..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|sample| f32::from(i16::from_le_bytes(*sample)) / 32_768.0)
                .collect());
        }
        offset = body.saturating_add(size).saturating_add(size % 2);
    }
    Err(invalid())
}

/// Convierte una muestra en el rango [-1, 1] a 16 bits.
pub fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
}
