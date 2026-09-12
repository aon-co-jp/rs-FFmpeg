# rs-FFmpeg

A Rust-native tribute to [FFmpeg](https://ffmpeg.org/).
FFmpegへの敬意を込めたRust版。

This is an early-stage reimplementation, **not** a drop-in replacement.
Feature parity with FFmpeg (the hundreds of codecs and container formats
it supports) is a long-term aspiration, not a current fact.
これは開発初期段階のリライトであり、FFmpegの完全な代替では**ありません**。
FFmpegが対応する膨大な数のコーデック・コンテナ形式との機能一致は
長期的な目標であり、現時点での実態ではありません。

## What works today / 現在動く機能

- Probing uncompressed PCM WAV files (sample rate, channels, bit depth,
  duration). / 非圧縮PCM WAVファイルの解析(サンプルレート・チャンネル数・
  ビット深度・尺)
- Converting PCM WAV sample rate and channel count (mono <-> stereo).
  / PCM WAVのサンプルレート変換・チャンネル数変換(モノラル<->ステレオ)

```bash
rs-ffmpeg probe input.wav
rs-ffmpeg -i input.wav -ar 22050 -ac 1 output.wav
```

## What does not work yet / まだ動かない機能

- Compressed audio codecs (MP3, AAC, FLAC, OGG, ...)
- Any video codec or container
- Bitrate control, trimming, and every other flag `ffmpeg`/`ffprobe`
  support today

圧縮音声コーデック(MP3/AAC/FLAC/OGG等)・動画コーデック/コンテナ全般・
ビットレート制御やトリミング等、本家`ffmpeg`/`ffprobe`が持つその他の
機能は、いずれも未対応です。

## Development / 開発

```bash
cargo test
cargo build --release
```

## License

MIT
