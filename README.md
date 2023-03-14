# helix

Provides useful utilities for interfacing with various OS systems. Aimed at N64 software written in C and C++.

## Features Provided
- [x] Audio Playback (audio)
- [x] Speech Synthesis (speech)
- [x] TCP Stream (network)

## How to install?

### CMake
If you're using CMake you can install and link this library easily by using [Corrosion](), a tool to integrate Rust libraries into a project using CMake. You'll want to:

1. Add this library as a submodule (or copy it) into your project
2. Enable Corrosion using FetchContent (see their README)
3. Include the features you want to be built using `FEATURES`. **audio** is always included.
4. Link against the helix library
5. Include the include folder of helix so your program can see the available methods

_NOTE: If you get a build error that certain header methods are not implemented, you're likely not building with that feature enabled. _

Example:
```
corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../helix/Cargo.toml FEATURES network)
# these headers work:
# include <helix/network.h>, <helix/audio.h>
# these header won't work:
# include <helix/speech.h>
```
