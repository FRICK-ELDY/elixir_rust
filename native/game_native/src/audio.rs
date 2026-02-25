//! Path: native/game_native/src/audio.rs
//! Summary: BGM・SE 管理（rodio、ループ再生・fire-and-forget）
//! 1.7.3: game_window から game_native に移動。
//!
//! 音声データは [AssetLoader](crate::asset::AssetLoader) 経由で取得すること（Single Source of Truth）。

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

#[allow(dead_code)]
pub struct AudioManager {
    _stream:  OutputStream,
    bgm_sink: Sink,
}

impl AudioManager {
    pub fn new() -> Option<Self> {
        let stream = OutputStreamBuilder::open_default_stream().ok()?;
        let bgm_sink = Sink::connect_new(&stream.mixer());
        Some(Self { _stream: stream, bgm_sink })
    }

    pub fn play_bgm(&self, bytes: &[u8]) {
        if !self.bgm_sink.empty() {
            return;
        }
        let cursor = std::io::Cursor::new(bytes);
        if let Ok(source) = Decoder::new(cursor) {
            self.bgm_sink.append(source.buffered().repeat_infinite());
        }
    }

    pub fn pause_bgm(&self) {
        self.bgm_sink.pause();
    }

    pub fn resume_bgm(&self) {
        self.bgm_sink.play();
    }

    pub fn set_bgm_volume(&self, volume: f32) {
        self.bgm_sink.set_volume(volume.clamp(0.0, 1.0));
    }

    pub fn play_se(&self, bytes: Vec<u8>) {
        self.play_se_with_volume(bytes, 1.0);
    }

    pub fn play_se_with_volume(&self, bytes: Vec<u8>, volume: f32) {
        let cursor = std::io::Cursor::new(bytes);
        if let Ok(source) = Decoder::new(cursor) {
            let sink = Sink::connect_new(&self._stream.mixer());
            sink.set_volume(volume.clamp(0.0, 1.0));
            sink.append(source);
            sink.detach();
        }
    }
}
