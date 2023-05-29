use imgui_glow_renderer::glow;

use crate::fast3d::gbi::utils::{
    other_mode_l_alpha_compare_dither, other_mode_l_alpha_compare_threshold,
    other_mode_l_uses_alpha, other_mode_l_uses_fog, other_mode_l_uses_texture_edge,
};

use crate::fast3d::utils::color_combiner::{
    CombineParams, ShaderInputMapping, ACMUX, CCMUX, SHADER,
};
use std::collections::HashMap;

#[derive(PartialEq, Eq)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

#[derive(Debug)]
pub struct OpenGLProgram {
    // Compiled program.
    pub preprocessed_vertex: String,
    pub preprocessed_frag: String,
    pub compiled_program: Option<glow::NativeProgram>,

    // inputs
    pub both: String,
    pub vertex: String,
    pub fragment: String,
    pub defines: HashMap<String, String>,

    // configurators
    other_mode_h: u32,
    other_mode_l: u32,
    combine: CombineParams,

    pub shader_input_mapping: ShaderInputMapping,
    pub num_floats: usize,
}

impl OpenGLProgram {
    // MARK: - Defines
    pub fn defines_changed(&mut self) {
        self.preprocessed_vertex = "".to_string();
        self.preprocessed_frag = "".to_string();
    }

    pub fn set_define_string(&mut self, name: String, v: Option<String>) -> bool {
        if let Some(v) = v {
            if self.defines.get(&name) == Some(&v) {
                return false;
            }
            self.defines.insert(name, v);
        } else {
            if !self.defines.contains_key(&name) {
                return false;
            }
            self.defines.remove(&name);
        }

        self.defines_changed();
        true
    }

    pub fn set_define_bool(&mut self, name: String, v: bool) -> bool {
        self.set_define_string(name, if v { Some("1".to_string()) } else { None })
    }

    pub fn get_define_string(&self, name: &str) -> Option<&String> {
        self.defines.get(name)
    }

    pub fn get_define_bool(&self, name: &str) -> bool {
        let str = self.get_define_string(name);

        if let Some(str) = str {
            assert_eq!(str, "1");
        }

        str.is_some()
    }

    // MARK: - Preprocessing

    pub fn preprocess(&mut self) {
        if !self.preprocessed_vertex.is_empty() {
            return;
        }

        self.preprocessed_vertex = self.preprocess_shader(
            ShaderType::Vertex,
            &format!("{}{}", self.both, self.vertex),
        );
        self.preprocessed_frag = self.preprocess_shader(
            ShaderType::Fragment,
            &format!("{}{}", self.both, self.fragment),
        );
    }

    pub fn preprocess_shader(
        &mut self,
        shader_type: ShaderType,
        shader: &str,
    ) -> String {
        let defines_string = self
            .defines
            .iter()
            .map(|(k, v)| format!("#define {} {}\n", k, v))
            .collect::<Vec<String>>()
            .join("");

        format!(
            r#"
            #version 330 core
            {}
            {}
            "#,
            defines_string, shader
        )
    }

    // MARK: - Defaults

    pub fn new(other_mode_h: u32, other_mode_l: u32, combine: CombineParams) -> Self {
        Self {
            preprocessed_vertex: "".to_string(),
            preprocessed_frag: "".to_string(),
            compiled_program: None,

            both: "".to_string(),
            vertex: "".to_string(),
            fragment: "".to_string(),
            defines: HashMap::new(),

            other_mode_h,
            other_mode_l,
            combine,

            shader_input_mapping: ShaderInputMapping::ZERO,
            num_floats: 0,
        }
    }

    pub fn init(&mut self) {
        // for debugging
        self.set_define_bool("USE_ALPHA_VISUALIZER".to_string(), false);

        self.set_define_bool("USE_TEXTURE0".to_string(), self.combine.uses_texture0());
        self.set_define_bool("USE_TEXTURE1".to_string(), self.combine.uses_texture1());
        self.set_define_bool(
            "TEXTURE_EDGE".to_string(),
            other_mode_l_uses_texture_edge(self.other_mode_l),
        );

        self.set_define_bool(
            "USE_FOG".to_string(),
            other_mode_l_uses_fog(self.other_mode_l),
        );
        self.set_define_bool(
            "USE_ALPHA".to_string(),
            other_mode_l_uses_alpha(self.other_mode_l)
                || other_mode_l_uses_texture_edge(self.other_mode_l),
        );
        self.set_define_bool(
            "ALPHA_COMPARE_DITHER".to_string(),
            other_mode_l_alpha_compare_dither(self.other_mode_l),
        );

        self.set_define_bool(
            "ALPHA_COMPARE_THRESHOLD".to_string(),
            other_mode_l_alpha_compare_threshold(self.other_mode_l),
        );

        self.set_define_bool("COLOR_ALPHA_SAME".to_string(), self.combine.cc_ac_same(0));

        self.shader_input_mapping = self.combine.shader_input_mapping();

        self.num_floats = 4;

        if self.get_define_bool("USE_TEXTURE0") || self.get_define_bool("USE_TEXTURE1") {
            self.num_floats += 2;
        }

        if self.get_define_bool("USE_FOG") {
            self.num_floats += 4;
        }

        self.both = format!(
            r#"
            precision mediump float;
            "#,
        );

        self.vertex = format!(
            r#"
            in vec4 aVtxPos;

            #if defined(USE_TEXTURE0) || defined(USE_TEXTURE1)
                in vec2 aTexCoord;
                out vec2 vTexCoord;
            #endif

            #ifdef USE_FOG
                in vec4 aFog;
                out vec4 vFog;
            #endif

            {}

            void main() {{
                #if defined(USE_TEXTURE0) || defined(USE_TEXTURE1)
                    vTexCoord = aTexCoord;
                #endif

                #ifdef USE_FOG
                    vFog = aFog;
                #endif

                {}

                gl_Position = aVtxPos;
            }}
        "#,
            self.generate_vtx_inputs_params(),
            self.generate_vtx_inputs_body(),
        );

        self.fragment = self.generate_frag();
    }

