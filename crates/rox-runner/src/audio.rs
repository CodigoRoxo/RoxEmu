//! Saída de áudio: recebe o som do core (i16 estéreo, ex.: 44100 Hz), converte para a taxa da
//! placa de som (ex.: 48000 Hz) e mantém um buffer pequeno para baixa latência.
//!
//! O tempo do jogo e o tempo do áudio nunca batem exatamente; para não estalar nem atrasar,
//! a taxa de conversão é ajustada levemente conforme o buffer enche ou esvazia.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Context};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// Latência alvo do buffer, em segundos.
const TARGET_LATENCY: f64 = 0.064;
/// Quanto a taxa pode variar para compensar a diferença de relógio (0,5%).
const MAX_RATE_DELTA: f64 = 0.005;

pub struct AudioOut {
    _stream: cpal::Stream,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    device_rate: f64,
    channels: usize,
    /// Posição fracionária do resampler entre a última amostra e a próxima.
    phase: f64,
    last: [f32; 2],
}

impl AudioOut {
    pub fn open() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow!("nenhuma saída de áudio encontrada"))?;
        let supported = device
            .default_output_config()
            .context("falha ao ler a configuração de áudio")?;
        let format = supported.sample_format();
        let config = supported.config();
        let channels = config.channels as usize;
        let device_rate = config.sample_rate as f64;
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(16_384)));

        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, config, buffer.clone(), channels)?,
            SampleFormat::I16 => build::<i16>(&device, config, buffer.clone(), channels)?,
            SampleFormat::U16 => build::<u16>(&device, config, buffer.clone(), channels)?,
            SampleFormat::I32 => build::<i32>(&device, config, buffer.clone(), channels)?,
            other => return Err(anyhow!("formato de áudio não suportado: {other:?}")),
        };
        stream.play().context("falha ao iniciar o áudio")?;
        log::info!("áudio: {device_rate} Hz, {channels} canais, {format:?}");

        Ok(Self {
            _stream: stream,
            buffer,
            device_rate,
            channels,
            phase: 0.0,
            last: [0.0; 2],
        })
    }

    /// Quantos segundos de som estão esperando para tocar.
    pub fn buffered_secs(&self) -> f64 {
        let len = self.buffer.lock().map(|b| b.len()).unwrap_or(0);
        len as f64 / (self.device_rate * self.channels as f64)
    }

    /// Envia amostras estéreo intercaladas do core.
    pub fn push(&mut self, samples: &[i16], core_rate: f64) {
        if samples.is_empty() || core_rate <= 0.0 {
            return;
        }
        // Controle dinâmico de taxa: buffer cheio demais -> gera um pouco menos de amostras.
        let fill = self.buffered_secs() / TARGET_LATENCY;
        let adjust = 1.0 + ((1.0 - fill) * MAX_RATE_DELTA).clamp(-MAX_RATE_DELTA, MAX_RATE_DELTA);
        let step = core_rate / (self.device_rate * adjust);

        let mut out = Vec::with_capacity((samples.len() as f64 / step) as usize + 8);
        let frames = samples.len() / 2;
        let mut i = 0usize;
        while i < frames {
            let cur = [
                samples[i * 2] as f32 / 32768.0,
                samples[i * 2 + 1] as f32 / 32768.0,
            ];
            while self.phase < 1.0 {
                let t = self.phase as f32;
                let l = self.last[0] + (cur[0] - self.last[0]) * t;
                let r = self.last[1] + (cur[1] - self.last[1]) * t;
                push_frame(&mut out, self.channels, l, r);
                self.phase += step;
            }
            self.phase -= 1.0;
            self.last = cur;
            i += 1;
        }

        if let Ok(mut buf) = self.buffer.lock() {
            // Se acumulou demais (ex.: depois do fast-forward), descarta o excesso antigo.
            let max = (self.device_rate * self.channels as f64 * TARGET_LATENCY * 3.0) as usize;
            buf.extend(out);
            if buf.len() > max {
                let drop = buf.len() - max;
                buf.drain(..drop - drop % self.channels);
            }
        }
    }
}

fn push_frame(out: &mut Vec<f32>, channels: usize, l: f32, r: f32) {
    match channels {
        1 => out.push((l + r) * 0.5),
        _ => {
            out.push(l);
            out.push(r);
            out.extend(std::iter::repeat_n(0.0, channels - 2));
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    buffer: Arc<Mutex<VecDeque<f32>>>,
    channels: usize,
) -> anyhow::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let stream = device.build_output_stream::<T, _, _>(
        config,
        move |data: &mut [T], _| {
            let mut buf = match buffer.lock() {
                Ok(b) => b,
                Err(_) => return,
            };
            // Falta de som: toca silêncio em vez de repetir lixo.
            if buf.len() < data.len() {
                for s in data.iter_mut() {
                    *s = T::from_sample(0.0);
                }
                return;
            }
            for s in data.iter_mut() {
                *s = T::from_sample(buf.pop_front().unwrap_or(0.0));
            }
            let _ = channels;
        },
        |err| log::error!("erro de áudio: {err}"),
        None,
    )?;
    Ok(stream)
}
