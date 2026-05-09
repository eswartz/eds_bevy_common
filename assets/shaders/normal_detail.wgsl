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
    scale: vec2<f32>,
    blend: f32,
#ifdef SIXTEEN_BYTE_ALIGNMENT
    _webgl2_padding_16b: u32,
#endif
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> detail_material: DetailExtendedNormal;

@group(#{MATERIAL_BIND_GROUP}) @binding(102) var blend_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var detail_normal_texture: texture_2d<f32>;

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
        in.uv * detail_material.scale,
    );

    let n1 = pbr_input.N;
    let n2 = (vec3(normal_color.xy, 0.0) * 2.0 - 1.0) * detail_material.blend;
    let r = normalize(n1 + n2);
    pbr_input.N = r;

    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

#endif

    return out;
}

fn ovl(u: f32, l: f32) -> f32 {
    if l < 0.5 {
        return u * 2.0 * l;
    } else {
        return 1.0 - ((1.0 - u) * (1.0 - l) * 2.0);
    }
}

fn overlay(u: vec3<f32>, l: vec3<f32>) -> vec3<f32> {
    // https://docs.gimp.org/2.10/en/layer-mode-group-contrast.html

    return vec3(ovl(u[0], l[0]), ovl(u[1], l[1]), ovl(u[2], l[2]));
}