    fn generate_vtx_inputs_params(&mut self) -> String {
        let mut out = String::new();
        let use_alpha = self.get_define_bool("USE_ALPHA");

        for i in 0..self.shader_input_mapping.num_inputs {
            out.push_str(&format!(
                r#"
                in vec{} aInput{};
                out vec{} vInput{};
            "#,
                if use_alpha { 4 } else { 3 },
                i + 1,
                if use_alpha { 4 } else { 3 },
                i + 1,
            ));
            self.num_floats += if use_alpha { 4 } else { 3 };
        }

        out
    }

    fn generate_vtx_inputs_body(&mut self) -> String {
        let mut out = String::new();

        for i in 0..self.shader_input_mapping.num_inputs {
            out.push_str(&format!("vInput{} = aInput{};\n", i + 1, i + 1));
        }

        out
    }

    fn generate_frag(&mut self) -> String {
        let mut inputs = String::new();
        for i in 0..self.shader_input_mapping.num_inputs {
            inputs.push_str(&format!(
                "in vec{} vInput{};\n",
                if self.get_define_bool("USE_ALPHA") {
                    4
                } else {
                    3
                },
                i + 1
            ));
        }

        format!(
            r#"
            #if defined(USE_TEXTURE0) || defined(USE_TEXTURE1)
                in vec2 vTexCoord;
            #endif

            #ifdef USE_FOG
                in vec4 vFog;
            #endif

            {}

            #ifdef USE_TEXTURE0
                uniform sampler2D uTex0;
            #endif
            #ifdef USE_TEXTURE1
                uniform sampler2D uTex1;
            #endif

            #if defined(USE_ALPHA)
                #if defined(ALPHA_COMPARE_DITHER)
                    layout(std140) uniform ubNoise {{
                        int uFrameCount;
                        int uFrameHeight;
                    }};

                    float random(in vec3 value) {{
                        float random = dot(sin(value), vec3(12.9898, 78.233, 37.719));
                        return fract(sin(random) * 143758.5453);
                    }}
                #endif

                #if defined(ALPHA_COMPARE_THRESHOLD)
                    uniform float uAlphaThreshold;
                #endif
            #endif

            out vec4 outColor;

            void main() {{
                #ifdef USE_TEXTURE0
                    vec4 texVal0 = texture(uTex0, vTexCoord);
                #endif
                #ifdef USE_TEXTURE1
                    vec4 texVal1 = texture(uTex1, vTexCoord);
                #endif

                {}

                #ifdef USE_FOG
                    #ifdef USE_ALPHA
                        texel = vec4(mix(texel.rgb, vFog.rgb, vFog.a), texel.a);
                    #else
                        texel = mix(texel, vFog.rgb, vFog.a);
                    #endif
                #endif

                #if defined(USE_ALPHA)
                    #if defined(ALPHA_COMPARE_DITHER)
                        texel.a *= floor(random(vec3(floor(gl_FragCoord.xy * (240.0 / float(uFrameHeight))), float(uFrameCount))) + 0.5);
                    #endif
                    
                    #if defined(ALPHA_COMPARE_THRESHOLD)
                        if (texel.a < uAlphaThreshold) discard;
                    #endif

                    #if defined(TEXTURE_EDGE)
                        if (texel.a < 0.125) discard;
                    #endif

                    #if defined(USE_ALPHA_VISUALIZER)
                        texel.rgb = vec3(texel.a);
                        texel.a = 1.0;
                    #endif
                #endif

                #ifdef USE_ALPHA
                    outColor = texel;
                #else
                    outColor = vec4(texel, 1.0);
                #endif
            }}
        "#,
            inputs,
            self.generate_color_combiner()
        )
    }

