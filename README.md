# Helix

Helix is a Rust library for running N64 software on PC and other systems. It provides a faithful libultra runtime alongside a modern renderer, audio, and input, so a decompilation runs on its original threading and `main.c` with minimal change.

## Features Provided

- [x] Libultra runtime (ultra) — real threads, message queues + events, a 60 Hz VI clock, an RCP task engine (graphics → fast3d, audio → an HLE Acmd interpreter), PI/DMA, timers, and EEPROM save, all over native pointers
- [x] Window Management (gui)
- [x] N64 RDP command processing and rendering via [fast3d-rs](https://github.com/retrofoundry/fast3d-rs) on wgpu (gui)
- [x] Input Handling (gamepad)
- [x] Audio Rendering via [arie](https://github.com/retrofoundry/arie) (audio)

### Optional Features
- [x] Speech Synthesis (speech)
- [x] TCP Stream (network)

For details on each of the features provided please see our [documentation](https://retrofoundry.github.io/helix/dev/).

## How to setup?

For setup and more in-depth information please see our [documentation](https://retrofoundry.github.io/helix/dev/).

## Community

[![](https://dcbadge.vercel.app/api/server/nGckYNTp4w)](https://discord.gg/nGckYNTp4w)
