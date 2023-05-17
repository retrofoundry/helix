#include <iostream>
#include <string>
#include <thread>

#include <helix/helix.h>
#include <helix/gui.h>
#include <helix/gamepad.h>
#include <imgui/imgui.h>

void draw_menu_bar() {
  if (ImGui::BeginMenu("File")) {
    if (ImGui::MenuItem("Quit", "Ctrl+Q")) {}
    ImGui::EndMenu();
  }

  ImGui::Separator();

  if (ImGui::BeginMenu("Edit")) {
    ImGui::EndMenu();
  }
}

auto main() -> int
{
  HelixInit();

  auto gamepad_manager = GamepadManagerCreate();
  uint8_t controller_bits = 0;
  GamepadManagerInit(gamepad_manager, &controller_bits);

  auto gui = GUICreate("Helix Example", &draw_menu_bar);

  auto event_loop_thread = std::thread([] {
    while (true) {
      std::cout << "Hello World!" << std::endl;
      std::this_thread::sleep_for(std::chrono::seconds(1));
    }
  });
  
  while (true) {
    GUIStartFrame(gui);
    GUIDrawListsDummy(gui);
    GUIEndFrame(gui);
  }

  event_loop_thread.join();

  return 0;
}
