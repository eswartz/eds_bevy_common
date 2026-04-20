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
    // Web examples WebGL2 support: structs must be 16 byte aligned.
    _webgl2_padding_12b: u32,
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
        // in.uv,
    );

    // pbr_input.N = normal_color.xyz;
    // let nmag = (normal_color.xyz - vec3(0.5, 0.5, 0.0)) * 2.0 * detail_material.blend + vec3(0.5, 0.5, 0.0);
    // pbr_input.N = normalize(pbr_input.N * (1.0 - detail_material.blend) + nmag);
    // pbr_input.N = normalize(pbr_input.N);

    let n1 = vec3(pbr_input.N);
    let n2 = vec3(normal_color.xyz);


    let r0 = (n1 * detail_material.blend) + (n2 * (1.0 -  detail_material.blend));
    // Rgb([r[0], r[1], r[2]])
    let r = vec3(
        // clamp(r0[0] * 0.5, -1.0, 1.0),
        // clamp(r0[1] * 0.5, -1.0, 1.0),
        // clamp(r0[2] * 0.25, -1.0, 1.0),
        clamp(r0[0] * 0.95, 0.0, 1.0),
        clamp(r0[1] * 0.95, 0.0, 1.0),
        clamp(r0[2] * 0.00, 0.0, 1.0),
    );

    // let t = vec3(n1[0] *  2.0 - 1.0, n1[1] *  2.0 - 1.0, n1[2] * 2.0 + 0.0);
    // let u = vec3(n2[0] * -2.0 + 1.0, n2[1] * -2.0 + 1.0, n2[2] * 2.0 - 1.0);
    // let v = t * dot(t, u) - u * t[2];
    // let r = normalize(v) * detail_material.blend + 0.5;

    // let n3 = vec3(0.5, normal_color.y, 0.5);

    // // https://docs.gimp.org/2.10/en/layer-mode-group-contrast.html
    // // overlay the red (X) and green (Y) components of the normal map in order
    // // to come up with a greyscale representation of the "effect" of the normal
    // let norm_x = vec3(n2[0], n2[0], n2[0]);
    // let norm_y = vec3(n2[1], n2[1], n2[1]);
    // let norm_ovl = overlay(norm_x, norm_y);

    // // let r = overlay(n1, norm_ovl);

    // // move the overlay values around 0.5 (e.g. to reduce strength)
    // let adj = (norm_ovl - 0.5) * detail_material.blend + 0.5;

    // let r = overlay(n1, adj);

    pbr_input.N = r;


    // apply lighting
    out.color = apply_pbr_lighting(pbr_input);

    // pbr_input.N = normalize(pbr_input.N * detail_material.scale);
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
