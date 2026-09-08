use anyhow::{bail, Context, Result};
use fast3d::capture::{CaptureSequence, Fixture, Provenance};
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Once;

pub(super) enum Selection {
    Frames(FrameSelection),
    Sequence(SequenceSelection),
}

impl Selection {
    pub fn from_env() -> Result<Option<Self>> {
        let selection = Self::parse(
            std::env::var_os("FAST3D_CAPTURE_DIR"),
            std::env::var("FAST3D_CAPTURE_FRAMES").ok(),
            std::env::var_os("FAST3D_CAPTURE_SEQUENCE"),
            std::env::var("FAST3D_CAPTURE_WARMUP_FRAMES").ok(),
            std::env::var("FAST3D_CAPTURE_PRESENTATIONS").ok(),
        )?;
        if let Some(Self::Sequence(sequence)) = &selection {
            static CONTEXTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let context = CONTEXTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            log::info!(
                "capture sequence context initialization #{context} (pid {}): {}",
                std::process::id(),
                sequence.path.display()
            );
            if context != 1 {
                bail!("capture sequence already initialized in this process; refusing to restart recording in a second render context");
            }
        }
        Ok(selection)
    }

    pub fn install_shutdown_handler(&self) {
        if matches!(self, Self::Sequence(_)) {
            static INSTALL: Once = Once::new();
            INSTALL.call_once(|| {
                match ctrlc::try_set_handler(crate::ultra::request_shutdown) {
                    Ok(()) => log::info!("capture shutdown handler installed: SIGINT/SIGTERM/SIGHUP save a completed prefix; SIGKILL cannot save"),
                    Err(error) => log::warn!("capture shutdown handler unavailable: {error}; a killed run will not save an unfinished capture; endpoint and normal window-close saves remain enabled"),
                }
            });
        }
    }

    fn parse(
        directory: Option<OsString>,
        frames: Option<String>,
        sequence: Option<OsString>,
        warmup: Option<String>,
        presentations: Option<String>,
    ) -> Result<Option<Self>> {
        match (directory, sequence) {
            (Some(_), Some(_)) => {
                bail!("FAST3D_CAPTURE_DIR and FAST3D_CAPTURE_SEQUENCE are mutually exclusive")
            }
            (Some(directory), None) => {
                if directory.is_empty() {
                    bail!("FAST3D_CAPTURE_DIR is empty");
                }
                let frames =
                    frames.context("FAST3D_CAPTURE_FRAMES is required with FAST3D_CAPTURE_DIR")?;
                Ok(Some(Self::Frames(FrameSelection {
                    directory: directory.into(),
                    frames: parse_serials(&frames)?,
                })))
            }
            (None, Some(path)) => Ok(Some(Self::Sequence(SequenceSelection::parse(
                path,
                warmup,
                presentations,
            )?))),
            (None, None) => Ok(None),
        }
    }
}

fn parse_serials(value: &str) -> Result<Vec<u64>> {
    let mut serials = value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let serial = s
                .parse::<u64>()
                .with_context(|| format!("invalid capture frame serial: {s:?}"))?;
            if serial == 0 {
                bail!("capture frame serials start at one");
            }
            Ok(serial)
        })
        .collect::<Result<Vec<_>>>()?;
    if serials.is_empty() {
        bail!("capture requires at least one frame serial");
    }
    serials.sort_unstable();
    serials.dedup();
    Ok(serials)
}

pub(super) struct FrameSelection {
    directory: PathBuf,
    frames: Vec<u64>,
}

impl FrameSelection {
    pub fn contains(&self, serial: u64) -> bool {
        self.frames.binary_search(&serial).is_ok()
    }

    pub fn write(&self, fixture: &Fixture) -> Result<()> {
        let path = self
            .directory
            .join(format!("frame-{:06}.f3dcap", fixture.frame.serial));
        CaptureOutput::create(&path)?.write(&fixture.to_bytes()?)?;
        log::info!(
            "captured frame {} to {}",
            fixture.frame.serial,
            path.display()
        );
        Ok(())
    }
}

