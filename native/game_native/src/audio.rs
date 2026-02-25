//! Path: native/game_native/src/audio.rs
//! Summary: BGM・SE 管理（rodio、ループ再生・fire-and-forget）
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

#[allow(dead_code)]
pub struct AudioManager {
    /// OutputStream を Drop させないためフィールドとして保持する
    _stream:  OutputStream,
    bgm_sink: Sink,
}

impl AudioManager {
    /// デフォルト出力デバイスで AudioManager を初期化する。
    /// デバイスが存在しない場合は `None` を返す。
    pub fn new() -> Option<Self> {
        let stream = OutputStreamBuilder::open_default_stream().ok()?;
        let bgm_sink = Sink::connect_new(&stream.mixer());
        Some(Self { _stream: stream, bgm_sink })
    }

    /// BGM をループ再生する。すでに再生中の場合は何もしない。
    pub fn play_bgm(&self, bytes: &'static [u8]) {
        if !self.bgm_sink.empty() {
            return;
        }
        let cursor = std::io::Cursor::new(bytes);
        if let Ok(source) = Decoder::new(cursor) {
            self.bgm_sink.append(source.repeat_infinite());
        }
    }

    /// BGM を一時停止する。
    pub fn pause_bgm(&self) {
        self.bgm_sink.pause();
    }

    /// BGM を再開する。
    pub fn resume_bgm(&self) {
        self.bgm_sink.play();
    }

    /// BGM の音量を設定する（0.0 = 無音、1.0 = 通常）。
    pub fn set_bgm_volume(&self, volume: f32) {
        self.bgm_sink.set_volume(volume.clamp(0.0, 1.0));
    }

    /// SE を非同期で再生する（再生後に自動解放）。
    pub fn play_se(&self, bytes: &'static [u8]) {
        self.play_se_with_volume(bytes, 1.0);
    }

    /// SE を指定音量で再生する（0.0 = 無音、1.0 = 通常）。
    /// デコードに失敗した場合は静かに無視する。
    pub fn play_se_with_volume(&self, bytes: &'static [u8], volume: f32) {
        let cursor = std::io::Cursor::new(bytes);
        if let Ok(source) = Decoder::new(cursor) {
            let sink = Sink::connect_new(&self._stream.mixer());
            sink.set_volume(volume.clamp(0.0, 1.0));
            sink.append(source);
            sink.detach();
        }
    }
}
