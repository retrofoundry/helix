use std::collections::HashMap;

use wgpu::{
    BindGroupLayout, ShaderModule, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexStepMode,
};

use crate::fast3d::{
    gbi::utils::{
        get_cycle_type_from_other_mode_h, get_textfilter_from_other_mode_h,
        other_mode_l_alpha_compare_dither, other_mode_l_alpha_compare_threshold,
        other_mode_l_uses_alpha, other_mode_l_uses_fog, other_mode_l_uses_texture_edge,
    },
    rdp::OtherModeHCycleType,
    utils::{
        color_combiner::{CombineParams, ACMUX, CCMUX},
        texture::TextFilt,
    },
};

pub struct WgpuProgram {
    // Compiled program.
    pub processed_shader: String,
    pub compiled_program: Option<ShaderModule>,

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

impl WgpuProgram {
    // MARK: - Defines
    pub fn defines_changed(&mut self) {
        self.processed_shader = "".to_string();
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
        if !self.processed_shader.is_empty() {
            return;
        }

        self.processed_shader = format!(
            "{both}{vertex}{fragment}",
            both = self.both,
            vertex = self.vertex,
            fragment = self.fragment,
        );
    }

    // MARK: - Defaults

    pub fn new(
        other_mode_h: u32,
        other_mode_l: u32,
        geometry_mode: u32,
        combine: CombineParams,
    ) -> Self {
        Self {
            processed_shader: "".to_string(),
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

        self.both = format!(
            r#"
            const tZero = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            const tHalf = vec4<f32>(0.5 ,0.5, 0.5, 0.5);
            const tOne = vec4<f32>(1.0 ,1.0, 1.0, 1.0);

            const DRAWING_RECT: f32 = 0.0;

            struct VertexInput {{
                @location(0) position: vec4<f32>,
                @location(1) color: vec4<f32>,
                {uv_input}
            }};
            
            struct VertexOutput {{
                @location(0) color: vec4<f32>,
                {uv_output}
                @builtin(position) position: vec4<f32>,
            }};

            "#,
            uv_input = self.on_define(
                &["USE_TEXTURE0", "USE_TEXTURE1"],
                "@location(2) uv: vec2<f32>,"
            ),
            uv_output = self.on_define(
                &["USE_TEXTURE0", "USE_TEXTURE1"],
                "@location(1) uv: vec2<f32>,"
            ),
        );

        self.vertex = self.generate_vertex();

        self.fragment = self.generate_fragment();
    }

    fn generate_vertex(&mut self) -> String {
        let compute_uniform_fields = || {
            if self.get_define_bool("USE_FOG") {
                return format!(
                    r#"
                    fog_multiplier: f32,
                    fog_offset: f32,
                    "#
                );
            }

            return "_pad: vec2<f32>,".to_string();
        };

        let compute_color = || {
            if self.get_define_bool("USE_FOG") {
                return format!(
                    r#"
                    var fog_value = (max(0.0, out.position.z) - out.position.w) * uniforms.fog_multiplier + uniforms.fog_offset;
                    fog_value = clamp(fog_value, 0.0, 255.0);
                    out.color = vec4<f32>(in.color.xyz, fog_value);
                    "#
                );
            }

            return "out.color = in.color;".to_string();
        };

        format!(
            r#"
            struct Uniforms {{
                projection_matrix: mat4x4<f32>,
                {fog_params}
            }}

            @group(0) @binding(0)
            var<uniform> uniforms: Uniforms;

            @vertex
            fn vs_main(in: VertexInput) -> VertexOutput {{
                var out: VertexOutput;

                if (in.position.w == DRAWING_RECT) {{
                    out.position = uniforms.projection_matrix * vec4<f32>(in.position.xyz, 1.0);
                }} else {{
                    out.position = uniforms.projection_matrix * in.position;
                }}

                // map z to [0, 1]
                out.position.z = (out.position.z + out.position.w) / (2.0 * out.position.w);

                {color}

                {uv}

                return out;
            }}
            "#,
            uv = self.on_define(&["USE_TEXTURE0", "USE_TEXTURE1"], "out.uv = in.uv;"),
            fog_params = compute_uniform_fields(),
            color = compute_color(),
        )
    }

    fn generate_fragment(&mut self) -> String {
        let tex_filter = match get_textfilter_from_other_mode_h(self.other_mode_h) {
            TextFilt::G_TF_POINT => "Point",
            TextFilt::G_TF_AVERAGE => "Average",
            TextFilt::G_TF_BILERP => "Bilerp",
        };

        let color_input_common = |input| match input {
            CCMUX::COMBINED => "comb_color.rgb",
            CCMUX::TEXEL0 => "tex0.rgb",
            CCMUX::TEXEL1 => "tex1.rgb",
            CCMUX::PRIMITIVE => "combine_uniforms.prim_color.rgb",
            CCMUX::SHADE => "shade_color.rgb",
            CCMUX::ENVIRONMENT => "combine_uniforms.env_color.rgb",
            _ => panic!("Should be unreachable"),
        };

        let color_input_a = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "tOne.rgb", // matching against ONE
                    CCMUX::COMBINED_ALPHA__NOISE__K4 => "vec3(noise, noise, noise)", // matching against NOISE
                    _ => "tZero.rgb",
                }
            }
        };

