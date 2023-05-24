use log::trace;

use crate::fast3d::gbi::utils::{
    other_mode_l_uses_alpha, other_mode_l_uses_fog, other_mode_l_uses_noise,
    other_mode_l_uses_texture_edge,
};
use crate::fast3d::utils::color_combiner::{
    ColorCombiner, CombineParams, ShaderInputMapping, ACMUX, CCMUX, SHADER,
};
use std::collections::HashMap;

use super::ShaderProgram;

#[derive(Debug)]
pub struct OpenGLProgram {
    // Compiled program.
    pub preprocessed_vertex: String,
    pub preprocessed_frag: String,
    pub compiled_program: *mut ShaderProgram,

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
        if self.preprocessed_vertex.len() > 0 {
            return;
        }

        self.preprocessed_vertex =
            self.preprocess_shader("vert", &format!("{}{}", self.both, self.vertex));
        self.preprocessed_frag =
            self.preprocess_shader("frag", &format!("{}{}", self.both, self.fragment));
    }

    pub fn preprocess_shader(&mut self, shader_type: &str, shader: &str) -> String {
        let mut shader = shader.to_string();

        // let definesString: string = '';
        // if (defines !== null)
            // definesString = [... defines.entries()].map(([k, v]) => defineStr(k, v)).join('\n');

        let defines_string = self
            .defines
            .iter()
            .map(|(k, v)| format!("#define {} {}\n", k, v))
            .collect::<Vec<String>>()
            .join("");

        format!(
            r#"
        #version 110
        {}
        {}
        "#,
            defines_string,
            shader
        )
    }

    // MARK: - Defaults

    pub fn new(other_mode_h: u32, other_mode_l: u32, combine: CombineParams) -> Self {
        Self {
            preprocessed_vertex: "".to_string(),
            preprocessed_frag: "".to_string(),
            compiled_program: std::ptr::null_mut(),

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
        self.set_define_bool("USE_TEXTURE".to_string(), self.combine.uses_texture0());
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
            other_mode_l_uses_alpha(self.other_mode_l) || other_mode_l_uses_texture_edge(self.other_mode_l),
        );
        self.set_define_bool(
            "USE_NOISE".to_string(),
            other_mode_l_uses_noise(self.other_mode_l),
        );

        self.set_define_bool("COLOR_ALPHA_SAME".to_string(), self.combine.cc_ac_same(0));

        self.shader_input_mapping = self.combine.shader_input_mapping();

        self.num_floats = 4;

        if self.get_define_bool("USE_TEXTURE") {
            self.num_floats += 2;
        }

        if self.get_define_bool("USE_FOG") {
            self.num_floats += 4;
        }

        self.vertex = format!(
            r#"
            attribute vec4 aVtxPos;

            #ifdef USE_TEXTURE
                attribute vec2 aTexCoord;
                varying vec2 vTexCoord;
            #endif

            #ifdef USE_FOG
                attribute vec4 aFog;
                varying vec4 vFog;
            #endif

            {}

            void main() {{
                #ifdef USE_TEXTURE
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

        self.fragment = self.generate_frag().to_string();
    }

    fn generate_vtx_inputs_params(&mut self) -> String {
        let mut out = String::new();
        let use_alpha = self.get_define_bool("USE_ALPHA");

        for i in 0..self.shader_input_mapping.num_inputs {
            out.push_str(&format!(
                r#"
                attribute vec{} aInput{};
                varying vec{} vInput{};
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
                "varying vec{} vInput{};\n",
                if self.get_define_bool("USE_ALPHA") {
                    4
                } else {
                    3
                },
                i + 1
            ));
        }

        // TODO: Could be called uNoise and uNoiseScale (frame_count / window_height)
        format!(
            r#"
            #ifdef USE_TEXTURE
                varying vec2 vTexCoord;
            #endif

            #ifdef USE_FOG
                varying vec4 vFog;
            #endif

            {}

            #ifdef USE_TEXTURE
                uniform sampler2D uTex0;
                #ifdef USE_TEXTURE1
                    uniform sampler2D uTex1;
                #endif
            #endif

            #if defined(USE_ALPHA) || defined(USE_NOISE)
                uniform int frame_count;
                uniform int window_height;

                float random(in vec3 value) {{
                    float random = dot(sin(value), vec3(12.9898, 78.233, 37.719));
                    return fract(sin(random) * 143758.5453);
                }}
            #endif

            void main() {{
                #ifdef USE_TEXTURE
                    vec4 texVal0 = texture2D(uTex0, vTexCoord);
                    #ifdef USE_TEXTURE1
                        vec4 texVal1 = texture2D(uTex1, vTexCoord);
                    #endif
                #endif

                {}

                #ifdef TEXTURE_EDGE
                    if (texel.a > 0.3) texel.a = 1.0; else discard;
                #endif

                #ifdef USE_FOG
                    #ifdef USE_ALPHA
                        texel = vec4(mix(texel.rgb, vFog.rgb, vFog.a), texel.a);
                    #else
                        texel = mix(texel, vFog.rgb, vFog.a);
                    #endif
                #endif

                #if defined(USE_ALPHA) && defined(USE_NOISE)
                    texel.a *= floor(random(vec3(floor(gl_FragCoord.xy * (240.0 / float(window_height))), float(frame_count))) + 0.5);
                #endif

                #ifdef USE_ALPHA
                    gl_FragColor = texel;
                #else
                    gl_FragColor = vec4(texel, 1.0);
                #endif
            }}
        "#,
            inputs,
            self.generate_color_combiner()
        )
    }

    fn generate_color_combiner(&mut self) -> String {
        let do_single: [bool; 2] = [
            self.shader_input_mapping.mirror_mapping[0][2] == SHADER::ZERO,
            self.shader_input_mapping.mirror_mapping[1][2] == SHADER::ZERO,
        ];
        let do_multiply: [bool; 2] = [
            self.shader_input_mapping.mirror_mapping[0][3] == SHADER::ZERO,
            self.shader_input_mapping.mirror_mapping[1][3] == SHADER::ZERO,
        ];
        let do_mix: [bool; 2] = [
            self.shader_input_mapping.mirror_mapping[0][1] == self.shader_input_mapping.mirror_mapping[0][3],
            self.shader_input_mapping.mirror_mapping[1][1] == self.shader_input_mapping.mirror_mapping[1][3],
        ];

        trace!("defines: {:?}", self.defines);
        trace!("do_single: {:?}", do_single);
        trace!("do_multiply: {:?}", do_multiply);
        trace!("do_mix: {:?}", do_mix);

        format!(
            r#"
                #ifdef USE_ALPHA
                    #if !defined(COLOR_ALPHA_SAME) && defined(USE_ALPHA)
                        vec4 texel = vec4({}, {});
                    #else
                        vec4 texel = {};
                    #endif
                #else
                    vec3 texel = {};
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
                self.get_define_bool("USE_ALPHA"),
                false,
                self.get_define_bool("USE_ALPHA"),
            ),
            self.generate_color_combiner_inputs(
                do_single[0],
                do_multiply[0],
                do_mix[0],
                self.get_define_bool("USE_ALPHA"),
                false,
                self.get_define_bool("USE_ALPHA"),
            ),
        )
        .to_string()
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
            out.push_str(
               &self.shader_input(shader_map[3], with_alpha, only_alpha, use_alpha, false,
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
        return if !only_alpha {
            match input {
                SHADER::ZERO => {
                    if with_alpha {
                        "vec4(0.0, 0.0, 0.0, 0.0)"
                    } else {
                        "vec3(0.0, 0.0, 0.0)"
                    }
                }
                SHADER::ONE => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput1"
                    } else {
                        "vInput1.rgb"
                    }
                }
                SHADER::TWO => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput2"
                    } else {
                        "vInput2.rgb"
                    }
                }
                SHADER::THREE => {
                    if with_alpha || !inputs_have_alpha {
                        "vInput3"
                    } else {
                        "vInput3.rgb"
                    }
                }
                SHADER::FOUR => {
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
                    } else {
                        if with_alpha {
                            "vec4(texVal0.a, texVal0.a, texVal0.a, texVal0.a)"
                        } else {
                            "vec3(texVal0.a, texVal0.a, texVal0.a)"
                        }
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
                SHADER::ONE => "vInput1.a",
                SHADER::TWO => "vInput2.a",
                SHADER::THREE => "vInput3.a",
                SHADER::FOUR => "vInput4.a",
                SHADER::TEXEL0 => "texVal0.a",
                SHADER::TEXEL0A => "texVal0.a",
                SHADER::TEXEL1 => "texVal1.a",
            }
        }.to_string();
    }
}
