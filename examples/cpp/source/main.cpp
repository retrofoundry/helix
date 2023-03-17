#include <iostream>
#include <string>
#include <thread>

#include <helix/speech.h>
#include <libultra/os.h>

auto main() -> int
{
  auto const message = "Hello, world!";
  std::cout << message << '\n';

  auto controllHub = HLXCreateControllerHub();
  __osControlHubInstance = controllHub;

  osContInit(nullptr, nullptr, nullptr);

#if defined(__APPLE__) || defined(__WIN32)
  auto speechSynthesizer = HLXSpeechSynthesizerCreate();
  HLXSpeechSynthesizerInit(speechSynthesizer);
  HLXSpeechSynthesizerSetVolume(speechSynthesizer, 1.0);
  HLXSpeechSynthesizerSpeak(speechSynthesizer, "Hello, world!", true);
#endif

  // Wait for the speech to finish.
  std::this_thread::sleep_for(std::chrono::milliseconds(2000));

  return 0;
}
