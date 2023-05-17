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

corrosion_import_crate(MANIFEST_PATH ../../Cargo.toml FEATURES speech network)

# ---- Declare executable ----

add_compile_definitions(_LANGUAGE_C_PLUS_PLUS=1)

add_executable(cpp_exe source/main.cpp)
add_executable(cpp::exe ALIAS cpp_exe)

set_property(TARGET cpp_exe PROPERTY OUTPUT_NAME cpp)

target_compile_features(cpp_exe PRIVATE cxx_std_17)

target_link_libraries(cpp_exe PRIVATE helix)
target_include_directories(cpp_exe PRIVATE ../../include)

if(APPLE)
    find_package(SDL2 REQUIRED)
    target_link_libraries(helix INTERFACE SDL2::SDL2)
    target_link_libraries(helix INTERFACE "-framework Foundation")
    target_link_libraries(helix INTERFACE "-framework CoreFoundation -framework AVFoundation -framework CoreAudio -framework AudioToolbox")
    target_link_libraries(helix INTERFACE "-framework Metal -framework QuartzCore")
    target_link_libraries(helix INTERFACE "-framework IOKit")
    target_link_libraries(helix INTERFACE "-framework ApplicationServices")
elseif(WIN32)
    find_package(SDL2 CONFIG REQUIRED)
    target_link_libraries(helix INTERFACE ntdll)
    target_link_libraries(helix INTERFACE
        $<TARGET_NAME_IF_EXISTS:SDL2::SDL2main>
        $<IF:$<TARGET_EXISTS:SDL2::SDL2>,SDL2::SDL2,SDL2::SDL2-static>
    )
    target_link_libraries(helix INTERFACE D3DCompiler)
else()
    find_package(SDL2 REQUIRED)
    target_link_libraries(helix INTERFACE SDL2::SDL2)
    target_link_libraries(helix INTERFACE asound speechd udev)
endif()


# ---- Install rules ----

if(NOT CMAKE_SKIP_INSTALL_RULES)
  include(cmake/install-rules.cmake)
endif()