        let color_input_b = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "combine_uniforms.key_center", // matching against CENTER
                    CCMUX::COMBINED_ALPHA__NOISE__K4 => "vec3(combine_uniforms.convert_k4, combine_uniforms.convert_k4, combine_uniforms.convert_k4)", // matching against K4
                    _ => "tZero.rgb",
                }
            }
        };

        let color_input_c = |input| {
            if input <= CCMUX::ENVIRONMENT {
                color_input_common(input)
            } else {
                match input {
                    CCMUX::CENTER__SCALE__ONE => "combine_uniforms.key_scale", // matching against SCALE
                    CCMUX::COMBINED_ALPHA__NOISE__K4 => "comb_color.aaa", // matching against COMBINED_ALPHA
                    CCMUX::TEXEL0_ALPHA => "tex0.aaa",
                    CCMUX::TEXEL1_ALPHA => "tex1.aaa",
                    CCMUX::PRIMITIVE_ALPHA => "combine_uniforms.prim_color.aaa",
                    CCMUX::SHADE_ALPHA => "shade_color.aaa",
                    CCMUX::ENV_ALPHA => "combine_uniforms.env_color.aaa",
                    CCMUX::LOD_FRACTION => "tZero.rgb", // TODO: LOD FRACTION
                    CCMUX::PRIM_LOD_FRACTION => "vec3(combine_uniforms.prim_lod_frac, combine_uniforms.prim_lod_frac, combine_uniforms.prim_lod_frac)",
                    CCMUX::K5 => "vec3(combine_uniforms.convert_k5, combine_uniforms.convert_k5, combine_uniforms.convert_k5)",
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
                ACMUX::COMBINED__LOD_FRAC => "comb_color.a", // matching against COMBINED
                ACMUX::TEXEL0 => "tex0.a",
                ACMUX::TEXEL1 => "tex1.a",
                ACMUX::PRIMITIVE => "combine_uniforms.prim_color.a",
                ACMUX::SHADE => {
                    if self.get_define_bool("USE_FOG") {
                        "tOne.a"
                    } else {
                        "shade_color.a"
                    }
                }
                ACMUX::ENVIRONMENT => "combine_uniforms.env_color.a",
                ACMUX::PRIM_LOD_FRAC__ONE => "tOne.a", // matching against ONE
                _ => "tZero.a",
            }
        };

        let alpha_input_c = |input| {
            match input {
                ACMUX::COMBINED__LOD_FRAC => "tZero.a", // TODO: LOD_FRAC
                ACMUX::TEXEL0 => "tex0.a",
                ACMUX::TEXEL1 => "tex1.a",
                ACMUX::PRIMITIVE => "combine_uniforms.prim_color.a",
                ACMUX::SHADE => "shade_color.a",
                ACMUX::ENVIRONMENT => "combine_uniforms.env_color.a",
                ACMUX::PRIM_LOD_FRAC__ONE => "combine_uniforms.prim_lod_frac",
                _ => "tZero.a",
            }
        };

        format!(
            r#"
            struct BlendParamsUniforms {{
                blend_color: vec4<f32>,
                fog_color: vec4<f32>,
            }};
            
            struct CombineParamsUniforms {{
                prim_color: vec4<f32>,
                env_color: vec4<f32>,
                key_center: vec3<f32>,
                key_scale: vec3<f32>,
                prim_lod_frac: f32,
                convert_k4: f32,
                convert_k5: f32,
            }};
            
            struct FrameUniforms {{
                count: u32,
                height: u32,
            }};

            @group(1) @binding(0)
            var<uniform> blend_uniforms: BlendParamsUniforms;
            @group(1) @binding(1)
            var<uniform> combine_uniforms: CombineParamsUniforms;
            @group(1) @binding(2)
            var<uniform> frame_uniforms: FrameUniforms;

            {tex0_bindings}

            {tex1_bindings}
            
            fn random(value: vec3<f32>) -> f32 {{
                var random = dot(sin(value), vec3<f32>(12.9898, 78.233, 37.719));
                return fract(sin(random) * 143758.5453);
            }}

            fn textureSampleN64Point(tex: texture_2d<f32>, samplr: sampler, uv: vec2<f32>) -> vec4<f32> {{
                return textureSample(tex, samplr, uv);
            }}

            fn textureSampleN64Average(tex: texture_2d<f32>, samplr: sampler, uv: vec2<f32>) -> vec4<f32> {{
                // Unimplemented.
                return textureSample(tex, samplr, uv);
            }}

            fn textureSampleN64Bilerp(tex: texture_2d<f32>, samplr: sampler, uv: vec2<f32>) -> vec4<f32> {{
                var tex_size = vec2<f32>(textureDimensions(tex, 0));
                var offset = fract(uv * tex_size - 0.5);
            
                var offset_sign = sign(offset);
                var offset_abs = abs(offset);

                var s0 = textureSample(tex, samplr, uv - offset / tex_size);
                var s1 = textureSample(tex, samplr, uv - offset_sign * vec2<f32>(1.0, 0.0) / tex_size);
                var s2 = textureSample(tex, samplr, uv - offset_sign * vec2<f32>(0.0, 1.0) / tex_size);
            
                return s0 + offset_abs.x * (s1 - s0) + offset_abs.y * (s2 - s0);
            }}

            fn combineColorCycle0(comb_color: vec4<f32>, shade_color: vec4<f32>, tex0: vec4<f32>, tex1: vec4<f32>, noise: f32) -> vec3<f32> {{
                return ({c0a} - {c0b}) * {c0c} + {c0d};
            }}

            fn combineAlphaCycle0(comb_color: vec4<f32>, shade_color: vec4<f32>, tex0: vec4<f32>, tex1: vec4<f32>, noise: f32) -> f32 {{
                return ({a0a} - {a0b}) * {a0c} + {a0d};
            }}

            fn combineColorCycle1(comb_color: vec4<f32>, shade_color: vec4<f32>, tex0: vec4<f32>, tex1: vec4<f32>, noise: f32) -> vec3<f32> {{
                return ({c1a} - {c1b}) * {c1c} + {c1d};
            }}
            
            fn combineAlphaCycle1(comb_color: vec4<f32>, shade_color: vec4<f32>, tex0: vec4<f32>, tex1: vec4<f32>, noise: f32) -> f32 {{
                return ({a1a} - {a1b}) * {a1c} + {a1d};
            }}

            @fragment
            fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
                var tex_val0 = tOne;
                var tex_val1 = tOne;

                {sample_texture0}
                {sample_texture1}
                
                var noise = (random(vec3(floor(in.position.xy * (240.0 / f32(frame_uniforms.height))), f32(frame_uniforms.count))) + 1.0) / 2.0;

                var texel = vec4<f32>(
                    combineColorCycle0(tHalf, in.color, tex_val0, tex_val1, noise),
                    combineAlphaCycle0(tHalf, in.color, tex_val0, tex_val1, noise)
                );

                {second_pass_combine}

                {alpha_compare_dither}
                {alpha_compare_threshold}

                {texture_edge}

                {alpha_visualizer}

                {blend_fog}

                return texel;
            }}
            "#,
            tex0_bindings = self.on_define(
                &["USE_TEXTURE0"],
                r#"
                @group(2) @binding(0)
                var texture0: texture_2d<f32>;
                @group(2) @binding(1)
                var sampler0: sampler;
                "#
            ),
            tex1_bindings = self.on_define(
                &["USE_TEXTURE1"],
                r#"
                @group(2) @binding(2)
                var texture1: texture_2d<f32>;
                @group(2) @binding(3)
                var sampler1: sampler;
            "#
            ),
            c0a = color_input_a(self.combine.c0.a),
            c0b = color_input_b(self.combine.c0.b),
            c0c = color_input_c(self.combine.c0.c),
            c0d = color_input_d(self.combine.c0.d),
            a0a = alpha_input_abd(self.combine.a0.a),
            a0b = alpha_input_abd(self.combine.a0.b),
            a0c = alpha_input_c(self.combine.a0.c),
            a0d = alpha_input_abd(self.combine.a0.d),
            c1a = color_input_a(self.combine.c1.a),
            c1b = color_input_b(self.combine.c1.b),
            c1c = color_input_c(self.combine.c1.c),
            c1d = color_input_d(self.combine.c1.d),
            a1a = alpha_input_abd(self.combine.a1.a),
            a1b = alpha_input_abd(self.combine.a1.b),
            a1c = alpha_input_c(self.combine.a1.c),
            a1d = alpha_input_abd(self.combine.a1.d),
            sample_texture0 = self.on_define_str(
                &["USE_TEXTURE0"],
                format!("tex_val0 = textureSampleN64{tex_filter}(texture0, sampler0, in.uv);")
            ),
            sample_texture1 = self.on_define_str(
                &["USE_TEXTURE1"],
                format!("tex_val1 = textureSampleN64{tex_filter}(texture1, sampler1, in.uv);")
            ),
            second_pass_combine =  self.on_define(
                &["TWO_CYCLE"],
                r#"
                // Note that in the second cycle, Tex0 and Tex1 are swapped
                texel = vec4<f32>(
                     combineColorCycle1(texel, in.color tex_val1, tex_val0, noise),
                     combineAlphaCycle1(texel, in.color, tex_val1, tex_val0, noise)
                 );
                "#,
            ),
            alpha_compare_dither = self.on_define(
                &["USE_ALPHA", "ALPHA_COMPARE_DITHER"],
                r#"
                var random_alpha = floor(random(vec3(floor(in.position.xy * (240.0 / f32(frame_uniforms.height))), f32(frame_uniforms.count))) + 0.5);
                if texel.a < random_alpha { discard; }
                "#,
            ),
            alpha_compare_threshold = self.on_define(
                &["USE_ALPHA", "ALPHA_COMPARE_THRESHOLD"],
                "if texel.a < blend_uniforms.blend_color.a { discard; }"
            ),
            texture_edge = self.on_define(
                &["USE_ALPHA", "TEXTURE_EDGE"],
                "if texel.a < 0.125 { discard; }"
            ),
            alpha_visualizer = self.on_define(
                &["USE_ALPHA", "USE_ALPHA_VISUALIZER"],
                "texel = mix(texel, vec4<f32>(1.0, 0.0, 1.0, 1.0), 0.5);"
            ),
            blend_fog = self.on_define(
                &["USE_FOG"],
                "texel = vec4<f32>(mix(texel.rgb, blend_uniforms.fog_color.rgb, in.color.a), texel.a);"
            ),
        )
    }

    // MARK: - Pipeline Helpers

    pub fn vertex_description(&self) -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: (self.num_floats * ::std::mem::size_of::<f32>()) as u64,
            step_mode: VertexStepMode::Vertex,
            // TODO: Is there a better way to construct this?
            attributes: if self.get_define_bool("USE_TEXTURE0")
                || self.get_define_bool("USE_TEXTURE1")
            {
                &[
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 0, // position
                        shader_location: 0,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: std::mem::size_of::<[f32; 4]>() as u64, // color
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: VertexFormat::Float32x2,
                        offset: std::mem::size_of::<[f32; 8]>() as u64, // texcoord
                        shader_location: 2,
                    },
                ]
            } else {
                &[
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 0, // position
                        shader_location: 0,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: std::mem::size_of::<[f32; 4]>() as u64, // color
                        shader_location: 1,
                    },
                ]
            },
        }
    }

    pub fn create_texture_bind_group_layout(&self, device: &wgpu::Device) -> BindGroupLayout {
        {
            let mut group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = Vec::new();

            for i in 0..2 {
                let texture_index = format!("USE_TEXTURE{}", i);
                if self.get_define_bool(&texture_index) {
                    group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                        binding: i * 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    });

                    group_layout_entries.push(wgpu::BindGroupLayoutEntry {
                        binding: (i * 2 + 1),
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // TODO: Is this the appropriate setting?
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    });
                }
            }

            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Textures/Samplers Group Layout"),
                entries: &group_layout_entries,
            })
        }
    }

    // MARK: - Helpers

    fn on_define(&self, def: &[&str], output: &'static str) -> &str {
        for d in def {
            if self.get_define_bool(d) {
                return output;
            } else {
                return "";
            }
        }

        ""
    }

    fn on_define_str(&self, def: &[&str], output: String) -> String {
        for d in def {
            if self.get_define_bool(d) {
                return output;
            } else {
                return "".to_string();
            }
        }

        "".to_string()
    }
}
