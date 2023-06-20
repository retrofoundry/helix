use crate::fast3d::gbi::utils::{
    geometry_mode_uses_lighting, get_cycle_type_from_other_mode_h,
    get_textfilter_from_other_mode_h, other_mode_l_alpha_compare_dither,
    other_mode_l_alpha_compare_threshold, other_mode_l_uses_alpha, other_mode_l_uses_fog,
    other_mode_l_uses_texture_edge,
};

use crate::fast3d::utils::{
    color_combiner::{CombineParams, ACMUX, CCMUX},
    texture::TextFilt,
};

use crate::fast3d::rdp::OtherModeHCycleType;
use std::collections::HashMap;

#[derive(PartialEq, Eq)]
pub enum ShaderType {
    Vertex,
    Fragment,
}

#[derive(Debug, Clone)]
pub struct OpenGLProgram<T> {
    // Compiled program.
    pub preprocessed_vertex: String,
    pub preprocessed_frag: String,
    pub compiled_program: Option<T>,

    // inputs
    pub both: String,
    pub vertex: String,
    pub fragment: String,
    pub defines: HashMap<String, String>,

    // configurators
    other_mode_h: u32,
    other_mode_l: u32,
    geometry_mode: u32,
    combine: CombineParams,

    pub num_floats: usize,
}

impl<T> OpenGLProgram<T> {
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

