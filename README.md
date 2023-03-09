# helix

Provides useful utilities for interfacing with various OS systems. Aimed at N64 software written in C and C++.

## Features Provided
- [x] Speech Synthesis
- [x] Audio Playback

## How to install?

### CMake
If you're using CMake you can install and link this library easily by using [Corrosion](), a tool to integrate Rust libraries into a project using CMake. You'll want to:

1. Add this library as a submodule (or copy it) into your project
2. Enable Corrosion using FetchContent (see their README)
3. Link against the helix library
4. Include the include folder of helix so your program can see the available methods

