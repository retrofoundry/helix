//! render thread: owns fast3d::Renderer off the process main thread. Main creates the surface and
//! hands it here; this thread consumes DLs and presents. Aspect ratio is a main-fed global the C HUD reads.

use crate::ultra::rcp::{take_render_receiver, RenderMsg};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// helix's N64-machine boundary. `rdram()` returns a `HostRam` reader over the live game memory: the
/// DL entry (`commands as u64`, passed to `process_dl`) and everything it reaches are raw host
/// pointers into the running sm64 process. The `'a` witness on `HostRam` is inert `PhantomData`, so
/// the safety contract holds at runtime: the submitting guest blocks (run token held) for the whole
/// consume, so it cannot free or rebuild the single-buffered DL while the render thread reads it.
/// `vi()` defaults to `None`, so `present` scans out the last-rendered framebuffer (helix has no live
/// VI registers).
pub(crate) struct HelixHardware;
impl fast3d::Hardware for HelixHardware {
    fn rdram(&self) -> impl fast3d::Rdram + '_ {
        // SAFETY: see the type doc above — the host-pointer lifetime contract is runtime-enforced.
        unsafe { fast3d::HostRam::new(&[]) }
    }
}

/// Dedup-then-log diagnostics: a message is logged at most once per process lifetime, keyed on
/// `d.kind`'s `Display` string.
pub(crate) struct DedupLogSink;
impl fast3d::DiagSink for DedupLogSink {
    fn emit(&mut self, d: fast3d::Diagnostic) {
        use std::sync::Mutex;
        static SEEN: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);
        let mut guard = SEEN.lock().unwrap();
        let seen = guard.get_or_insert_with(std::collections::HashSet::new);
        let msg = d.kind.to_string();
        if seen.insert(msg.clone()) {
            log::warn!("hle diag @ {:#x}: {}", d.at, msg);
        }
    }
}

/// sm64's native display aspect (SCREEN_WIDTH / SCREEN_HEIGHT).
pub const NATIVE_ASPECT: f32 = 4.0 / 3.0;

static ASPECT_BITS: AtomicU32 = AtomicU32::new(NATIVE_ASPECT.to_bits());

/// winit can transiently emit a `u32::MAX` size, and a minimized window reports a zero dimension.
/// Both mean "ignore this size" for surface reconfigure AND aspect.
pub(crate) fn valid_surface_size(width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 || width == u32::MAX || height == u32::MAX {
        None
    } else {
        Some((width, height))
    }
}

/// Store the widescreen aspect from a window's physical pixel size, clamped to never render narrower
/// than native 4:3 (a tall window pillarboxes; no ultrawide upper clamp). Invalid/transient sizes fall
/// back to native rather than storing garbage.
pub fn set_aspect_from_size(width: u32, height: u32) {
    let aspect = match valid_surface_size(width, height) {
        Some((w, h)) => (w as f32 / h as f32).max(NATIVE_ASPECT),
        None => NATIVE_ASPECT,
    };
    ASPECT_BITS.store(aspect.to_bits(), Ordering::Relaxed);
}

pub fn aspect_ratio() -> f32 {
    f32::from_bits(ASPECT_BITS.load(Ordering::Relaxed))
}

/// C-facing aspect getter (cpp/helix.c calls this). Replaces the dead `_gui`-based path.
#[no_mangle]
pub extern "C" fn HLXAspectRatio() -> f32 {
    aspect_ratio()
}

/// Graphics microcode the guest declares via `HLXRenderSetMicrocode` (from its own build);
/// `consume_dl` hands it to fast3d so the renderer matches the ROM. 0 = F3dex2 (fast3d's
/// default), 1 = F3d — kept in sync with `HLXMicrocode` in runtime.h.
static MICROCODE: AtomicU32 = AtomicU32::new(0);

