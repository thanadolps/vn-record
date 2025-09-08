use duct::{Handle, cmd, unix::HandleExt};
use thiserror::Error;
use xcap::{XCapError, image::ImageError};

use crate::process::Process;
use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub struct Recorder {
    config: RecordConfig,
    audio_path: PathBuf,
    screenshot_path: PathBuf,

    record_cmd: Handle,
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
    #[error("IO error on recording: {0}")]
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
        // Stop the recording command to end the audio recording
        if let Some(status) = self.record_cmd.try_wait()? {
            eprintln!(
                "Recording stopped before the recording should have been stopped: {:?}",
                status
            );
        }

        const SIGTERM: i32 = 15;
        self.record_cmd.send_signal(SIGTERM)?;
        self.record_cmd.wait()?;

        let audio_duration = audio_duration(&self.audio_path);
        println!("Captured {:?} of audio (before trim)", audio_duration);

        // Trim the silence from the beginning and end of the audio
        let tmp_trimmed_audio_path = self.audio_path.with_extension("tmp.mp3");
        let res = cmd!(
            "sox",
            &self.audio_path,
            &tmp_trimmed_audio_path,
            "silence",
            "1",
            "0.1",
            "1%",
            "reverse",
            "silence",
            "1",
            "0.1",
            "1%",
            "reverse"
        )
        .run();
        if let Err(e) = res {
            eprintln!("sox failed: {:?}", e);
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

    match cmd!("soxi", "-D", audio_path).read() {
        Ok(duration_str) => {
            let duration = duration_str.trim().parse::<f64>().unwrap();
            Duration::from_secs_f64(duration)
        }
        Err(e) => {
            panic!("soxi failed: {:?}", e);
        }
    }
}

fn start_audio_record(audio_path: &Path, target_sink: Option<&str>) -> Result<Handle, RecordError> {
    // based on https://github.com/JayXT/RecordAudioOutput/blob/main/record_audio_output_pw

    let target_sink = target_sink.unwrap_or("auto");
    let inner_expr = format!(
        "pw-record --target \"{}\" -P '{{ stream.capture.sink=true }}' - | lame -r -s 48 -m s -V7 - \"{}\"",
        target_sink,
        audio_path.display()
    );
    // afaik, must wrap in shell context otherwise it won't record the correct audio
    let expr = cmd("/usr/bin/env", ["bash", "-c", &inner_expr]);
    let command = expr.unchecked().start()?;
    Ok(command)
}
