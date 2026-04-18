#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

struct DetailExtendedNormal {
    normalize_scale: vec2<f32>,
    //blend_sampler: sampler,
    //detail_normal_texture: texture_2d<f32>,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    // Web examples WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_8b: u32,
    _webgl2_padding_12b: u32,
    _webgl2_padding_16b: u32,
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> detail_material: DetailExtendedNormal;

@group(#{MATERIAL_BIND_GROUP}) @binding(101) var blend_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var detail_normal_texture: texture_2d<f32>;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;

    // modify normal
    let normal_color = textureSample(
        detail_normal_texture,
        blend_sampler,
        in.uv * detail_material.normalize_scale
    );

    // pbr_input.N = normal_color.xyz;
    pbr_input.N = normalize(pbr_input.N * normal_color.xyz);

    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // pbr_input.N = normalize(pbr_input.N * detail_material.normalize_scale);
    // var new_color = apply_pbr_lighting(pbr_input);

    // out.color *= new_color;

    // we can optionally modify the lit color before post-processing is applied
//    out.color = vec4<f32>(vec4<u32>(out.color * f32(detail_material.quantize_steps))) / f32(detail_material.quantize_steps);

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

#endif

    return out;
}
