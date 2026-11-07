# reality64

reality64 is a Rust library for running N64 software on PC and other systems. It provides a faithful libultra runtime alongside a modern renderer, audio, and input, so a decompilation runs on its original threading and `main.c` with minimal change.

## Features Provided

- [x] Libultra runtime (ultra) — real threads, message queues + events, a 60 Hz VI clock, an RCP task engine (graphics → ultragfx, audio → the Rust classic US aspMain handler), PI/DMA, timers, and EEPROM save, all over native pointers
- [x] Window Management (gui)
- [x] N64 RDP command processing and rendering via [ultragfx](https://github.com/retrofoundry/ultragfx) on wgpu (gui)
- [x] Input Handling (gamepad)
- [x] Audio Rendering via [arie](https://github.com/retrofoundry/arie) (audio)

### Optional Features
- [x] Speech Synthesis (speech)
- [x] TCP Stream (network)

For details on each of the features provided please see our [documentation](https://retrofoundry.github.io/reality64/dev/).

## How to setup?

For setup and more in-depth information please see our [documentation](https://retrofoundry.github.io/reality64/dev/).

For display-list fixtures and continuous recordings, see [ultragfx capture](docs/capture.md).

## Community

[![](https://dcbadge.vercel.app/api/server/nGckYNTp4w)](https://discord.gg/nGckYNTp4w)