    fn generate_color_combiner(&mut self) -> String {
        let do_single: [bool; 2] = [
            self.combine.c0.c == CCMUX::COMBINED,
            self.combine.a0.c == ACMUX::COMBINED__LOD_FRAC,
        ];
        let do_multiply: [bool; 2] = [
            self.combine.c0.b == CCMUX::COMBINED && self.combine.c0.d == CCMUX::COMBINED,
            self.combine.a0.b == ACMUX::COMBINED__LOD_FRAC
                && self.combine.a0.d == ACMUX::COMBINED__LOD_FRAC,
        ];
        let do_mix: [bool; 2] = [
            self.combine.c0.b == self.combine.c0.d,
            self.combine.a0.b == self.combine.a0.d,
        ];

        let use_alpha = self.get_define_bool("USE_ALPHA");

        format!(
            r#"
                #ifdef USE_ALPHA
                    vec4 texel = 
                #else
                    vec3 texel =
                #endif

                #if !defined(COLOR_ALPHA_SAME) && defined(USE_ALPHA)
                    vec4({}, {});
                #else
                    {};
                #endif
        "#,
            self.generate_color_combiner_inputs(
                do_single[0],
                do_multiply[0],
                do_mix[0],
                false,
                false,
                true,
            ),
            self.generate_color_combiner_inputs(
                do_single[1],
                do_multiply[1],
                do_mix[1],
                true,
                true,
                true,
            ),
            self.generate_color_combiner_inputs(
                do_single[0],
                do_multiply[0],
                do_mix[0],
                use_alpha,
                false,
                use_alpha,
            ),
        )
    }

    fn generate_color_combiner_inputs(
        &mut self,
        do_single: bool,
        do_multiply: bool,
        do_mix: bool,
        with_alpha: bool,
        only_alpha: bool,
        use_alpha: bool,
    ) -> String {
        let mut out = String::new();
        let shader_map = self.shader_input_mapping.mirror_mapping[if only_alpha { 1 } else { 0 }];

        if do_single {
            out.push_str(&self.shader_input(
                shader_map[3],
                with_alpha,
                only_alpha,
                use_alpha,
                false,
            ));
        } else if do_multiply {
            out.push_str(&format!(
                "{} * {}",
                self.shader_input(shader_map[0], with_alpha, only_alpha, use_alpha, false),
                self.shader_input(shader_map[2], with_alpha, only_alpha, use_alpha, true),
            ));
        } else if do_mix {
            out.push_str(&format!(
                "mix({}, {}, {})",
                self.shader_input(shader_map[1], with_alpha, only_alpha, use_alpha, false),
                self.shader_input(shader_map[0], with_alpha, only_alpha, use_alpha, false),
                self.shader_input(shader_map[2], with_alpha, only_alpha, use_alpha, true),
            ));
        } else {
            out.push_str(&format!(
                "({} - {}) * {} + {}",
                self.shader_input(shader_map[0], with_alpha, only_alpha, use_alpha, false),
                self.shader_input(shader_map[1], with_alpha, only_alpha, use_alpha, false),
                self.shader_input(shader_map[2], with_alpha, only_alpha, use_alpha, true),
                self.shader_input(shader_map[3], with_alpha, only_alpha, use_alpha, false),
            ));
        }

        out
    }

    fn shader_input(
        &self,
        input: SHADER,
        with_alpha: bool,
        only_alpha: bool,
        inputs_have_alpha: bool,
        hint_single_element: bool,
    ) -> String {
        if !only_alpha {
            match input {
                SHADER::ZERO => {
                    if with_alpha {
                        "vec4(0.0, 0.0, 0.0, 0.0)"
                    } else {
                        "vec3(0.0, 0.0, 0.0)"
                    }
                }
                SHADER::INPUT1 => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput1"
                    } else {
                        "vInput1.rgb"
                    }
                }
                SHADER::INPUT2 => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput2"
                    } else {
                        "vInput2.rgb"
                    }
                }
                SHADER::INPUT3 => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput3"
                    } else {
                        "vInput3.rgb"
                    }
                }
                SHADER::INPUT4 => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput4"
                    } else {
                        "vInput4.rgb"
                    }
                }
                SHADER::TEXEL0 => {
                    if with_alpha {
                        "texVal0"
                    } else {
                        "texVal0.rgb"
                    }
                }
                SHADER::TEXEL0A => {
                    if hint_single_element {
                        "texVal0.a"
                    } else if with_alpha {
                        "vec4(texVal0.a, texVal0.a, texVal0.a, texVal0.a)"
                    } else {
                        "vec3(texVal0.a, texVal0.a, texVal0.a)"
                    }
                }
                SHADER::TEXEL1 => {
                    if with_alpha {
                        "texVal1"
                    } else {
                        "texVal1.rgb"
                    }
                }
            }
        } else {
            match input {
                SHADER::ZERO => "0.0",
                SHADER::INPUT1 => "vInput1.a",
                SHADER::INPUT2 => "vInput2.a",
                SHADER::INPUT3 => "vInput3.a",
                SHADER::INPUT4 => "vInput4.a",
                SHADER::TEXEL0 => "texVal0.a",
                SHADER::TEXEL0A => "texVal0.a",
                SHADER::TEXEL1 => "texVal1.a",
            }
        }
        .to_string()
    }
}
