//! `REALITY64_INPUT_RECORD` and `REALITY64_INPUT_REPLAY` select files keyed by `osViSwapBuffer` count.
//! Input is latched per frame so record and replay use the same sampling boundary.
//!
//! File format: a `reality64-input 1 <controller bits>` header, then one `<frame> <buttons hex> <x> <y>`
//! line per frame on which the state changed, then `end <frame>`, the last frame recorded. Without
//! an `end` line, replay ends at the last change.

use std::io::{Seek, SeekFrom, Write};
use std::sync::Mutex;

use crate::gamepad::snapshot::GamepadSnapshot;

const HEADER: &str = "reality64-input 1";

/// A recorded state change: from `frame` on, the pad reads `button`, `stick_x`, `stick_y`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    pub frame: u64,
    pub button: u16,
    pub stick_x: i8,
    pub stick_y: i8,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Recording {
    pub bits: u8,
    pub changes: Vec<Change>,
    pub end: Option<u64>,
}

pub fn parse(text: &str) -> Result<Recording, String> {
    let mut lines = text.lines();
    let bits = lines
        .next()
        .and_then(|header| header.strip_prefix(HEADER))
        .and_then(|bits| bits.trim().parse().ok())
        .ok_or("missing `reality64-input 1 <bits>` header")?;
    let mut lines: Vec<_> = lines.filter(|line| !line.trim().is_empty()).collect();
    let end = match lines.last().and_then(|line| line.strip_prefix("end ")) {
        Some(frame) => {
            let frame = frame
                .trim()
                .parse()
                .map_err(|_| format!("bad end frame: {frame:?}"))?;
            lines.pop();
            Some(frame)
        }
        None => None,
    };
    let changes = lines
        .into_iter()
        .map(|line| {
            let fields: Vec<_> = line.split_whitespace().collect();
            let [frame, button, x, y] = fields[..] else {
                return Err(format!("expected 4 fields: {line:?}"));
            };
            Ok(Change {
                frame: frame.parse().map_err(|_| format!("bad frame: {line:?}"))?,
                button: u16::from_str_radix(button, 16)
                    .map_err(|_| format!("bad buttons: {line:?}"))?,
                stick_x: x.parse().map_err(|_| format!("bad stick x: {line:?}"))?,
                stick_y: y.parse().map_err(|_| format!("bad stick y: {line:?}"))?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if changes
        .windows(2)
        .any(|pair| pair[0].frame >= pair[1].frame)
    {
        return Err("frames must increase".into());
    }
    let last = changes.last().map(|change| change.frame);
    if end.is_some_and(|end| last.is_some_and(|last| end < last)) {
        return Err("end precedes the last change".into());
    }
    Ok(Recording {
        bits,
        changes,
        end: end.or(last),
    })
}

fn format_change(change: &Change) -> String {
    format!(
        "{} {:04x} {} {}\n",
        change.frame, change.button, change.stick_x, change.stick_y
    )
}

struct Replay {
    bits: u8,
    changes: Vec<Change>,
    end: Option<u64>,
    next: usize,
    done: bool,
}

impl From<Recording> for Replay {
    fn from(recording: Recording) -> Self {
        Replay {
            bits: recording.bits,
            changes: recording.changes,
            end: recording.end,
            next: 0,
            done: false,
        }
    }
}

impl Replay {
    fn state(&mut self, frame: u64, latched: GamepadSnapshot) -> Option<GamepadSnapshot> {
        if self.done {
            return None;
        }
        let mut state = latched;
        while self
            .changes
            .get(self.next)
            .is_some_and(|c| c.frame <= frame)
        {
            let c = self.changes[self.next];
            state = GamepadSnapshot {
                button: c.button,
                stick_x: c.stick_x,
                stick_y: c.stick_y,
                errno: 0,
                bits: self.bits,
            };
            self.next += 1;
        }
        if self.end.is_none_or(|end| frame > end) {
            self.done = true;
            log::warn!("input replay finished at frame {frame}");
            return None;
        }
        Some(state)
    }
}

struct Record {
    file: std::fs::File,
    last: Option<(u16, i8, i8)>,
    /// Offset of the `end` line, which each frame overwrites.
    end_offset: u64,
}

impl Record {
    fn new(file: std::fs::File) -> Self {
        Record {
            file,
            last: None,
            end_offset: 0,
        }
    }

    fn write(&mut self, frame: u64, pad: GamepadSnapshot) {
        let mut text = String::new();
        if self.last.is_none() {
            text += &format!("{HEADER} {}\n", pad.bits);
        }
        let state = (pad.button, pad.stick_x, pad.stick_y);
        if self.last != Some(state) {
            self.last = Some(state);
            text += &format_change(&Change {
                frame,
                button: pad.button,
                stick_x: pad.stick_x,
                stick_y: pad.stick_y,
            });
        }
        let offset = self.end_offset;
        self.end_offset += text.len() as u64;
        // Frames only grow, so the new `end` line always covers the previous one.
        text += &format!("end {frame}\n");
        let _ = self
            .file
            .seek(SeekFrom::Start(offset))
            .and_then(|_| self.file.write_all(text.as_bytes()));
    }
}

struct Script {
    replay: Option<Replay>,
    record: Option<Record>,
    frame: u64,
    latched: GamepadSnapshot,
}

static SCRIPT: Mutex<Option<Script>> = Mutex::new(None);

fn replay_from_env() -> Option<Replay> {
    let path = std::env::var_os("REALITY64_INPUT_REPLAY")?;
    match std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|text| parse(&text))
    {
        Ok(recording) => {
            log::warn!(
                "input replay: {} ({} changes)",
                std::path::Path::new(&path).display(),
                recording.changes.len()
            );
            Some(recording.into())
        }
        Err(e) => {
            log::error!(
                "input replay {}: {e}; using live input",
                path.to_string_lossy()
            );
            None
        }
    }
}

fn record_from_env() -> Option<Record> {
    let path = std::env::var_os("REALITY64_INPUT_RECORD")?;
    match std::fs::File::create(&path) {
        Ok(file) => {
            log::warn!("input record: {}", std::path::Path::new(&path).display());
            Some(Record::new(file))
        }
        Err(e) => {
            log::error!(
                "input record {}: {e}; not recording",
                path.to_string_lossy()
            );
            None
        }
    }
}

fn with_script<T>(f: impl FnOnce(&mut Script) -> T) -> T {
    let mut guard = SCRIPT.lock().unwrap();
    let script = guard.get_or_insert_with(|| {
        Script::new(
            replay_from_env(),
            record_from_env(),
            crate::gamepad::snapshot::load(),
        )
    });
    f(script)
}

impl Script {
    /// Frame 0 is latched once here, so reads before the first swap all agree.
    fn new(replay: Option<Replay>, record: Option<Record>, live: GamepadSnapshot) -> Self {
        let mut script = Script {
            replay,
            record,
            frame: 0,
            latched: live,
        };
        if script.scripted() {
            script.latch(live);
        }
        script
    }

    fn scripted(&self) -> bool {
        self.replay.is_some() || self.record.is_some()
    }

    fn latch(&mut self, live: GamepadSnapshot) {
        let replayed = self
            .replay
            .as_mut()
            .and_then(|replay| replay.state(self.frame, self.latched));
        self.latched = replayed.unwrap_or(live);
        if let Some(record) = &mut self.record {
            record.write(self.frame, self.latched);
        }
    }

    fn read(&self, live: GamepadSnapshot) -> GamepadSnapshot {
        if self.scripted() {
            self.latched
        } else {
            live
        }
    }
}

/// The guest presented a frame: latch the input the next frame reads.
pub fn on_swap() {
    let live = crate::gamepad::snapshot::load();
    with_script(|script| {
        script.frame += 1;
        script.latch(live);
    });
}

pub fn read() -> GamepadSnapshot {
    let live = crate::gamepad::snapshot::load();
    with_script(|script| script.read(live))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(button: u16, stick_x: i8, stick_y: i8) -> GamepadSnapshot {
        GamepadSnapshot {
            button,
            stick_x,
            stick_y,
            errno: 0,
            bits: 1,
        }
    }

    #[test]
    fn parse_roundtrips_and_rejects_bad_input() {
        let changes = [
            Change {
                frame: 0,
                button: 0,
                stick_x: 0,
                stick_y: 0,
            },
            Change {
                frame: 7,
                button: 0x9000,
                stick_x: -80,
                stick_y: 12,
            },
        ];
        let text = format!(
            "{HEADER} 1\n{}{}",
            format_change(&changes[0]),
            format_change(&changes[1])
        );
        let parsed = |end| Recording {
            bits: 1,
            changes: changes.to_vec(),
            end: Some(end),
        };
        assert_eq!(parse(&text), Ok(parsed(7)));
        assert_eq!(parse(&format!("{text}end 9\n")), Ok(parsed(9)));
        assert!(parse(&format!("{text}end 5\n")).is_err());
        assert!(parse("0 0000 0 0\n").is_err());
        assert!(parse(&format!("{HEADER} 1\n3 0000 0 0\n3 8000 0 0\n")).is_err());
        assert!(parse(&format!("{HEADER} 1\n3 zz 0 0\n")).is_err());
    }

    #[test]
    fn record_latches_per_frame_and_writes_only_changes() {
        let path = std::env::temp_dir().join(format!("reality64_input_{}.txt", std::process::id()));
        let mut script = Script {
            replay: None,
            record: Some(Record::new(std::fs::File::create(&path).unwrap())),
            frame: 0,
            latched: GamepadSnapshot::default(),
        };
        script.latch(pad(0, 0, 0));
        assert_eq!(
            script.read(pad(0x8000, 0, 0)).button,
            0,
            "mid-frame input waits for the swap"
        );
        for (frame, live) in [
            (1, pad(0, 0, 0)),
            (2, pad(0x8000, 0, 0)),
            (3, pad(0x8000, 0, 0)),
            (4, pad(0, 40, 0)),
        ] {
            script.frame = frame;
            script.latch(live);
            assert_eq!(script.read(pad(0x4000, 1, 1)).button, live.button);
        }
        drop(script);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            format!("{HEADER} 1\n0 0000 0 0\n2 8000 0 0\n4 0000 40 0\nend 4\n")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn replay_feeds_recorded_frames_then_returns_to_live() {
        let recording = parse(&format!(
            "{HEADER} 1\n0 0000 0 0\n2 8000 0 0\n4 0000 40 -5\n"
        ))
        .unwrap();
        let mut script = Script {
            replay: Some(recording.into()),
            record: None,
            frame: 0,
            latched: GamepadSnapshot::default(),
        };
        let live = pad(0x0010, 9, 9);
        let mut seen = vec![];
        for frame in 0..6 {
            script.frame = frame;
            script.latch(live);
            let read = script.read(live);
            seen.push((read.button, read.stick_x, read.stick_y));
        }
        assert_eq!(
            seen,
            [
                (0, 0, 0),
                (0, 0, 0),
                (0x8000, 0, 0),
                (0x8000, 0, 0),
                (0, 40, -5),
                (0x0010, 9, 9)
            ]
        );
    }

    #[test]
    fn replay_then_record_extends_the_route() {
        let path =
            std::env::temp_dir().join(format!("reality64_input_ext_{}.txt", std::process::id()));
        let recording = parse(&format!("{HEADER} 1\n0 0000 0 0\n2 8000 0 0\n")).unwrap();
        let mut script = Script {
            replay: Some(recording.into()),
            record: Some(Record::new(std::fs::File::create(&path).unwrap())),
            frame: 0,
            latched: GamepadSnapshot::default(),
        };
        for (frame, live) in [
            (0, pad(0x4000, 0, 0)),
            (1, pad(0x4000, 0, 0)),
            (2, pad(0x4000, 0, 0)),
            (3, pad(0, 30, 0)),
            (4, pad(0, 30, 0)),
        ] {
            script.frame = frame;
            script.latch(live);
        }
        drop(script);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            format!("{HEADER} 1\n0 0000 0 0\n2 8000 0 0\n3 0000 30 0\nend 4\n")
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn replay_holds_the_last_state_until_the_end_frame() {
        let recording = parse(&format!("{HEADER} 1\n0 0000 0 0\n2 8000 0 0\nend 4\n")).unwrap();
        let mut script = Script::new(Some(recording.into()), None, GamepadSnapshot::default());
        let live = pad(0x0010, 9, 9);
        let mut seen = vec![script.read(live).button];
        for frame in 1..6 {
            script.frame = frame;
            script.latch(live);
            seen.push(script.read(live).button);
        }
        assert_eq!(seen, [0, 0, 0x8000, 0x8000, 0x8000, 0x0010]);
    }

    #[test]
    fn frame_zero_is_latched_once() {
        let path =
            std::env::temp_dir().join(format!("reality64_input_zero_{}.txt", std::process::id()));
        let record = Record::new(std::fs::File::create(&path).unwrap());
        let script = Script::new(None, Some(record), pad(0, 0, 0));
        assert_eq!(script.read(pad(0x8000, 0, 0)).button, 0);
        assert_eq!(script.read(pad(0x4000, 0, 0)).button, 0);
        drop(script);
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, format!("{HEADER} 1\n0 0000 0 0\nend 0\n"));
        let _ = std::fs::remove_file(&path);
    }
}
