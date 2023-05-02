#include <iostream>
#include <string>
#include <thread>

#include <helix/gui.h>
#include <imgui/imgui.h>

static bool show_app_metrics = true;

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