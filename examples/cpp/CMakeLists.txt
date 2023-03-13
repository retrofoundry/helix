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

# Import targets defined in a package or workspace manifest `Cargo.toml` file
corrosion_import_crate(MANIFEST_PATH ../../Cargo.toml)

# include headers from the imported crate
target_include_directories(helix INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/../../include)

# ---- Declare executable ----

add_executable(cpp_exe source/main.cpp)
add_executable(cpp::exe ALIAS cpp_exe)

set_property(TARGET cpp_exe PROPERTY OUTPUT_NAME cpp)

target_compile_features(cpp_exe PRIVATE cxx_std_17)

target_link_libraries(cpp_exe PRIVATE helix)

if(APPLE)
    target_link_libraries(cpp_exe PRIVATE "-framework Foundation")
    target_link_libraries(cpp_exe PRIVATE "-framework AVFoundation")
    target_link_libraries(cpp_exe PRIVATE "-framework CoreFoundation")
    target_link_libraries(cpp_exe PRIVATE "-framework CoreAudio")
    target_link_libraries(cpp_exe PRIVATE "-framework AudioToolbox")
endif()

if(UNIX AND NOT APPLE)
  find_package(ALSA REQUIRED)
  target_link_libraries(cpp_exe PRIVATE ALSA::ALSA)
endif()

if(WIN32)
    target_link_libraries(cpp_exe PRIVATE ntdll)
endif()

# ---- Install rules ----

if(NOT CMAKE_SKIP_INSTALL_RULES)
  include(cmake/install-rules.cmake)
endif()

