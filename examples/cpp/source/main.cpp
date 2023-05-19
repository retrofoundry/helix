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

  auto event_loop = GUICreateEventLoop();

  auto gui = GUICreate("Helix Example", event_loop, &draw_menu_bar);
  
  while (true) {
    GUIStartFrame(gui, event_loop);
    GUIDrawListsDummy(gui);
    GUIEndFrame(gui);
  }

  return 0;
}