pub(super) struct SequenceSelection {
    path: PathBuf,
    warmup_frames: u32,
    presentations: Vec<u64>,
}

impl SequenceSelection {
    fn parse(
        path: OsString,
        warmup: Option<String>,
        presentations: Option<String>,
    ) -> Result<Self> {
        if path.is_empty() {
            bail!("FAST3D_CAPTURE_SEQUENCE is empty");
        }
        let warmup_frames = warmup
            .as_deref()
            .unwrap_or("0")
            .trim()
            .parse::<u32>()
            .context("invalid FAST3D_CAPTURE_WARMUP_FRAMES")?;
        let presentations = parse_serials(
            &presentations
                .context("FAST3D_CAPTURE_PRESENTATIONS is required with FAST3D_CAPTURE_SEQUENCE")?,
        )?;
        if presentations[0] <= u64::from(warmup_frames) {
            bail!("FAST3D_CAPTURE_PRESENTATIONS must be after FAST3D_CAPTURE_WARMUP_FRAMES");
        }
        Ok(Self {
            path: path.into(),
            warmup_frames,
            presentations,
        })
    }

    fn last_serial(&self) -> u64 {
        *self.presentations.last().unwrap()
    }

    fn prefix(&self, frames: u64) -> Result<(u32, Vec<u64>)> {
        if frames == 0 {
            bail!("no completed frames; no sequence can be written");
        }
        let warmup = u64::from(self.warmup_frames).min(frames - 1) as u32;
        let mut presentations: Vec<_> = self
            .presentations
            .iter()
            .copied()
            .take_while(|&serial| serial <= frames)
            .collect();
        if presentations.is_empty() {
            presentations.push(frames);
        }
        Ok((warmup, presentations))
    }
}

pub(super) struct SequenceRecording {
    pub recorder: CaptureSequence,
    selection: SequenceSelection,
    output: CaptureOutput,
    completed_frames: u64,
}

impl SequenceRecording {
    pub fn begin(renderer: &mut fast3d::Renderer, selection: SequenceSelection) -> Result<Self> {
        let output = CaptureOutput::create(&selection.path)?;
        let recorder = CaptureSequence::begin(renderer, 0)?;
        log::info!(
            "capture sequence started: {} frames 1..={}, warmup {}, presentations {:?}",
            selection.path.display(),
            selection.last_serial(),
            selection.warmup_frames,
            selection.presentations
        );
        Ok(Self {
            recorder,
            selection,
            output,
            completed_frames: 0,
        })
    }

    pub fn present(&mut self, renderer: &mut fast3d::Renderer) -> Result<bool> {
        self.recorder.present_last(renderer)?;
        self.completed_frames += 1;
        Ok(self.completed_frames == self.selection.last_serial())
    }

    pub fn finish(self, reason: &str) -> Result<()> {
        let (warmup, presentations) = self.selection.prefix(self.completed_frames)?;
        let complete = self.completed_frames == self.selection.last_serial();
        if !complete {
            log::warn!("capture sequence incomplete ({reason}): {}/{} frames; saving warmup {warmup}, presentations {presentations:?}", self.completed_frames, self.selection.last_serial());
        }
        let sequence = self.recorder.finish(warmup, presentations)?;
        self.output.write(&sequence.to_bytes()?)?;
        log::info!(
            "capture sequence {}: wrote {} frames to {}",
            if complete { "complete" } else { "partial" },
            self.completed_frames,
            self.selection.path.display()
        );
        Ok(())
    }
}

struct CaptureOutput {
    path: PathBuf,
    temporary: PathBuf,
    file: File,
}

