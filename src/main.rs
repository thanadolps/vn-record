mod clipboard;
mod hotkey;
mod process;
mod record;

use std::{path::PathBuf, sync::LazyLock, time::Duration};

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use hotkey::{GHKMessage, GHKService};
use iced::{
    Alignment::{Center, End, Start},
    Element, Font,
    Length::Fill,
    Pixels, Subscription, Task, Theme,
    font::{self, Weight},
    futures::{SinkExt, Stream, StreamExt},
    keyboard, stream, theme,
    widget::{
        Column, Container, Row, button, center, column, container, image, pick_list, text, value,
    },
};
use process::{Process, processes};
use record::{RecordConfig, RecordedData, Recorder};

static GHK: LazyLock<GHKService> = LazyLock::new(|| {
    let mut ghk = GHKService::new();
    ghk.bind(
        HotKey::new(Some(Modifiers::SHIFT), Code::Space),
        GHKMessage::Record,
    );
    ghk.bind(
        HotKey::new(Some(Modifiers::SHIFT), Code::Digit1),
        GHKMessage::CopyLastScreenshot,
    );
    ghk.bind(
        HotKey::new(Some(Modifiers::SHIFT), Code::Digit2),
        GHKMessage::CopyLastAudio,
    );
    ghk
});

#[derive(Debug, Clone)]
enum Message {
    RefreshProcessesList,
    ProcessSelected(process::Process),
    ProcessDeselected,
    StartRecord(process::Process),
    StopRecord,
    ToggleRecord,
    Tick(std::time::Instant),
    CopyLastScreenshot,
    CopyLastAudio,
    OpenOutDir,
}

enum Page {
    Main,
    Setting, // TODO
}

struct RecordSession {
    recorder: Recorder,
    elasped: std::time::Duration,
    start_time: std::time::Instant,
}

struct VNRecord {
    page: Page,
    process_list: Vec<process::Process>,
    selected_process: Option<process::Process>,
    record_session: Option<RecordSession>,
    last_recorded: Option<RecordedData>,

    out_dir: PathBuf,
}

impl Default for VNRecord {
    fn default() -> Self {
        Self {
            page: Page::Main,
            process_list: processes().unwrap(),
            selected_process: None,
            record_session: None,
            last_recorded: None,

            out_dir: default_output_dir(),
        }
    }
}

impl VNRecord {
    pub fn view(&self) -> Container<Message> {
        let header = Column::new()
            .push(text("VN Record").size(40))
            .push(self.process_bar());

        center(
            Column::new()
                .push(header)
                .push_maybe(self.selected_process.as_ref().map(|p| self.main_view(p)))
                .push(self.setting_view())
                .spacing(40),
        )
        .padding(20)
    }

    fn process_bar(&self) -> Row<Message> {
        let process_selector = || {
            Row::new()
                .push(
                    pick_list(
                        self.process_list.as_slice(),
                        self.selected_process.clone(),
                        Message::ProcessSelected,
                    )
                    .placeholder("Attach a process"),
                )
                .push(button("Refresh").on_press(Message::RefreshProcessesList))
        };

        let process_display = |process: &Process| {
            let text = value(process)
                .font(Font {
                    weight: Weight::Bold,
                    style: font::Style::Italic,
                    ..Default::default()
                })
                .size(20)
                .style(text::secondary);

            Row::new()
                .push(text)
                .push(
                    button("X")
                        .on_press_maybe((!self.is_recording()).then(|| Message::ProcessDeselected))
                        .style(button::secondary),
                )
                .spacing(8)
        };

        if let Some(process) = &self.selected_process {
            process_display(process).into()
        } else {
            process_selector().into()
        }
    }

    fn main_view(&self, selected_process: &process::Process) -> Element<Message> {
        fn duration_str(duration: Duration) -> String {
            let minutes = duration.as_secs() / 60;
            let seconds = duration.as_secs() % 60;
            format!("{:0>2}:{:0>2}", minutes, seconds)
        }

        let mut c = Column::new().spacing(20);

        // Last Recorded
        if let Some(lr) = &self.last_recorded {
            c = c.push(
                Column::new()
                    .align_x(Center)
                    .push(image(&lr.screenshot_path).height(256))
                    .push(text(duration_str(lr.duration)).size(20)),
            )
        }

        c = c.push(if let Some(rs) = &self.record_session {
            let duration = text(duration_str(rs.elasped)).size(30);

            Row::new()
                .push(
                    button("Stop")
                        .on_press(Message::StopRecord)
                        .style(button::danger),
                )
                .push(duration)
                .spacing(30)
                .into()
        } else {
            Element::from(button("Record").on_press(Message::StartRecord(selected_process.clone())))
        });

        c.into()
    }

