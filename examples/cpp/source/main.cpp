#include <iostream>
#include <string>
#include <thread>

#include <helix/helix.h>

auto main() -> int
{
  auto const message = "Hello, world!";
  std::cout << message << '\n';

  HLXSpeechSynthesizerInit();
  HLXSpeechSynthesizerSetVolume(1.0);
  HLXSpeechSynthesizerSpeak("Hello, world!", true);

  // Wait for the speech to finish.
  std::this_thread::sleep_for(std::chrono::milliseconds(2000));

  return 0;
}
