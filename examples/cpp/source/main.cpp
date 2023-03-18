#include <iostream>
#include <string>
#include <thread>

#include <helix/gui.h>
#include <cimgui/cimgui.h>

void draw_menu_bar() {
  if (igBeginMenu("File", true)) {
    igEndMenu();
  };

  igSeparator();

  if (igBeginMenu("Edit", true)) {
    igEndMenu();
  };
}

auto main() -> int
{
  auto event_loop = HLXGUICreateEventLoop();
  auto gui = HLXGUICreate("Helix Example", event_loop, &draw_menu_bar);

  auto event_loop_thread = std::thread([] {
    while (true) {
      std::cout << "Hello World!" << std::endl;
      std::this_thread::sleep_for(std::chrono::seconds(1));
    }
  });

  HLXGUIStart(event_loop, gui);
  event_loop_thread.join();

  return 0;
}
