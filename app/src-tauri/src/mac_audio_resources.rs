//! macOS CoreAudio resource ownership.
//!
//! cpal's macOS streams are intentionally not `Send`/`Sync` because they own
//! CoreAudio property-listener callbacks. Tauri application state is shared
//! across command workers and therefore must satisfy both bounds. These small
//! owners keep the native streams on dedicated resource threads; the rest of
//! the application shares only thread-safe handles and immutable metadata.

use std::sync::mpsc::{self, Sender};
use std::thread;

use rodio::cpal::traits::{DeviceTrait, HostTrait};

pub(crate) struct MacAudioOutput {
    mixer: rodio::mixer::Mixer,
    device_name: Option<String>,
    sample_rate: u32,
    channels: u16,
    stop: Sender<()>,
}

impl MacAudioOutput {
    pub(crate) fn open(
        requested_device_name: Option<&str>,
        allow_fallback: bool,
    ) -> Result<Self, String> {
        let requested_device_name = requested_device_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel();
        let worker_requested_device_name = requested_device_name.clone();
        thread::Builder::new()
            .name("syndocal-macos-audio-output".to_string())
            .spawn(move || {
                let host = rodio::cpal::default_host();
                let device = match worker_requested_device_name.as_deref() {
                    Some(name) => match host.output_devices() {
                        Ok(devices) => devices
                            .filter_map(|device| {
                                (device.name().ok().as_deref() == Some(name)).then_some(device)
                            })
                            .next()
                            .ok_or_else(|| format!("Audio output device '{name}' was not found")),
                        Err(error) => Err(format!("Failed to list audio output devices: {error}")),
                    },
                    None => host
                        .default_output_device()
                        .ok_or_else(|| "No default audio output device is available".to_string()),
                };
                let result = device.and_then(|device| {
                    let resolved_name = device.name().ok();
                    let builder = rodio::OutputStreamBuilder::from_device(device)
                        .map_err(|error| format!("Failed to configure audio output: {error}"))?;
                    let stream = if allow_fallback {
                        builder
                            .open_stream_or_fallback()
                            .map_err(|error| format!("Failed to open audio output: {error}"))?
                    } else {
                        builder
                            .open_stream()
                            .map_err(|error| format!("Failed to open audio output: {error}"))?
                    };
                    let config = *stream.config();
                    let mixer = stream.mixer().clone();
                    Ok((
                        stream,
                        mixer,
                        resolved_name,
                        config.sample_rate(),
                        config.channel_count(),
                    ))
                });

                match result {
                    Ok((stream, mixer, device_name, sample_rate, channels)) => {
                        if ready_tx
                            .send(Ok((mixer, device_name, sample_rate, channels)))
                            .is_ok()
                        {
                            // CoreAudio stream ownership remains on this thread
                            // until the explicit output retirement boundary.
                            let _ = stop_rx.recv();
                        }
                        drop(stream);
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                }
            })
            .map_err(|error| format!("Failed to start macOS audio owner: {error}"))?;

        let (mixer, device_name, sample_rate, channels) = ready_rx
            .recv()
            .map_err(|_| "macOS audio owner stopped before opening the output".to_string())??;
        Ok(Self {
            mixer,
            device_name,
            sample_rate,
            channels,
            stop: stop_tx,
        })
    }

    pub(crate) fn mixer(&self) -> &rodio::mixer::Mixer {
        &self.mixer
    }

    pub(crate) fn device_name(&self) -> Option<&str> {
        self.device_name.as_deref()
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub(crate) fn channels(&self) -> u16 {
        self.channels
    }
}

impl Drop for MacAudioOutput {
    fn drop(&mut self) {
        // The owner thread drops the CoreAudio stream after receiving this
        // signal. Do not join here: callers may retire an output while the
        // media mutex is held, and retirement must remain non-blocking.
        let _ = self.stop.send(());
    }
}

pub(crate) struct MacAudioInputStream {
    stop: Sender<()>,
}

pub(crate) fn own_input_stream(
    build: impl FnOnce() -> Result<rodio::cpal::Stream, String> + Send + 'static,
) -> Result<MacAudioInputStream, String> {
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (stop_tx, stop_rx) = mpsc::channel();
    thread::Builder::new()
        .name("syndocal-macos-audio-input".to_string())
        .spawn(move || match build() {
            Ok(stream) => {
                if ready_tx.send(Ok(())).is_ok() {
                    let _ = stop_rx.recv();
                }
                drop(stream);
            }
            Err(error) => {
                let _ = ready_tx.send(Err(error));
            }
        })
        .map_err(|error| format!("Failed to start macOS audio input owner: {error}"))?;
    ready_rx
        .recv()
        .map_err(|_| "macOS audio input owner stopped before opening the stream".to_string())??;
    Ok(MacAudioInputStream { stop: stop_tx })
}

impl Drop for MacAudioInputStream {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}