/// An FFI enum id had no match: the C header and this file's mapping have drifted. Panics in dev
/// (debug_assert) and logs once in release (where debug_assert is compiled out) so drift is never
/// silent; the caller falls back to a safe default.
fn warn_id_drift(what: &str, id: u32) {
    debug_assert!(false, "unknown {what} id {id} (runtime.h/render.rs drift)");
    static WARNED: AtomicBool = AtomicBool::new(false);
    if !WARNED.swap(true, Ordering::Relaxed) {
        log::error!("unknown {what} id {id}; using default (runtime.h/render.rs drift)");
    }
}

fn microcode() -> fast3d::Microcode {
    match MICROCODE.load(Ordering::Relaxed) {
        0 => fast3d::Microcode::F3dex2,
        1 => fast3d::Microcode::F3d,
        other => {
            warn_id_drift("microcode", other);
            fast3d::Microcode::F3dex2
        }
    }
}

/// C-facing microcode selector: the guest declares its build's microcode before the first gfx task.
#[no_mangle]
pub extern "C" fn HLXRenderSetMicrocode(microcode: u32) {
    MICROCODE.store(microcode, Ordering::Relaxed);
}

/// Guest vertex/matrix layout (fixed N64 s16 vs float), declared via `HLXRenderSetDataFormat`;
/// fast3d treats it as orthogonal to the microcode, so `consume_dl` applies it before each frame.
/// 0 = Fixed (fast3d's default), 1 = Float — kept in sync with `HLXDataFormat` in runtime.h.
static DATA_FORMAT: AtomicU32 = AtomicU32::new(0);

fn data_format() -> fast3d::DataFormat {
    match DATA_FORMAT.load(Ordering::Relaxed) {
        0 => fast3d::DataFormat::Fixed,
        1 => fast3d::DataFormat::Float,
        other => {
            warn_id_drift("data format", other);
            fast3d::DataFormat::Fixed
        }
    }
}

/// C-facing data-format selector: the guest declares its build's vertex layout before the first gfx task.
#[no_mangle]
pub extern "C" fn HLXRenderSetDataFormat(format: u32) {
    DATA_FORMAT.store(format, Ordering::Relaxed);
}

pub(crate) const SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// The surface config helix pins, matching fast3d `Renderer::new` for `Some(Bgra8Unorm)`. `with_device`
/// does not configure the surface, so the render thread calls `surface.configure` with this first.
pub(crate) fn surface_config(width: u32, height: u32) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: SURFACE_FORMAT,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    }
}

pub struct SurfaceHandoff {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub width: u32,
    pub height: u32,
}

struct CaptureSelection {
    directory: std::path::PathBuf,
    frames: std::collections::BTreeSet<u64>,
}

impl CaptureSelection {
    fn parse(
        directory: Option<std::ffi::OsString>,
        frames: Option<String>,
    ) -> Result<Option<Self>, String> {
        let Some(directory) = directory else {
            return Ok(None);
        };
        if directory.is_empty() {
            return Err("FAST3D_CAPTURE_DIR is empty".into());
        }
        let frames = frames.ok_or("FAST3D_CAPTURE_FRAMES is required with FAST3D_CAPTURE_DIR")?;
        let frames = frames
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<u64>()
                    .map_err(|_| format!("invalid capture frame serial: {s:?}"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Some(Self {
            directory: directory.into(),
            frames,
        }))
    }

    fn write(&self, fixture: &fast3d::capture::Fixture) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        let bytes = fixture.to_bytes()?;
        std::fs::create_dir_all(&self.directory)?;
        let path = self
            .directory
            .join(format!("frame-{:06}.f3dcap", fixture.frame.serial));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(&bytes)?;
        log::info!(
            "captured frame {} to {}",
            fixture.frame.serial,
            path.display()
        );
        Ok(())
    }
}

pub struct RenderContext {
    renderer: fast3d::Renderer,
    frame_serial: u64,
    capture_selection: Option<CaptureSelection>,
    capture_frame: Option<fast3d::capture::CaptureFrame>,
}

