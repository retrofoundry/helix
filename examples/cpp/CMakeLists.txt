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
    GIT_REPOSITORY https://github.com/dcvz/corrosion.git
    GIT_TAG 1f8fc7733b7a174a249e423e101e0914dc4f9f61 # Optionally specify a commit hash, version tag or branch here
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(MANIFEST_PATH ../../Cargo.toml FEATURES cpp speech network)

# ---- Declare executable ----

add_executable(cpp_exe source/main.cpp)
add_executable(cpp::exe ALIAS cpp_exe)

set_property(TARGET cpp_exe PROPERTY OUTPUT_NAME cpp)

target_compile_features(cpp_exe PRIVATE cxx_std_17)

target_link_libraries(cpp_exe PRIVATE helix)
target_include_directories(cpp_exe PRIVATE ../../include)

# ---- Install rules ----

if(NOT CMAKE_SKIP_INSTALL_RULES)
  include(cmake/install-rules.cmake)
endif()
