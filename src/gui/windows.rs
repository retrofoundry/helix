use imgui::{CollapsingHeader, TreeNode, Ui};

use crate::fast3d::gbi::utils::{geometry_mode_uses_fog, geometry_mode_uses_lighting};

#[cfg(feature = "glium")]
use super::gui_glium::Gui;

#[cfg(feature = "wgpu")]
use super::gui_wgpu::Gui;

pub trait HelixWindows {
    fn show_profiler_window(&self, opened: &mut bool, gui: &mut Gui);
}

impl HelixWindows for Ui {
    fn show_profiler_window(&self, opened: &mut bool, gui: &mut Gui) {
        self.window("Profiler")
            .opened(opened)
            .always_auto_resize(true)
            .build(|| {
                self.text(format!("Helix {}", env!("CARGO_PKG_VERSION")));

                self.text(format!(
                    "Application average {:.3} ms/frame ({:.1} FPS)",
                    1000.0 / self.io().framerate,
                    self.io().framerate
                ));

                let total_tris = gui
                    .intermediate_graphics_device
                    .draw_calls
                    .iter()
                    .fold(0, |acc, draw_call| acc + draw_call.vbo.num_tris);

                self.text(format!(
                    "{} draw calls, {} vertices ({} triangles)",
                    gui.intermediate_graphics_device.draw_calls.len(),
                    total_tris * 3,
                    total_tris,
                ));

                self.separator();

                if CollapsingHeader::new("Draw Calls").build(self) {
                    self.indent();
                    for (i, dc) in gui
                        .intermediate_graphics_device
                        .draw_calls
                        .iter()
                        .enumerate()
                    {
                        TreeNode::new(format!("Draw Call: {}", i), self).build(|| {
                            self.text(format!("Viewport: {}", dc.viewport));
                            self.text(format!("Scissor: {:?}", dc.scissor));
                            self.text(format!(
                                "Fog: Mul {} Offset {}",
                                dc.fog.multiplier, dc.fog.offset
                            ));
                            self.text(format!("Cull Mode: {:?}", dc.cull_mode));

                            TreeNode::new("Geometry Mode", self).build(|| {
                                self.text(format!(
                                    "Lighting: {}",
                                    geometry_mode_uses_lighting(dc.geometry_mode)
                                ));
                                self.text(format!(
                                    "Fog: {}",
                                    geometry_mode_uses_fog(dc.geometry_mode)
                                ));
                            });

                            TreeNode::new("Uniforms", self).build(|| {
                                self.text("Blend");
                                self.indent();
                                self.text(format!(
                                    "Blend Color: {}",
                                    dc.uniforms.blend.blend_color
                                ));
                                self.text(format!("Fog Color: {}", dc.uniforms.blend.fog_color));
                                self.unindent();
                                self.text("Combine");
                                self.indent();
                                self.text(format!(
                                    "Prim Color: {}",
                                    dc.uniforms.combine.prim_color
                                ));
                                self.text(format!("Env Color: {}", dc.uniforms.combine.env_color));
                                self.text(format!(
                                    "Key Center: {}",
                                    dc.uniforms.combine.key_center
                                ));
                                self.text(format!("Key Scale: {}", dc.uniforms.combine.key_scale));
                                self.text(format!(
                                    "Prim LOD (Frac/Min): {}",
                                    dc.uniforms.combine.prim_lod
                                ));
                                self.text(format!(
                                    "Convert K4: {}",
                                    dc.uniforms.combine.convert_k4
                                ));
                                self.text(format!(
                                    "Convert K5: {}",
                                    dc.uniforms.combine.convert_k5
                                ));
                                self.unindent();
                            });
                        });
                    }
                    self.unindent();
                }
            });
    }
}

// MARK: - C API

#[no_mangle]
pub extern "C" fn GUIShowProfilerWindow(ui: &Ui, gui: Option<&mut Gui>, opened: Option<&mut bool>) {
    let opened = opened.unwrap();
    let gui = gui.unwrap();
    if !*opened {
        return;
    }

    ui.show_profiler_window(opened as &mut bool, gui);
}