impl CaptureOutput {
    fn create(path: &Path) -> Result<Self> {
        if path.try_exists()? {
            bail!("capture output already exists: {}", path.display());
        }
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let (temporary, file) = loop {
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut temporary = path.as_os_str().to_owned();
            temporary.push(format!(".{}-{id}.partial", std::process::id()));
            let temporary = PathBuf::from(temporary);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("create capture staging file {}", temporary.display())
                    })
                }
            }
        };
        Ok(Self {
            path: path.to_owned(),
            temporary,
            file,
        })
    }

    fn write(mut self, bytes: &[u8]) -> Result<()> {
        self.file
            .write_all(bytes)
            .with_context(|| format!("write capture staging file {}", self.temporary.display()))?;
        self.file
            .sync_all()
            .with_context(|| format!("sync capture staging file {}", self.temporary.display()))?;
        // Publish only complete bytes, without replacing a capture from another run.
        std::fs::hard_link(&self.temporary, &self.path).with_context(|| {
            format!(
                "publish capture {}; staging file retained at {}",
                self.path.display(),
                self.temporary.display()
            )
        })?;
        if let Err(error) = std::fs::remove_file(&self.temporary) {
            log::warn!(
                "remove capture staging file {}: {error}",
                self.temporary.display()
            );
        }
        Ok(())
    }
}

impl Drop for CaptureOutput {
    fn drop(&mut self) {
        if self
            .file
            .metadata()
            .is_ok_and(|metadata| metadata.len() == 0)
        {
            let _ = std::fs::remove_file(&self.temporary);
        }
    }
}

