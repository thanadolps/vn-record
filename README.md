# VN Record

VN Record is a custom tool for recording audio and screenshots of a specific process such as visual novel games.

This tool is likely specific to my system (Pop!\_OS) and not guranteed to work on others. It also assumes PipeWire audio configuration and invoke multiple commands to record and process audio and screenshots.

## System Requirements

- Linux desktop (Pop!\_OS tested)
- PipeWire (for pw-record)

```bash
sudo apt update
sudo apt install -y pipewire-bin sox libsox-fmt-mp3 lame xclip
```

## Features

### Main

- Record audio and screenshots of a specific process and save them in a directory
- Copy both the audio and screenshots to the clipboard
- Global hotkey control for most operations (eg. record, copy last record, copy last screenshot, copy last audio)

### Extra

- GUI for ease of use
- Automatically trim silence from the beginning and end of the audio

## Motivation

The main motivation for this tool is to aid in my word/sentence mining workflow for visual novel games. From my research, I haven't found any tools which satisfies the features I need for my workflow and runs on Linux.

### Workflow

#### Setup

- Use [SteamTinkerLaunch](https://github.com/sonic2kk/steamtinkerlaunch) to run VN game alongside [Agent](https://github.com/0xDC00/agent). Enable clipboard in Agent and attach the VN process to Agent.
- Launch _VN Record_ and select the VN process.
- Open [Migaku](https://migaku.com)'s reader with clipboard mode enabled, along with card creator.

#### Mining

1. [Agent](https://github.com/0xDC00/agent) automatically extracts text from the VN game which [Migaku](https://migaku.com)'s clipboard mode automatically receives
2. In [Migaku](https://migaku.com)'s clipboard, select a word to mine and "send to card creator"
3. Start recording in _VN Record_, replay audio in VN game, then stop recording.

   - It's recommended to minimize gap between replaying audio and starting/stopping recording (global hotkey is helpful here).

4. Add the audio and screenshots to [Migaku](https://migaku.com)'s card creator by hovering over the audio/screenshot field and pasting (ctrl-v).
   - The audio and screenshots should already be automatically copied to clipboard by _VN Record_ when stop recording.
5. Adjust fields as needed and create card