        self.preprocessed_vertex =
            self.preprocess_shader(ShaderType::Vertex, &format!("{}{}", self.both, self.vertex));
        self.preprocessed_frag = self.preprocess_shader(
            ShaderType::Fragment,
            &format!("{}{}", self.both, self.fragment),
        );
    }

    pub fn preprocess_shader(&mut self, _shader_type: ShaderType, shader: &str) -> String {
        let defines_string = self
            .defines
            .iter()
            .map(|(k, v)| format!("#define {} {}\n", k, v))
            .collect::<Vec<String>>()
            .join("");

        format!(
            r#"
            #version 140
            {}
            {}
            "#,
            defines_string, shader
        )
    }

    // MARK: - Defaults

    pub fn new(
        other_mode_h: u32,
        other_mode_l: u32,
        geometry_mode: u32,
        combine: CombineParams,
    ) -> Self {
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
            geometry_mode,
            combine,

            num_floats: 0,
        }
    }

    pub fn init(&mut self) {
        // for debugging
        self.set_define_bool("USE_ALPHA_VISUALIZER".to_string(), false);
        self.set_define_bool("ONLY_VERTEX_COLOR".to_string(), false);

        self.set_define_bool(
            "TWO_CYCLE".to_string(),
            get_cycle_type_from_other_mode_h(self.other_mode_h)
                == OtherModeHCycleType::G_CYC_2CYCLE,
        );
        self.set_define_bool(
            "LIGHTING".to_string(),
            geometry_mode_uses_lighting(self.geometry_mode),
        );
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

        self.num_floats = 8;

        if self.get_define_bool("USE_TEXTURE0") || self.get_define_bool("USE_TEXTURE1") {
            self.num_floats += 2;
        }

        self.both = r#"
            const vec4 tZero = vec4(0.0);
            const vec4 tHalf = vec4(0.5);
            const vec4 tOne = vec4(1.0);

            const int DRAWING_RECT = 0;
            "#
        .to_string();

        self.vertex = r#"
            in vec4 aVtxPos;

            in vec4 aVtxColor;
            out vec4 vVtxColor;

            #if defined(USE_TEXTURE0) || defined(USE_TEXTURE1)
                in vec2 aTexCoord;
                out vec2 vTexCoord;
            #endif

            uniform mat4 uProjection;
            #ifdef USE_FOG
                uniform float uFogMultiplier;
                uniform float uFogOffset;
            #endif

            // Convert from 0...1 UNORM range to SNORM range
            vec3 ConvertToSignedInt(vec3 t_Input) {
                ivec3 t_Num = ivec3(t_Input * 255.0);
                // Sign extend
                t_Num = t_Num << 24 >> 24;
                return vec3(t_Num) / 127.0;
            }

            void main() {
                if (aVtxPos.w == DRAWING_RECT) {
                    gl_Position = vec4(aVtxPos.xyz, 1.0);
                } else {
                    gl_Position = aVtxPos * uProjection;
                }

                #ifdef USE_FOG
                    float fogValue = (max(gl_Position.z, 0.0) / gl_Position.w) * uFogMultiplier + uFogOffset;
                    fogValue = clamp(fogValue, 0.0, 255.0);
                    vVtxColor = vec4(aVtxColor.rgb, fogValue / 255.0);
                #else
                    vVtxColor = aVtxColor;
                #endif

                #ifdef LIGHTING
                    // convert (unsigned) colors to normal vector components
                    vec4 t_Normal = vec4(ConvertToSignedInt(aVtxColor.rgb), 0.0);
                    // t_Normal = normalize(Mul(_Mat4x4(u_BoneMatrix[t_BoneIndex]), t_Normal));
                #endif

                #if defined(USE_TEXTURE0) || defined(USE_TEXTURE1)
                    vTexCoord = aTexCoord;
                #endif
            }
        "#
        .to_string();

        self.fragment = self.generate_frag();
    }

    fn generate_frag(&mut self) -> String {
        let tex_filter = match get_textfilter_from_other_mode_h(self.other_mode_h) {
            TextFilt::G_TF_POINT => "Point",
            TextFilt::G_TF_AVERAGE => "Average",
            TextFilt::G_TF_BILERP => "Bilerp",
        };

        let color_input_common = |input| match input {
            CCMUX::COMBINED => "tCombColor.rgb",
            CCMUX::TEXEL0 => "texVal0.rgb",
            CCMUX::TEXEL1 => "texVal1.rgb",
            CCMUX::PRIMITIVE => "uPrimColor.rgb",
            CCMUX::SHADE => "vVtxColor.rgb",
            CCMUX::ENVIRONMENT => "uEnvColor.rgb",
            _ => panic!("Should be unreachable"),
        };

        let color_input_a = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "tOne.rgb", // matching against ONE
                    CCMUX::COMBINED_ALPHA__NOISE__K4 => "vec3(RAND_NOISE, RAND_NOISE, RAND_NOISE)", // matching against NOISE
                    _ => "tZero.rgb",
                }
            }
        };

        let color_input_b = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "uKeyCenter", // matching against CENTER
                    CCMUX::COMBINED_ALPHA__NOISE__K4 => "vec3(uK4, uK4, uK4)", // matching against K4
                    _ => "tZero.rgb",
                }
            }
        };

        let color_input_c = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "uKeyScale", // matching against SCALE
                    CCMUX::COMBINED_ALPHA__NOISE__K4 => "tCombColor.aaa", // matching against COMBINED_ALPHA
                    CCMUX::TEXEL0_ALPHA => "texVal0.aaa",
                    CCMUX::TEXEL1_ALPHA => "texVal1.aaa",
                    CCMUX::PRIMITIVE_ALPHA => "uPrimColor.aaa",
                    CCMUX::SHADE_ALPHA => "vVtxColor.aaa",
                    CCMUX::ENV_ALPHA => "uEnvColor.aaa",
                    CCMUX::LOD_FRACTION => "tZero.rgb", // TODO: LOD FRACTION
                    CCMUX::PRIM_LOD_FRACTION => "vec3(uPrimLodFrac, uPrimLodFrac, uPrimLodFrac)",
                    CCMUX::K5 => "vec3(uK5, uK5, uK5)",
                    _ => "tZero.rgb",
                }
            }
        };

        let color_input_d = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "tOne.rgb", // matching against ONE
                    _ => "tZero.rgb",
                }
            }
        };

        let alpha_input_abd = |input| {
            match input {
                ACMUX::COMBINED__LOD_FRAC => "tCombColor.a", // matching against COMBINED
                ACMUX::TEXEL0 => "texVal0.a",
                ACMUX::TEXEL1 => "texVal1.a",
                ACMUX::PRIMITIVE => "uPrimColor.a",
                ACMUX::SHADE => {
                    if self.get_define_bool("USE_FOG") {
                        "tOne.a"
                    } else {
                        "vVtxColor.a"
                    }
                }
                ACMUX::ENVIRONMENT => "uEnvColor.a",
                ACMUX::PRIM_LOD_FRAC__ONE => "tOne.a", // matching against ONE
                _ => "tZero.a",
            }
        };

        let alpha_input_c = |input| {
            match input {
                ACMUX::COMBINED__LOD_FRAC => "tZero.a", // TODO: LOD_FRAC
                ACMUX::TEXEL0 => "texVal0.a",
                ACMUX::TEXEL1 => "texVal1.a",
                ACMUX::PRIMITIVE => "uPrimColor.a",
                ACMUX::SHADE => "vVtxColor.a",
                ACMUX::ENVIRONMENT => "uEnvColor.a",
                ACMUX::PRIM_LOD_FRAC__ONE => "uPrimLodFrac",
                _ => "tZero.a",
            }
        };

        format!(
            r#"
            in vec4 vVtxColor;

            #if defined(USE_TEXTURE0) || defined(USE_TEXTURE1)
                in vec2 vTexCoord;
            #endif

            #ifdef USE_FOG
                uniform vec3 uFogColor;
            #endif

            // blend parameters
            uniform vec4 uBlendColor;

            // combine parameters
            // TODO: use a uniform block?
            uniform vec4 uPrimColor;
            uniform vec4 uEnvColor;
            uniform vec3 uKeyCenter;
            uniform vec3 uKeyScale;
            uniform float uPrimLodFrac;
            uniform float uK4;
            uniform float uK5;

            #ifdef USE_TEXTURE0
                uniform sampler2D uTex0;
            #endif
            #ifdef USE_TEXTURE1
                uniform sampler2D uTex1;
            #endif

            #if defined(USE_ALPHA)
                #if defined(ALPHA_COMPARE_DITHER)
                    uniform int uFrameCount;
                    uniform int uFrameHeight;

                    float random(in vec3 value) {{
                        float random = dot(sin(value), vec3(12.9898, 78.233, 37.719));
                        return fract(sin(random) * 143758.5453);
                    }}
                #endif
            #endif

            out vec4 outColor;

            #define TEX_OFFSET(offset) texture(tex, texCoord - (offset) / texSize)
            #define RAND_NOISE "((random(vec3(floor(gl_FragCoord.xy * (240.0 / float(uFrameHeight)), float(uFrameCount))) + 1.0) / 2.0)"

            vec4 Texture2D_N64_Point(in sampler2D tex, in vec2 texCoord) {{
                return texture(tex, texCoord);
            }}
            
            vec4 Texture2D_N64_Average(in sampler2D tex, in vec2 texCoord) {{
                // Unimplemented.
                return texture(tex, texCoord);
            }}
            
            // Implements N64-style "triangle bilienar filtering" with three taps.
            // Based on ArthurCarvalho's implementation, modified for use here.
            vec4 Texture2D_N64_Bilerp(in sampler2D tex, in vec2 texCoord) {{
                vec2 texSize = vec2(textureSize(tex, 0));
                vec2 offset = fract(texCoord * texSize - vec2(0.5));
                offset -= step(1.0, offset.x + offset.y);
                vec4 s0 = TEX_OFFSET(offset);
                vec4 s1 = TEX_OFFSET(vec2(offset.x - sign(offset.x), offset.y));
                vec4 s2 = TEX_OFFSET(vec2(offset.x, offset.y - sign(offset.y)));
                return s0 + abs(offset.x) * (s1 - s0) + abs(offset.y) * (s2 - s0);
            }}
            
            #define Texture2D_N64 Texture2D_N64_{}

            vec3 CombineColorCycle0(vec4 tCombColor, vec4 texVal0, vec4 texVal1) {{
                return ({} - {}) * {} + {};
            }} 
            
            float CombineAlphaCycle0(vec4 tCombColor, vec4 texVal0, vec4 texVal1) {{
                return ({} - {}) * {} + {};
            }}
            
            vec3 CombineColorCycle1(vec4 tCombColor, vec4 texVal0, vec4 texVal1) {{
                return ({} - {}) * {} + {};
            }}
            
            float CombineAlphaCycle1(vec4 tCombColor, vec4 texVal0, vec4 texVal1) {{
                return ({} - {}) * {} + {};
            }}

            vec3 to_linear(vec3 v) {{
                return pow(v, vec3(2.2));
            }}

            vec4 to_linear(vec4 v) {{
                return vec4(to_linear(v.rgb), v.a);
            }}

            vec3 to_srgb(vec3 v) {{
                return pow(v, vec3(1.0 / 2.2));
            }}
            
            vec4 to_srgb(vec4 v) {{
                return vec4(to_srgb(v.rgb), v.a);
            }}

            void main() {{
                vec4 texVal0 = tOne, texVal1 = tOne;

                #ifdef USE_TEXTURE0
                    texVal0 = Texture2D_N64(uTex0, vTexCoord);
                #endif
                #ifdef USE_TEXTURE1
                    texVal1 = Texture2D_N64(uTex1, vTexCoord);
                #endif

                #ifdef ONLY_VERTEX_COLOR
                    vec4 texel = vVtxColor;
                #else
                    vec4 texel = vec4(
                        CombineColorCycle0(tHalf, texVal0, texVal1),
                        CombineAlphaCycle0(tHalf, texVal0, texVal1)
                    );
                    
                    #ifdef TWO_CYCLE
                        // Note that in the second cycle, Tex0 and Tex1 are swapped
                        texel = vec4(
                            CombineColorCycle1(texel, texVal1, texVal0),
                            CombineAlphaCycle1(texel, texVal1, texVal0)
                        );
                    #endif
                #endif

                #if defined(USE_ALPHA)
                    #if defined(ALPHA_COMPARE_DITHER)
                        if (texel.a < floor(random(vec3(floor(gl_FragCoord.xy * (240.0 / float(uFrameHeight))), float(uFrameCount))) + 0.5)) discard;
                    #endif
                    
                    #if defined(ALPHA_COMPARE_THRESHOLD)
                        if (texel.a < uBlendColor.a) discard;
                    #endif

                    #if defined(TEXTURE_EDGE)
                        if (texel.a < 0.125) discard;
                    #endif

                    #if defined(USE_ALPHA_VISUALIZER)
                        texel = mix(texel, vec4(1.0f, 0.0f, 1.0f, 1.0f), 0.5f);
                    #endif
                #endif

                // TODO: Blender
                #ifdef USE_FOG
                    texel = vec4(mix(texel.rgb, uFogColor.rgb, vVtxColor.a), texel.a);
                #endif

                outColor = texel;
            }}
        "#,
            tex_filter,
            color_input_a(self.combine.c0.a),
            color_input_b(self.combine.c0.b),
            color_input_c(self.combine.c0.c),
            color_input_d(self.combine.c0.d),
            alpha_input_abd(self.combine.a0.a),
            alpha_input_abd(self.combine.a0.b),
            alpha_input_c(self.combine.a0.c),
            alpha_input_abd(self.combine.a0.d),
            color_input_a(self.combine.c1.a),
            color_input_b(self.combine.c1.b),
            color_input_c(self.combine.c1.c),
            color_input_d(self.combine.c1.d),
            alpha_input_abd(self.combine.a1.a),
            alpha_input_abd(self.combine.a1.b),
            alpha_input_c(self.combine.a1.c),
            alpha_input_abd(self.combine.a1.d),
        )
    }
}
