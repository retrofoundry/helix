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

void draw_main() {
  if (show_app_metrics)
    ImGui::ShowMetricsWindow(&show_app_metrics);
}

auto main() -> int
{
  auto event_loop = GUICreateEventLoop();
  auto gui = GUICreate("Helix Example", event_loop, &draw_menu_bar, &draw_main);

  auto event_loop_thread = std::thread([] {
    while (true) {
      std::cout << "Hello World!" << std::endl;
      std::this_thread::sleep_for(std::chrono::seconds(1));
    }
  });

  GUIStart(event_loop, gui);
  event_loop_thread.join();

  return 0;
}