pub(super) fn provenance(serial: u64) -> Provenance {
    Provenance {
        decomp_revision: std::env::var("FAST3D_CAPTURE_REVISION")
            .unwrap_or_else(|_| "unknown".into()),
        source_symbols: std::env::var("FAST3D_CAPTURE_SYMBOLS")
            .unwrap_or_else(|_| "unknown (live task)".into()),
        command_vector: format!("helix/frame/{serial}"),
        synthetic_data: "none; live guest memory".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_directory() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let directory = std::env::temp_dir().join(format!(
            "helix-capture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir(&directory).unwrap();
        directory
    }

    #[cfg(unix)]
    fn reset_shutdown_signals_in_child() {
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            unsafe {
                assert_ne!(libc::signal(signal, libc::SIG_DFL), libc::SIG_ERR);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn sequence_termination_requests_runtime_shutdown() {
        const CHILD: &str = "HELIX_TEST_CAPTURE_TERMINATION";
        if let Ok(signal) = std::env::var(CHILD) {
            reset_shutdown_signals_in_child();
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Info)
                .init();
            let selection = Selection::parse(
                None,
                None,
                Some("unused.f3dcap".into()),
                None,
                Some("1".into()),
            )
            .unwrap()
            .unwrap();
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    scope.spawn(|| selection.install_shutdown_handler());
                }
            });
            assert!(!crate::ultra::shutdown_requested());
            assert!(std::process::Command::new("kill")
                .arg(format!("-{signal}"))
                .arg(std::process::id().to_string())
                .status()
                .unwrap()
                .success());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while !crate::ultra::shutdown_requested() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "signal did not request runtime shutdown"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            return;
        }
        for signal in ["INT", "TERM", "HUP"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "render::capture::tests::sequence_termination_requests_runtime_shutdown",
                    "--nocapture",
                ])
                .env(CHILD, signal)
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success(),
                "{signal}: {}\n{stderr}",
                String::from_utf8_lossy(&output.stdout),
            );
            assert_eq!(
                stderr.matches("capture shutdown handler installed").count(),
                1
            );
            assert!(!stderr.contains("capture shutdown handler unavailable"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn shutdown_handler_conflicts_warn_once_and_do_not_stop_capture_startup() {
        const CHILD: &str = "HELIX_TEST_CAPTURE_HANDLER_CONFLICT";
        if let Ok(conflict) = std::env::var(CHILD) {
            reset_shutdown_signals_in_child();
            env_logger::Builder::new()
                .filter_level(log::LevelFilter::Info)
                .init();
            let signal = match conflict.as_str() {
                "INT" => libc::SIGINT,
                "TERM" => libc::SIGTERM,
                "HUP" => libc::SIGHUP,
                _ => unreachable!(),
            };
            unsafe {
                assert_ne!(libc::signal(signal, libc::SIG_IGN), libc::SIG_ERR);
            }
            let selection = Selection::parse(
                None,
                None,
                Some("unused.f3dcap".into()),
                None,
                Some("1".into()),
            )
            .unwrap()
            .unwrap();
            selection.install_shutdown_handler();
            assert!(!crate::ultra::shutdown_requested());
            unsafe {
                assert_eq!(libc::signal(signal, libc::SIG_DFL), libc::SIG_IGN);
            }
            selection.install_shutdown_handler();
            unsafe {
                assert_eq!(libc::signal(signal, libc::SIG_DFL), libc::SIG_DFL);
            }
            assert!(!crate::ultra::shutdown_requested());
            return;
        }
        for conflict in ["INT", "TERM", "HUP"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "render::capture::tests::shutdown_handler_conflicts_warn_once_and_do_not_stop_capture_startup",
                    "--nocapture",
                ])
                .env(CHILD, conflict)
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success(),
                "{conflict}: {}\n{stderr}",
                String::from_utf8_lossy(&output.stdout),
            );
            assert_eq!(stderr.matches("killed run will not save").count(), 1);
            assert!(!stderr.contains("capture shutdown handler installed"));
        }
    }

    #[test]
    fn sequence_selection_cannot_be_reinitialized() {
        const CHILD: &str = "HELIX_TEST_CAPTURE_SELECTION_OWNER";
        if std::env::var_os(CHILD).is_some() {
            let mut selection = Selection::from_env().unwrap();
            assert!(selection.is_some());
            for _ in 0..2 {
                let Err(error) = Selection::from_env() else {
                    panic!("a second context must not start another sequence");
                };
                assert!(error.to_string().contains("already initialized"));
                drop(selection.take());
            }
            return;
        }
        let directory = scratch_directory();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "render::capture::tests::sequence_selection_cannot_be_reinitialized",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env_remove("FAST3D_CAPTURE_DIR")
            .env("FAST3D_CAPTURE_SEQUENCE", directory.join("run.f3dcap"))
            .env("FAST3D_CAPTURE_WARMUP_FRAMES", "0")
            .env("FAST3D_CAPTURE_PRESENTATIONS", "1")
            .output()
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    fn fixture(serial: u64) -> Fixture {
        use fast3d::capture::{Frame, MemoryLayout, MemorySpan, SourceLayout, Task};
        Fixture {
            frame: Frame {
                serial,
                dither_seed: 0,
                config: fast3d::RendererConfig {
                    clear_policy: fast3d::ClearPolicy::Persist,
                    resolution_multiplier: 1,
                    sample_count: 1,
                    present_mode: wgpu::PresentMode::Fifo,
                    format: Some(wgpu::TextureFormat::Bgra8Unorm),
                    power_preference: wgpu::PowerPreference::HighPerformance,
                },
                width: 4,
                height: 4,
                vi: None,
                dual_source_blending: false,
            },
            tasks: vec![Task {
                entry: 0x100,
                microcode: fast3d::Microcode::F3d,
                data_format: fast3d::DataFormat::Fixed,
                order: 0,
                source: SourceLayout {
                    memory: MemoryLayout::IMAGE,
                    segments: [0; 16],
                },
                spans: vec![MemorySpan {
                    address: 0x100,
                    bytes: vec![0xb8, 0, 0, 0, 0, 0, 0, 0],
                }],
            }],
            provenance: Provenance {
                synthetic_data: "test end-DL".into(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn capture_modes_require_an_output_and_cannot_be_combined() {
        assert!(Selection::parse(
            None,
            Some("bad".into()),
            None,
            Some("bad".into()),
            Some("bad".into())
        )
        .unwrap()
        .is_none());
        assert!(Selection::parse(
            Some("frames".into()),
            Some("1".into()),
            Some("run.f3dcap".into()),
            None,
            Some("1".into())
        )
        .is_err());
        assert!(Selection::parse(Some("frames".into()), None, None, None, None).is_err());
        assert!(Selection::parse(Some("".into()), Some("1".into()), None, None, None).is_err());
    }

    #[test]
    fn per_frame_output_uses_fixture_serial_and_never_overwrites() {
        let directory = scratch_directory();
        let selection = FrameSelection {
            directory: directory.clone(),
            frames: vec![7],
        };
        assert!(selection.contains(7));
        assert!(!selection.contains(6));
        let fixture = fixture(7);
        selection.write(&fixture).unwrap();
        let path = directory.join("frame-000007.f3dcap");
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(Fixture::from_bytes(&bytes).unwrap(), fixture);
        assert!(selection.write(&fixture).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn output_is_unpublished_until_complete_and_refuses_a_competing_writer() {
        let directory = scratch_directory();
        let path = directory.join("run.f3dcap");
        let output = CaptureOutput::create(&path).unwrap();
        assert!(!path.exists());
        let competing = CaptureOutput::create(&path).unwrap();
        assert_ne!(output.temporary, competing.temporary);
        drop(competing);
        std::fs::write(&path, b"another run").unwrap();
        assert!(output.write(b"new capture").is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"another run");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn abandoned_empty_output_is_cleaned_up_and_can_be_retried() {
        let directory = scratch_directory();
        let path = directory.join("run.f3dcap");
        drop(CaptureOutput::create(&path).unwrap());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        CaptureOutput::create(&path)
            .unwrap()
            .write(b"retry")
            .unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"retry");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn partial_prefix_writes_a_valid_sequence_with_every_original_serial() {
        let directory = scratch_directory();
        let path = directory.join("run.f3dcap");
        let selection = SequenceSelection::parse(
            path.clone().into_os_string(),
            Some("10".into()),
            Some("12,20".into()),
        )
        .unwrap();
        let (warmup_frames, presentations) = selection.prefix(4).unwrap();
        let sequence = fast3d::capture::Sequence {
            frames: (1..=4).map(fixture).collect(),
            warmup_frames,
            presentations,
        };
        CaptureOutput::create(&path)
            .unwrap()
            .write(&sequence.to_bytes().unwrap())
            .unwrap();
        let saved = fast3d::capture::Sequence::from_bytes(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(
            saved
                .frames
                .iter()
                .map(|f| f.frame.serial)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(saved.warmup_frames, 3);
        assert_eq!(saved.presentations, vec![4]);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn serial_lists_accept_bsd_seq_trailing_comma() {
        assert_eq!(parse_serials(" 1, 5,,19,5, \n").unwrap(), vec![1, 5, 19]);
        for invalid in ["", ", ,", "0", "-1", "1,no", "18446744073709551616"] {
            assert!(parse_serials(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn sequence_selection_requires_presentations_after_warmup() {
        let selection = SequenceSelection::parse(
            "run.f3dcap".into(),
            Some("10".into()),
            Some("20,11,".into()),
        )
        .unwrap();
        assert_eq!(selection.warmup_frames, 10);
        assert_eq!(selection.presentations, vec![11, 20]);
        assert_eq!(selection.last_serial(), 20);
        for (path, warmup, presentations) in [
            ("", None, Some("1")),
            ("run.f3dcap", Some("-1"), Some("1")),
            ("run.f3dcap", Some("4294967296"), Some("1")),
            ("run.f3dcap", Some("10"), Some("10,11")),
            ("run.f3dcap", None, None),
        ] {
            assert!(SequenceSelection::parse(
                path.into(),
                warmup.map(str::to_owned),
                presentations.map(str::to_owned)
            )
            .is_err());
        }
    }

    #[test]
    fn interrupted_sequence_retains_reached_selections_or_last_frame() {
        let selection =
            SequenceSelection::parse("run.f3dcap".into(), Some("10".into()), Some("12,20".into()))
                .unwrap();
        assert_eq!(selection.prefix(20).unwrap(), (10, vec![12, 20]));
        assert_eq!(selection.prefix(15).unwrap(), (10, vec![12]));
        assert_eq!(selection.prefix(11).unwrap(), (10, vec![11]));
        assert_eq!(selection.prefix(4).unwrap(), (3, vec![4]));
        assert!(selection.prefix(0).is_err());
    }
}
