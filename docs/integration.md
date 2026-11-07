# Integration

## C/C++ Project 
If you're working on a C or C++ project the recommended way of integrating reality64 into your project is using CMake. You can easily build and link this library by using [Corrosion](https://github.com/corrosion-rs/corrosion), a tool that integrate Rust libraries into C/C++ projects. You'll want to:

1. Add this library as a submodule (or copy it) into your project
2. Enable Corrosion using FetchContent (see their README)
3. Include the features you want to be built using `FEATURES`. **audio** is always included.
4. Link against the reality64 library
5. Include the include folder of reality64 so your program can see the available methods
6. Include the cpp folder into your compilation (this is our C++ API wrapper providing a nicer API for the C language)

__NOTE: If you get a build error that certain header methods are not implemented, you're likely not building with that feature enabled.__

Example:
```cpp
corrosion_import_crate(MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/../reality64/Cargo.toml FEATURES network)
# methods in these headers will work:
# include <reality64/network.h>, <reality64/audio.h>
# methods in these headers won't work:
# include <reality64/speech.h>
```

Advanced Example:
```cmake
#==============================================================================#
# reality64 Integration                                                            #
#==============================================================================#

add_subdirectory(./corrosion)

corrosion_import_crate(MANIFEST_PATH ./reality64/Cargo.toml)

target_link_libraries(sm64.${VERSION} PRIVATE reality64)
target_include_directories(sm64.${VERSION} PRIVATE ./reality64/include)

if(APPLE)
    target_link_libraries(reality64 INTERFACE "-framework OpenGL")
    target_link_libraries(reality64 INTERFACE "-framework Foundation")
    target_link_libraries(reality64 INTERFACE "-framework CoreFoundation -framework AVFoundation -framework CoreAudio -framework AudioToolbox")
    target_link_libraries(reality64 INTERFACE "-framework Metal -framework QuartzCore")
    target_link_libraries(reality64 INTERFACE "-framework IOKit")
    target_link_libraries(reality64 INTERFACE "-framework ApplicationServices")
    target_link_libraries(reality64 INTERFACE "-framework AppKit")
    
    target_link_libraries(reality64 INTERFACE "-lc++")
elseif(WIN32)
    target_link_libraries(reality64 INTERFACE OpenGL32)
    target_link_libraries(reality64 INTERFACE D3DCompiler)
    target_link_libraries(reality64 INTERFACE imm32)
    target_link_libraries(reality64 INTERFACE winmm)
    target_link_libraries(reality64 INTERFACE uxtheme)
    target_link_libraries(reality64 INTERFACE dwmapi)
    if(MINGW)
      target_link_libraries(reality64 INTERFACE "-lstdc++")
    endif()
else()
    find_package(X11 REQUIRED)
    find_package(libUsb REQUIRED)
    find_package(ALSA REQUIRED)
    find_package(PulseAudio REQUIRED)
    find_package(Freetype REQUIRED)
    find_package(udev REQUIRED)
    target_link_libraries(reality64 INTERFACE ${ALSA_LIBRARY})
    target_link_libraries(reality64 INTERFACE ${OPENGL_glx_LIBRARY} ${OPENGL_opengl_LIBRARY})
    target_link_libraries(reality64 INTERFACE ${X11_LIBRARIES})
    target_link_libraries(reality64 INTERFACE ${UDEV_LIBRARY})
    target_link_libraries(reality64 INTERFACE ${FREETYPE_LIBRARIES})
    target_link_libraries(reality64 INTERFACE Fontconfig::Fontconfig)
    target_link_libraries(reality64 INTERFACE libUsb::libUsb)

    target_link_libraries(reality64 INTERFACE "-lstdc++")
endif()
```

## Rust Project
If you're working on a Rust project, you can add reality64 via `cargo add reality64`.

## Dependencies Required

### Linux 
- libasound2-dev
- fontconfig, freetype, x11
- speech-dispatcher (optional - feature: speech)
