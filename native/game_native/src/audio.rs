/// Step 22: BGM・SE 管理モジュール（rodio 0.21 クレート使用）
///
/// # rodio 0.21 API の変更点
/// - `OutputStream::try_default()` → `OutputStreamBuilder::open_default_stream()`
/// - `Sink::try_new(&handle)` → `Sink::connect_new(&stream.mixer())`
/// - `OutputStreamHandle` は廃止。`OutputStream` が直接 `mixer()` を持つ
/// - `playback` フィーチャーが必要（Cargo.toml に追加済み）
///
/// # 設計方針
/// - BGM はループ再生（`repeat_infinite`）
/// - SE は fire-and-forget（`Sink::detach()`）で自動解放
/// - `_stream` フィールドは Drop 防止のためオーナーシップを保持する
/// - 音声ファイルは `include_bytes!` でバイナリに埋め込む
/// - 音声デバイスが存在しない環境（CI 等）では `AudioManager::new()` が `None` を返す
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
    /// デコードに失敗した場合は静かに無視する。
    pub fn play_se(&self, bytes: &'static [u8]) {
        let cursor = std::io::Cursor::new(bytes);
        if let Ok(source) = Decoder::new(cursor) {
            let sink = Sink::connect_new(&self._stream.mixer());
            sink.append(source);
            sink.detach();
        }
    }

    /// SE を指定音量で再生する。
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
