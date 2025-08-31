use thiserror::Error;
use xcap::{
    image::{ImageError, RgbaImage},
    XCapError, XCapResult,
};

use crate::process::Process;
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, SystemTime},
};

pub struct Recorder {
    config: RecordConfig,
    audio_path: PathBuf,
    screenshot_path: PathBuf,

    record_cmd: Child,
}

#[derive(Clone)]
pub struct RecordConfig {
    pub process: Process,
    pub output_dir: PathBuf,
}

pub struct RecordedData {
    pub audio_path: PathBuf,
    pub screenshot_path: PathBuf,
    pub duration: Duration,
}

#[derive(Error, Debug)]
pub enum RecordError {
    #[error("Failed to capture screenshot: {0}")]
    CaptureScreenshot(#[from] XCapError),
    #[error("Failed to save screenshot: {0}")]
    SaveScreenshot(#[from] ImageError),
    #[error("IO error on parecord: {0}")]
    IO(#[from] std::io::Error),
}

impl Recorder {
    // TODO: refactor
    pub fn start(config: RecordConfig) -> Recorder {
        // Generate the file paths
        let [audio_path, screenshot_path] = {
            let unix = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let name_prefix = format!("{}", unix);

            let audio_path = config.output_dir.join(format!("{}_audio.mp3", name_prefix));
            let screenshot_path = config
                .output_dir
                .join(format!("{}_screenshot.png", name_prefix));
            [audio_path, screenshot_path]
        };

        // Start the audio recording
        let record_cmd = start_audio_record(&audio_path, None).unwrap();

        Recorder {
            config,
            audio_path,
            screenshot_path,
            record_cmd,
        }
    }

    pub fn stop(mut self) -> Result<RecordedData, RecordError> {
        self.stop_audio()?;
        let audio_duration = audio_duration(&self.audio_path);

        // Capture and save the last image screenshot
        let screenshot = self.config.process.capture_image()?;
        screenshot
            .save(&self.screenshot_path)
            .map_err(RecordError::SaveScreenshot)?;

        Ok(RecordedData {
            audio_path: self.audio_path.clone(),
            screenshot_path: self.screenshot_path.clone(),
            duration: audio_duration,
        })
    }

    fn stop_audio(&mut self) -> Result<(), RecordError> {
        // Stop the parecord to end the audio recording
        if let Some(status) = self.record_cmd.try_wait()? {
            eprintln!(
                "parecord stopped before the recording should have been stopped: {:?}",
                status
            );
        }
        self.record_cmd.kill()?;

        // TODO: not hacky
        std::thread::sleep(Duration::from_secs(2));

        let audio_duration = audio_duration(&self.audio_path);
        println!("Captured {:?} of audio (before trim)", audio_duration);

        // Trim the silence from the beginning and end of the audio
        let tmp_trimmed_audio_path = self.audio_path.with_extension("tmp.mp3");
        let mut cmd = Command::new("sox");
        cmd.args([&self.audio_path, &tmp_trimmed_audio_path]).args([
            "silence", "1", "0.1", "1%", "reverse", "silence", "1", "0.1", "1%", "reverse",
        ]);
        let res = cmd.status().unwrap();
        if !res.success() {
            eprintln!("sox failed: {:?}", res);
        }
        std::fs::rename(&tmp_trimmed_audio_path, &self.audio_path)?;

        Ok(())
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.record_cmd.kill().unwrap();
    }
}

fn audio_duration(audio_path: &Path) -> Duration {
    // Hack to avoid processing empty audio files
    if std::fs::metadata(audio_path).unwrap().size() < 500 {
        return Duration::ZERO;
    }

    let output = Command::new("soxi")
        .arg("-D")
        .arg(audio_path)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("soxi failed: {:?}", output);
    }
    let duration_str = String::from_utf8(output.stdout).unwrap();
    let duration = duration_str.trim().parse::<f64>().unwrap();
    Duration::from_secs_f64(duration)
}

// fn default_audio_sink() -> Result<u32, RecordError> {
//     // Get audio pipewire info
//     let res = Command::new("wpctl").arg("status").output()?;
//     if !res.status.success() {
//         eprintln!("wpctl status failed: {:?}", res);
//     }

//     // Extract the default sink information line
//     let default_sink_info = std::str::from_utf8(&res.stdout)
//         .unwrap()
//         .lines()
//         .find_map(|line| line.trim().trim_start_matches('│').trim().strip_prefix('*'))
//         .expect("active sink should exists")
//         .trim();

//     // Extract the ID
//     let last_digit_idx = default_sink_info
//         .find(|c: char| !c.is_ascii_digit())
//         .unwrap_or(default_sink_info.len());
//     let default_sink_id = default_sink_info[..last_digit_idx]
//         .parse::<u32>()
//         .expect("sink ID should be a number");

//     Ok(default_sink_id)
// }

fn start_audio_record(audio_path: &Path, target_sink: Option<&str>) -> Result<Child, RecordError> {
    // based on https://github.com/JayXT/RecordAudioOutput/blob/main/record_audio_output

    let target_sink = target_sink.unwrap_or(r"@DEFAULT_MONITOR@");
    let mut record_cmd = Command::new("parec")
        .arg("-d")
        .arg(target_sink)
        .arg("--volume=65536")
        .stdout(Stdio::piped())
        .spawn()?;
    let convert_cmd = Command::new("lame")
        .arg("-r")
        .arg("-V5")
        .arg("-")
        .arg(audio_path)
        .stdin(record_cmd.stdout.take().unwrap())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(record_cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_record() {
        let mut cmd = start_audio_record("/tmp/test.wav".as_ref(), None).unwrap();
        std::thread::sleep(Duration::from_secs(3));
        cmd.kill().unwrap();
    }
}
