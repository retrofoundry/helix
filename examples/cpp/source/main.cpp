#include <iostream>
#include <string>
#include <thread>

#include <helix/speech.h>
#include <helix/con

auto main() -> int
{
  auto const message = "Hello, world!";
  std::cout << message << '\n';

#if defined(__APPLE__) || defined(__WIN32)
  HLXSpeechSynthesizerInit();
  HLXSpeechSynthesizerSetVolume(1.0);
  HLXSpeechSynthesizerSpeak("Hello, world!", true);
#endif

  // Wait for the speech to finish.
  std::this_thread::sleep_for(std::chrono::milliseconds(2000));

  return 0;
}
