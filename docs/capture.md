# fast3d capture

Set capture variables before launching the game. Helix must be built against a
fast3d revision with `CaptureSequence` and the native capture API. The dependency
enables fast3d's `capture` feature. A statically linked game must be relinked after
rebuilding Helix.

| Variable | Meaning |
| --- | --- |
| `FAST3D_CAPTURE_DIR` | Directory for individual frame fixtures. Requires `FAST3D_CAPTURE_FRAMES`. |
| `FAST3D_CAPTURE_FRAMES` | Comma-separated renderer serials for individual fixtures, starting at 1. |
| `FAST3D_CAPTURE_SEQUENCE` | Output file for one sequence, for example `/tmp/startup.f3dcap`. Mutually exclusive with `FAST3D_CAPTURE_DIR`. |
| `FAST3D_CAPTURE_WARMUP_FRAMES` | Number of initial sequence frames before observation; defaults to 0. These frames remain in the recording. |
| `FAST3D_CAPTURE_PRESENTATIONS` | Required sequence presentation serials, comma-separated, all greater than the warm-up count. |
| `FAST3D_CAPTURE_REVISION` | Decomp revision in each frame's provenance; defaults to `unknown`. |
| `FAST3D_CAPTURE_SYMBOLS` | Source symbols or scene annotations in provenance; defaults to `unknown (live task)`. |

Lists accept surrounding whitespace, duplicate serials, and empty comma-separated
elements, including the trailing comma produced by BSD `seq -s,`. Empty lists,
zero, invalid numbers, and invalid combinations cause render-thread failure and
runtime shutdown. Existing capture files are never overwritten. Auxiliary variables
alone do not enable capture.

## Individual frames

```sh
FAST3D_CAPTURE_DIR=/tmp/sm64-frames \
FAST3D_CAPTURE_FRAMES=120,240,360, \
FAST3D_CAPTURE_REVISION="$(git rev-parse HEAD)" \
FAST3D_CAPTURE_SYMBOLS='live sm64 graphics tasks' \
RUST_LOG=helix::render=info \
./build-cmake/sm64-us
```

This writes `frame-000120.f3dcap`, `frame-000240.f3dcap`, and
`frame-000360.f3dcap`, named from `fixture.frame.serial`. Serial 1 is the first
graphics task, including startup before gameplay. PR #33 used zero-based consume
indices, so its selection 120 wrote serial 121; this hook selects the renderer
serial directly. Add one to old selections to capture the same tasks.

Standalone fixtures still require self-contained frame state. A frame that depends
on prior RDP/TMEM or framebuffer history can fail standalone capture or replay;
use a sequence for those workloads. Capture failures log an error and request
shutdown. A successful write logs `captured frame ...`.

## Sequences

```sh
FAST3D_CAPTURE_SEQUENCE=/tmp/sm64-startup.f3dcap \
FAST3D_CAPTURE_WARMUP_FRAMES=120 \
FAST3D_CAPTURE_PRESENTATIONS=121,180,240, \
FAST3D_CAPTURE_REVISION="$(git rev-parse HEAD)" \
FAST3D_CAPTURE_SYMBOLS='startup through selected gameplay route' \
RUST_LOG=helix::render=info \
./build-cmake/sm64-us
```

The sequence resets the renderer before the first task and records every frame
through serial 240. It retains frames 1–120 as warm-up and selects 121, 180, and
240 for replay images. Serials are never cropped or renumbered. Each Helix graphics
task is one frame. The dither seed is 0 and depth reset policy is `Never`.

After serial 240 is presented, `CaptureSequence::finish` validates the recording,
the file is written, and recording stops. The game continues running. Look for
`capture sequence started: ... frames 1..=240` and
`capture sequence complete: wrote 240 frames to ...` in the log. The completed
file uses the version-two sequence container, not a standalone frame payload.

Sequence startup also logs `capture sequence context initialization #1` with the
process ID. A second sequence context in the same process logs its attempt number
and fails before creating a renderer or recording. It cannot restart at serial 1
and compete for the same output. Helix's current GUI starts one render context;
resuming an existing window does not recreate it. Restart the process for another
sequence, even after the first finishes.

Guest memory is copied inside `consume_dl`, before the blocked guest is released.
All sequence renderer operations use the capture wrapper. Presentation uses
`present_last`, without VI registers or guest-memory reads. Renderer resizes are
deferred until recording finishes to keep one extent and avoid invalidating the
sequence. Avoid resizing or minimizing during a recording: a surface presentation
failure stops capture and attempts to save the earlier completed prefix.

Without a capture output variable, Helix retains the normal renderer operations
and presentation error handling and installs no signal handler.

## Ending a run early

Sequence mode attempts to install a `ctrlc` handler with termination support once
per process, including when installation fails. Success logs
`capture shutdown handler installed`. With that handler, Ctrl-C, SIGTERM
(`kill -TERM <game-pid>`), SIGHUP, and normal window close reach Helix's shutdown
path, which joins the render thread after it finishes and writes completed frames.
Allow shutdown to finish; a second forced kill can interrupt serialization.
Failure logs `capture shutdown handler unavailable` and warns that a killed run
will not save an unfinished capture. Recording continues: endpoint and normal
window-close saves remain enabled. The sequence-start message alone does not
promise signal save. `ctrlc::try_set_handler` also rejects existing native signal
handlers and ignored signals, so its "already registered" error does not prove
Helix initialized twice. Existing signal dispositions are not deliberately
overwritten.

For a prefix ending at serial N, requested presentations greater than N are
removed. If none were reached, N becomes the sole presentation. Warm-up becomes
`min(requested_warmup, N - 1)`. Every frame 1..N remains present. The log says
`capture sequence incomplete` with the actual counts and selection, followed by
`capture sequence partial: wrote N frames ...`. This is a valid partial recording,
not evidence that the requested range completed. If no frame completed, there is
no valid sequence to write; the error says so.

SIGKILL, process abort, power loss, or a render-thread panic cannot reliably save
an unfinished recording. `CaptureSequence` keeps frames in memory and only exposes
them through consuming `finish`; it has no snapshot/checkpoint API. This hook saves
on catchable termination and at the requested endpoint. It does not install unsafe
signal handlers or claim crash recovery. Memory grows with the retained prefix,
and serialization needs additional memory.

Writes use a unique `<output>.<pid>-<id>.partial` file in the output directory,
sync its bytes, then atomically publish the final name with a hard link. The
filesystem must support hard links. The final path never exposes a half-written
container. Empty staging files are removed on graceful abandonment; crashes can
leave them behind without blocking retries. Failed writes/publications can leave
nonempty staging files; the error identifies the path. Validate such a file with
the replay tool before using it. A `.partial` file alone is not a successful capture.

## Replay and validation

From the fast3d worktree, with its supported toolchain:

```sh
mkdir -p /tmp/sm64-replay
cargo run -p fast3d --features capture --example replay_capture -- \
  /tmp/sm64-startup.f3dcap /tmp/sm64-replay/startup all
```

The replay loader rejects gaps, missing reset history, invalid warm-up/presentation
metadata, and malformed/truncated files. The `all` mode replays depth policies
`persist`, `cimg`, `task`, and `frame`; it writes per-policy logs and selected
serial PNG/RGBA8 files, plus comparisons against persistence. For example, expect
`startup-persist-000121.png`, `startup-persist-000180.png`,
`startup-persist-000240.png`, and `startup-persist.log`.

Check that the log lists every original frame 1..240, warm-up 120, the three
requested presentations, expected provenance, one native task per frame, and no
missing-memory errors. Review the images against the live route. Container
validation alone does not establish visual correctness.
