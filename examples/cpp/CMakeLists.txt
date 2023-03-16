cmake_minimum_required(VERSION 3.16)

include(cmake/prelude.cmake)

project(
    cpp
    VERSION 0.1.0
    DESCRIPTION "Short description"
    HOMEPAGE_URL "https://example.com/"
    LANGUAGES CXX
)

if(APPLE)
    enable_language(OBJCXX)
endif()

include(cmake/project-is-top-level.cmake)
include(cmake/variables.cmake)

# ---- Import Helix ----

include(FetchContent)

FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.3 # Optionally specify a commit hash, version tag or branch here
)
FetchContent_MakeAvailable(Corrosion)

if(UNIX AND NOT APPLE)
    corrosion_import_crate(MANIFEST_PATH ../../Cargo.toml FEATURES cpp network)
else()
    corrosion_import_crate(MANIFEST_PATH ../../Cargo.toml FEATURES cpp speech network)
endif()

# ---- Declare executable ----

add_executable(cpp_exe source/main.cpp)
add_executable(cpp::exe ALIAS cpp_exe)

set_property(TARGET cpp_exe PROPERTY OUTPUT_NAME cpp)

target_compile_features(cpp_exe PRIVATE cxx_std_17)

target_link_libraries(cpp_exe PRIVATE helix)
target_include_directories(cpp_exe PRIVATE ../../include)

if(APPLE)
    target_link_libraries(cpp_exe PRIVATE "-framework Foundation")
    target_link_libraries(cpp_exe PRIVATE "-framework AVFoundation")
    target_link_libraries(cpp_exe PRIVATE "-framework CoreFoundation")
    target_link_libraries(cpp_exe PRIVATE "-framework CoreAudio")
    target_link_libraries(cpp_exe PRIVATE "-framework AudioToolbox")
    target_link_libraries(cpp_exe PRIVATE "-framework CoreGraphics")
    target_link_libraries(cpp_exe PRIVATE "-framework AppKit")
    target_link_libraries(cpp_exe PRIVATE "-framework Metal")
    target_link_libraries(cpp_exe PRIVATE "-framework CoreVideo")
    target_link_libraries(cpp_exe PRIVATE "-framework OpenGL")
    target_link_libraries(cpp_exe PRIVATE "-framework QuartzCore")
endif()

if(WIN32)
    target_link_libraries(cpp_exe PRIVATE ntdll imm32 winmm d3d11 dxgi d3dcompiler uxtheme dwmapi)
endif()

if(UNIX AND NOT APPLE)
    target_link_libraries(cpp_exe PRIVATE X11 fontconfig freetype asound)
endif()

# ---- Install rules ----

if(NOT CMAKE_SKIP_INSTALL_RULES)
  include(cmake/install-rules.cmake)
endif()
