mod wav;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn print_usage() {
    eprintln!("使い方 / usage:");
    eprintln!("  rs-ffmpeg probe <input.wav>");
    eprintln!("  rs-ffmpeg -i <input.wav> [-ar <rate>] [-ac <channels>] <output.wav>");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.first().map(String::as_str) == Some("probe") {
        let Some(path) = args.get(1) else {
            print_usage();
            return ExitCode::FAILURE;
        };
        return match wav::probe(&PathBuf::from(path)) {
            Ok(info) => {
                println!("sample_rate={} channels={} bits_per_sample={} duration_secs={:.3}",
                    info.sample_rate, info.channels, info.bits_per_sample, info.duration_secs());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("エラー: {e}");
                ExitCode::FAILURE
            }
        };
    }

    // ffmpeg風の簡易互換: -i in.wav [-ar rate] [-ac channels] out.wav
    let mut input = None;
    let mut output = None;
    let mut target_rate = None;
    let mut target_channels = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" => { i += 1; input = args.get(i).map(PathBuf::from); }
            "-ar" => { i += 1; target_rate = args.get(i).and_then(|s| s.parse().ok()); }
            "-ac" => { i += 1; target_channels = args.get(i).and_then(|s| s.parse().ok()); }
            other => { output = Some(PathBuf::from(other)); }
        }
        i += 1;
    }

    let (Some(input), Some(output)) = (input, output) else {
        print_usage();
        return ExitCode::FAILURE;
    };

    let data = match wav::read(&input) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("エラー: {e}");
            return ExitCode::FAILURE;
        }
    };

    let channels = target_channels.unwrap_or(data.info.channels);
    let rate = target_rate.unwrap_or(data.info.sample_rate);

    let samples = if channels != data.info.channels {
        match wav::convert_channels(&data, channels) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("エラー: {e}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        data.samples.clone()
    };

    let samples = if rate != data.info.sample_rate {
        // resampleはconvert_channels後のデータ構造を前提としないため、
        // レート変換のみ元のWavData構造で行い、チャンネル変換は別途適用する
        // v0.1の割り切りとして、rate変換とchannel変換は同時指定時は
        // rate変換を先に元データへ適用してからchannel変換する。
        let resampled_data = wav::WavData {
            info: wav::WavInfo { sample_rate: data.info.sample_rate, channels: data.info.channels, bits_per_sample: data.info.bits_per_sample, num_samples_per_channel: data.info.num_samples_per_channel },
            samples: data.samples.clone(),
        };
        let resampled = wav::resample(&resampled_data, rate);
        if channels != data.info.channels {
            let intermediate = wav::WavData {
                info: wav::WavInfo { sample_rate: rate, channels: data.info.channels, bits_per_sample: data.info.bits_per_sample, num_samples_per_channel: (resampled.len() / data.info.channels as usize) as u32 },
                samples: resampled,
            };
            match wav::convert_channels(&intermediate, channels) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("エラー: {e}");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            resampled
        }
    } else {
        samples
    };

    match wav::write(&output, rate, channels, &samples) {
        Ok(()) => {
            println!("変換完了 / done: {}", output.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("エラー: {e}");
            ExitCode::FAILURE
        }
    }
}
