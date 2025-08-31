use std::{collections::HashMap, time::Duration};

use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use iced::{
    futures::{SinkExt, Stream, StreamExt},
    stream,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GHKMessage {
    Record,
    CopyLastScreenshot,
    CopyLastAudio,
}

pub struct GHKService {
    manage: GlobalHotKeyManager,
    map: HashMap<u32, GHKMessage>,
    rev_map: HashMap<GHKMessage, Vec<HotKey>>,
}

impl GHKService {
    pub fn new() -> Self {
        Self {
            manage: GlobalHotKeyManager::new().unwrap(),
            map: HashMap::new(),
            rev_map: HashMap::new(),
        }
    }

    pub fn bind(&mut self, hotkey: HotKey, message: GHKMessage) {
        self.manage.register(hotkey).unwrap();
        self.map.insert(hotkey.id, message);
        self.rev_map.entry(message).or_default().push(hotkey);
    }

    pub fn get_message(&self, key: HotKey) -> Option<GHKMessage> {
        self.map.get(&key.id).copied()
    }

    pub fn get_key(&self, message: GHKMessage) -> &[HotKey] {
        self.rev_map.get(&message).map_or(&[], |v| v)
    }

    pub fn stream<'a>(&'a self) -> impl Stream<Item = GHKMessage> + 'a {
        ghk_stream().filter_map(move |ev| async move {
            if ev.state != HotKeyState::Pressed {
                return None;
            }
            self.map.get(&ev.id).copied()
        })
    }
}

fn ghk_stream() -> impl Stream<Item = GlobalHotKeyEvent> {
    let receiver = GlobalHotKeyEvent::receiver();
    stream::channel(16, move |mut sender| async move {
        loop {
            if let Ok(event) = receiver.try_recv() {
                sender.send(event).await.unwrap();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
}
