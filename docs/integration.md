# Integration

## C/C++ Project 
If you're working on a C or C++ project the recommended way of integrating Helix into your project is using CMake. You can easily build and link this library by using [Corrosion](https://github.com/dcvz/corrosion/tree/helix), a tool that integrate Rust libraries into C/C++ projects. You'll want to:

1. Add this library as a submodule (or copy it) into your project
2. Enable Corrosion using FetchContent (see their README)
3. Include the features you want to be built using `FEATURES`. **audio** is always included.
4. Link against the helix library
5. Include the include folder of helix so your program can see the available methods

__NOTE: If you get a build error that certain header methods are not implemented, you're likely not building with that feature enabled.__

Example:
```cpp
corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../helix/Cargo.toml FEATURES network)
# methods in these headers will work:
# include <helix/network.h>, <helix/audio.h>
# methods in these headers won't work:
# include <helix/speech.h>
```

## Rust Project
If you're working on a Rust project, you can add Helix via `cargo add helix`.

## Dependencies Required

### Linux 
- libasound2-dev
- fontconfig, freetype, x11
- speech-dispatcher (optional - feature: speech)