    fn setting_view(&self) -> Element<Message> {
        let mut elems: Vec<(&str, Element<Message>)> = vec![(
            "Output Folder",
            button(
                value(self.out_dir.display())
                    .size(10)
                    .style(text::secondary),
            )
            .style(button::text)
            .padding(0)
            .on_press(Message::OpenOutDir)
            .into(),
        )];
        if let Some(r_key) = GHK.get_key(GHKMessage::Record).get(0) {
            elems.push((
                "Start/Stop Record",
                value(r_key).size(10).style(text::secondary).into(),
            ));
        }
        if let Some(cls_key) = GHK.get_key(GHKMessage::CopyLastScreenshot).get(0) {
            elems.push((
                "Copy Last Screenshot",
                value(cls_key).size(10).style(text::secondary).into(),
            ));
        }
        if let Some(cla_key) = GHK.get_key(GHKMessage::CopyLastAudio).get(0) {
            elems.push((
                "Copy Last Audio",
                value(cla_key).size(10).style(text::secondary).into(),
            ));
        }

        // Convert into table-like layout
        let (labels, contents): (Vec<&str>, Vec<Element<Message>>) = elems.into_iter().unzip();
        let labels_col =
            Column::from_iter(labels.into_iter().map(|label| text(label).size(10).into()))
                .spacing(4)
                .align_x(Start);
        let contents_col = Column::from_iter(contents.into_iter())
            .spacing(4)
            .align_x(End);
        Row::new()
            .spacing(20)
            .push(labels_col)
            .push(contents_col)
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = if self.record_session.is_some() {
            iced::time::every(std::time::Duration::from_millis(100)).map(Message::Tick)
        } else {
            Subscription::none()
        };

        let ghk = Subscription::run(|| {
            GHK.stream().map(|msg| match msg {
                GHKMessage::Record => Message::ToggleRecord,
                GHKMessage::CopyLastScreenshot => Message::CopyLastScreenshot,
                GHKMessage::CopyLastAudio => Message::CopyLastAudio,
            })
        });

        Subscription::batch([tick, ghk])
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::RefreshProcessesList => {
                self.process_list = processes().unwrap();
            }
            Message::ProcessSelected(process) => {
                self.selected_process = Some(process);
            }
            Message::ProcessDeselected => {
                if let Some(rs) = self.record_session.take() {
                    eprintln!("Process deselected while recording, recording forced to stop");
                    rs.recorder.stop().unwrap();
                }
                self.selected_process = None;
            }
            Message::StartRecord(process) => {
                let recorder = Recorder::start(RecordConfig {
                    process,
                    output_dir: self.out_dir.clone(),
                });
                self.record_session = Some(RecordSession {
                    recorder,
                    elasped: Default::default(),
                    start_time: std::time::Instant::now(),
                });
                println!("Start recording");
            }
            Message::StopRecord => {
                if let Some(rs) = self.record_session.take() {
                    let data = rs.recorder.stop().unwrap();
                    self.last_recorded = Some(data);
                }
                println!("Stop recording");
            }
            Message::ToggleRecord => {
                let Some(selected_process) = self.selected_process.clone() else {
                    return;
                };
                println!("Toggle recording");
                match self.record_session {
                    Some(_) => self.update(Message::StopRecord),
                    None => self.update(Message::StartRecord(selected_process)),
                }
            }
            Message::Tick(now) => {
                if let Some(rs) = &mut self.record_session {
                    rs.elasped = now.duration_since(rs.start_time)
                }
            }
            Message::CopyLastScreenshot => {
                if let Some(lr) = &self.last_recorded {
                    clipboard::write_image(&lr.screenshot_path);
                    println!("Last screenshot copied to clipboard");
                }
            }
            Message::CopyLastAudio => {
                if let Some(lr) = &self.last_recorded {
                    clipboard::write_file_uri(&lr.audio_path);
                    println!("Last audio copied to clipboard");
                }
            }
            Message::OpenOutDir => {
                let output_dir = &self.out_dir;
                if output_dir.exists() {
                    open::that(output_dir).unwrap();
                }
            }
        }
    }

    fn is_recording(&self) -> bool {
        self.record_session.is_some()
    }
}

fn default_output_dir() -> PathBuf {
    let mut output_dir = dirs::data_local_dir().unwrap();
    output_dir.push("vn_record");
    if !output_dir.exists() {
        std::fs::create_dir_all(&output_dir).unwrap();
    }

    // TODO: remove this line
    println!("default output dir is {}", output_dir.display());

    output_dir
}

fn main() -> iced::Result {
    iced::application("VN Record", VNRecord::update, VNRecord::view)
        .subscription(VNRecord::subscription)
        .theme(|_| Theme::Dark)
        .run()
}