impl RenderContext {
    /// Build the Renderer on this thread from the handed-over surface. Replicates fast3d `new()`'s
    /// device setup verbatim (`with_device` adopts a caller-made device and does not configure the
    /// surface — spec §6). Panics here are caught by `spawn` (fatal-on-panic).
    pub fn from_handoff(h: SurfaceHandoff) -> RenderContext {
        let SurfaceHandoff {
            instance,
            surface,
            width,
            height,
        } = h;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .expect("helix render: request_adapter");

        let dual = adapter
            .features()
            .contains(wgpu::Features::DUAL_SOURCE_BLENDING);
        let required_features = if dual {
            wgpu::Features::DUAL_SOURCE_BLENDING
        } else {
            wgpu::Features::empty()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("fast3d-device"),
            required_features,
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }))
        .expect("helix render: request_device");

        let config = surface_config(width, height);
        surface.configure(&device, &config);

        let renderer = fast3d::Renderer::with_device(
            device,
            queue,
            fast3d::PresentTarget::Surface { surface, config },
            fast3d::RendererConfig {
                resolution_multiplier: 1,
                sample_count: 1,
                present_mode: wgpu::PresentMode::Fifo,
                format: Some(SURFACE_FORMAT),
                clear_policy: fast3d::ClearPolicy::Persist,
                power_preference: wgpu::PowerPreference::HighPerformance,
            },
        );
        let capture_selection = match CaptureSelection::parse(
            std::env::var_os("FAST3D_CAPTURE_DIR"),
            std::env::var("FAST3D_CAPTURE_FRAMES").ok(),
        ) {
            Ok(selection) => selection,
            Err(e) => {
                log::warn!("capture disabled: {e}");
                None
            }
        };
        RenderContext {
            renderer,
            frame_serial: 0,
            capture_selection,
            capture_frame: None,
        }
    }

    pub fn consume_dl(&mut self, data_ptr: usize) {
        let serial = self.frame_serial;
        self.frame_serial += 1;
        if self
            .capture_selection
            .as_ref()
            .is_some_and(|selection| selection.frames.contains(&serial))
        {
            let mut capture = fast3d::capture::CaptureFrame::begin(
                &mut self.renderer,
                serial,
                0,
                fast3d::capture::Provenance {
                    decomp_revision: std::env::var("FAST3D_CAPTURE_REVISION")
                        .unwrap_or_else(|_| "unknown".into()),
                    source_symbols: std::env::var("FAST3D_CAPTURE_SYMBOLS")
                        .unwrap_or_else(|_| "unknown (live task)".into()),
                    command_vector: format!("helix/frame/{serial}"),
                    synthetic_data: "none; live guest memory".into(),
                },
            );
            if let Err(e) = capture.process_dl(
                &mut self.renderer,
                &HelixHardware,
                data_ptr as u64,
                microcode(),
                data_format(),
                &mut DedupLogSink,
            ) {
                log::warn!("capture frame {serial}: {e}");
            }
            self.capture_frame = Some(capture);
        } else {
            self.renderer.set_data_format(data_format());
            self.renderer.begin_frame();
            let _ = self.renderer.process_dl(
                &HelixHardware,
                data_ptr as u64,
                microcode(),
                &mut DedupLogSink,
            );
        }
    }

    pub fn present(&mut self) {
        if let Some(capture) = self.capture_frame.take() {
            match capture.present(&mut self.renderer, &HelixHardware) {
                Ok(fixture) => {
                    if let Err(e) = self.capture_selection.as_ref().unwrap().write(&fixture) {
                        log::warn!("write capture frame {}: {e}", fixture.frame.serial);
                    }
                }
                Err(e) => log::warn!("capture present: {e}"),
            }
        } else {
            match self.renderer.present(&HelixHardware) {
                Ok(()) => {}
                Err(fast3d::PresentError::SurfaceLost) => {}
                Err(e) => log::warn!("present: {e:?}"),
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some((w, h)) = valid_surface_size(width, height) {
            self.renderer.resize(w, h);
        }
    }
}

/// Consume/present/resize seam so the loop's dispatch is unit-testable without a GPU.
pub(crate) trait GfxConsumer {
    fn consume_dl(&mut self, data_ptr: usize);
    fn present(&mut self);
    fn resize(&mut self, width: u32, height: u32);
}

impl GfxConsumer for RenderContext {
    fn consume_dl(&mut self, data_ptr: usize) {
        RenderContext::consume_dl(self, data_ptr)
    }
    fn present(&mut self) {
        RenderContext::present(self)
    }
    fn resize(&mut self, width: u32, height: u32) {
        RenderContext::resize(self, width, height)
    }
}

/// The render loop over the consumer seam. Exits on Shutdown or a dropped sender.
pub(crate) fn render_loop_on(ctx: &mut impl GfxConsumer, rx: Receiver<RenderMsg>) {
    while let Ok(msg) = rx.recv() {
        match msg {
            RenderMsg::Gfx { data_ptr, done } => {
                ctx.consume_dl(data_ptr);
                let _ = done.send(()); // release the blocked guest → it posts SP then DP
                ctx.present();
            }
            RenderMsg::Resize { width, height } => ctx.resize(width, height),
            RenderMsg::Shutdown => break,
        }
    }
}

pub struct RenderHandle {
    join: Option<JoinHandle<()>>,
}

impl RenderHandle {
    /// Join the render thread. Call only after `RenderMsg::Shutdown` was sent (else it never exits).
    pub fn join(mut self) {
        if let Some(j) = self.join.take() {
            let _ = j.join(); // panics are caught inside the thread (see `spawn`)
        }
    }
}

/// Spawn the render thread. The Renderer is `!Send` — constructed and owned here, never moved. Any
/// failure (adapter/device init or a frame) is caught, logged, and trips shutdown so main stops
/// waiting and the process exits cleanly instead of running blind.
pub fn spawn(handoff: SurfaceHandoff) -> RenderHandle {
    let rx = take_render_receiver();
    let join = std::thread::Builder::new()
        .name("helix-render".into())
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut ctx = RenderContext::from_handoff(handoff);
                render_loop_on(&mut ctx, rx);
            }));
            if outcome.is_err() {
                log::error!("helix render thread failed; shutting down");
                crate::ultra::request_shutdown();
            }
        })
        .expect("spawn helix render thread");
    RenderHandle { join: Some(join) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_selection_requires_explicit_valid_serials() {
        assert!(CaptureSelection::parse(None, Some("broken".into()))
            .unwrap()
            .is_none());
        let selection = CaptureSelection::parse(Some("frames".into()), Some("0, 5,19,5".into()))
            .unwrap()
            .unwrap();
        assert_eq!(
            selection.frames.into_iter().collect::<Vec<_>>(),
            vec![0, 5, 19]
        );
        for frames in [
            None,
            Some(""),
            Some("1,"),
            Some("-1"),
            Some("1,abc"),
            Some("18446744073709551616"),
        ] {
            assert!(
                CaptureSelection::parse(Some("frames".into()), frames.map(str::to_owned)).is_err()
            );
        }
        assert!(CaptureSelection::parse(Some("".into()), Some("0".into())).is_err());
    }

    #[test]
    fn valid_surface_size_rejects_sentinels_and_zeros() {
        assert_eq!(valid_surface_size(1280, 720), Some((1280, 720)));
        assert_eq!(valid_surface_size(0, 720), None, "zero width rejected");
        assert_eq!(valid_surface_size(1280, 0), None, "zero height rejected");
        assert_eq!(valid_surface_size(u32::MAX, 720), None);
        assert_eq!(valid_surface_size(1280, u32::MAX), None);
    }

    #[test]
    fn aspect_expand_clamps_to_native_and_widens() {
        set_aspect_from_size(800, 600);
        assert!(
            (aspect_ratio() - NATIVE_ASPECT).abs() < 1e-6,
            "4:3 stays 4:3"
        );
        set_aspect_from_size(1920, 1080);
        assert!((aspect_ratio() - 16.0 / 9.0).abs() < 1e-6, "16:9 widens");
        set_aspect_from_size(600, 800);
        assert!(
            (aspect_ratio() - NATIVE_ASPECT).abs() < 1e-6,
            "portrait never narrower than 4:3"
        );
        set_aspect_from_size(u32::MAX, u32::MAX);
        assert!(
            (aspect_ratio() - NATIVE_ASPECT).abs() < 1e-6,
            "winit sentinel → native, no garbage"
        );
    }

    #[test]
    fn surface_config_matches_fast3d_new() {
        let c = surface_config(1280, 720);
        assert_eq!(c.format, wgpu::TextureFormat::Bgra8Unorm);
        assert_eq!(c.present_mode, wgpu::PresentMode::Fifo);
        assert_eq!(c.usage, wgpu::TextureUsages::RENDER_ATTACHMENT);
        assert_eq!(c.alpha_mode, wgpu::CompositeAlphaMode::Auto);
        assert_eq!(c.desired_maximum_frame_latency, 2);
        assert!(c.view_formats.is_empty());
        assert_eq!((c.width, c.height), (1280, 720));
        assert_eq!(
            (surface_config(0, 0).width, surface_config(0, 0).height),
            (1, 1)
        );
    }

    use crate::ultra::rcp::RenderMsg;
    use std::sync::mpsc::{channel, Receiver};
    use std::time::Duration;

    struct FakeConsumer {
        log: Vec<String>,
        done_before_present: Receiver<()>,
    }
    impl super::GfxConsumer for FakeConsumer {
        fn consume_dl(&mut self, data_ptr: usize) {
            self.log.push(format!("consume:{data_ptr:#x}"));
        }
        fn present(&mut self) {
            assert!(
                self.done_before_present.try_recv().is_ok(),
                "done must fire before present"
            );
            self.log.push("present".into());
        }
        fn resize(&mut self, w: u32, h: u32) {
            self.log.push(format!("resize:{w}x{h}"));
        }
    }

    #[test]
    fn render_loop_dispatch_and_done_before_present() {
        let (tx_done, rx_done) = channel::<()>();
        let (tx, rx) = channel::<RenderMsg>();
        tx.send(RenderMsg::Gfx {
            data_ptr: 0xABC,
            done: tx_done,
        })
        .unwrap();
        tx.send(RenderMsg::Resize {
            width: 640,
            height: 480,
        })
        .unwrap();
        tx.send(RenderMsg::Shutdown).unwrap();
        drop(tx);
        let mut fake = FakeConsumer {
            log: vec![],
            done_before_present: rx_done,
        };
        super::render_loop_on(&mut fake, rx);
        assert_eq!(fake.log, vec!["consume:0xabc", "present", "resize:640x480"]);
    }

    #[test]
    fn shutdown_exits_the_loop_without_hanging() {
        let (tx, rx) = channel::<RenderMsg>();
        let (_td, rx_done) = channel::<()>();
        let (tx_started, rx_started) = channel::<()>();
        let (tx_exited, rx_exited) = channel::<()>();
        let worker = std::thread::spawn(move || {
            tx_started.send(()).unwrap(); // proves the worker entered before we send Shutdown
            let mut fake = FakeConsumer {
                log: vec![],
                done_before_present: rx_done,
            };
            super::render_loop_on(&mut fake, rx);
            tx_exited.send(()).unwrap();
        });
        rx_started.recv().unwrap();
        tx.send(RenderMsg::Shutdown).unwrap();
        rx_exited
            .recv_timeout(Duration::from_secs(5))
            .expect("Shutdown must exit the loop; a regression fails here instead of hanging");
        worker.join().unwrap();
    }
}
