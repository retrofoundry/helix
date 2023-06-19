# Integration

## C/C++ Project 
If you're working on a C or C++ project the recommended way of integrating Helix into your project is using CMake. You can easily build and link this library by using [Corrosion](https://github.com/corrosion-rs/corrosion), a tool that integrate Rust libraries into C/C++ projects. You'll want to:

1. Add this library as a submodule (or copy it) into your project
2. Enable Corrosion using FetchContent (see their README)
3. Include the features you want to be built using `FEATURES`. **audio** is always included.
4. Link against the helix library
5. Include the include folder of helix so your program can see the available methods
6. Include the cpp folder into your compilation (this is our C++ API wrapper providing a nicer API for the C language)

__NOTE: If you get a build error that certain header methods are not implemented, you're likely not building with that feature enabled.__

Example:
```cpp
corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../helix/Cargo.toml FEATURES f3dex2e network)
# methods in these headers will work:
# include <helix/network.h>, <helix/audio.h>
# methods in these headers won't work:
# include <helix/speech.h>
```

Advanced Example:
```cmake
#==============================================================================#
# Helix Integration                                                            #
#==============================================================================#

add_subdirectory(./corrosion)

corrosion_import_crate(MANIFEST_PATH ./helix/Cargo.toml FEATURES f3dex2e)

target_link_libraries(sm64.${VERSION} PRIVATE helix)
target_include_directories(sm64.${VERSION} PRIVATE ./helix/include)

if(APPLE)
    target_link_libraries(helix INTERFACE "-framework OpenGL")
    target_link_libraries(helix INTERFACE "-framework Foundation")
    target_link_libraries(helix INTERFACE "-framework CoreFoundation -framework AVFoundation -framework CoreAudio -framework AudioToolbox")
    target_link_libraries(helix INTERFACE "-framework Metal -framework QuartzCore")
    target_link_libraries(helix INTERFACE "-framework IOKit")
    target_link_libraries(helix INTERFACE "-framework ApplicationServices")
    target_link_libraries(helix INTERFACE "-framework AppKit")
    
    target_link_libraries(helix INTERFACE "-lc++")
elseif(WIN32)
    target_link_libraries(helix INTERFACE OpenGL32)
    target_link_libraries(helix INTERFACE D3DCompiler)
    target_link_libraries(helix INTERFACE imm32)
    target_link_libraries(helix INTERFACE winmm)
    target_link_libraries(helix INTERFACE uxtheme)
    target_link_libraries(helix INTERFACE dwmapi)
    if(MINGW)
      target_link_libraries(helix INTERFACE "-lstdc++")
    endif()
else()
    find_package(X11 REQUIRED)
    find_package(libUsb REQUIRED)
    find_package(ALSA REQUIRED)
    find_package(PulseAudio REQUIRED)
    find_package(Freetype REQUIRED)
    find_package(udev REQUIRED)
    target_link_libraries(helix INTERFACE ${ALSA_LIBRARY})
    target_link_libraries(helix INTERFACE ${OPENGL_glx_LIBRARY} ${OPENGL_opengl_LIBRARY})
    target_link_libraries(helix INTERFACE ${X11_LIBRARIES})
    target_link_libraries(helix INTERFACE ${UDEV_LIBRARY})
    target_link_libraries(helix INTERFACE ${FREETYPE_LIBRARIES})
    target_link_libraries(helix INTERFACE Fontconfig::Fontconfig)
    target_link_libraries(helix INTERFACE libUsb::libUsb)

    target_link_libraries(helix INTERFACE "-lstdc++")
endif()
```

## Rust Project
If you're working on a Rust project, you can add Helix via `cargo add helix`.

## Dependencies Required

### Linux 
- libasound2-dev
- fontconfig, freetype, x11
- speech-dispatcher (optional - feature: speech)
