// Detail normal mapping.
//
// Driven from `AssignDetailNormal` component, which is realized into
// `CustomMaterialNormalExtension` at runtime, which then drives this.
import bevy_pbr::{
    render::{
        pbr_types,
        pbr_functions::alpha_discard,
        pbr_fragment::pbr_input_from_vertex_output,
       pbr_fragment::pbr_input_from_standard_material,
    },
    decal::clustered::apply_decals,
};
import bevy_pbr::render::forward_io;

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::FragmentOutput,
    deferred::functions::deferred_output,
    parallax_mapping::parallax_uv,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};
@else
import bevy_pbr::render::{
    parallax_mapping::parallax_uv,
    forward_io::{FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};

@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

@if(MESHLET_MESH_MATERIAL_PASS)
import bevy_pbr::meshlet::visibility_buffer_resolve::resolve_vertex_output;

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;

@if(FORWARD_DECAL)
import bevy_pbr::decal::forward::get_forward_decal_info;

import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d};

import bevy_pbr::{
    render::{
        pbr_functions,
        pbr_functions::SampleBias,
        pbr_bindings,
        pbr_lighting as lighting,
        mesh_bindings::mesh,
        mesh_view_bindings::view,
        parallax_mapping::parallaxed_uv,
    },
    prepass::utils as prepass_utils,
    lightmap::lightmap,
};

@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::render::mesh_view_bindings::screen_space_ambient_occlusion_texture;
@if(SCREEN_SPACE_AMBIENT_OCCLUSION)
import bevy_pbr::ssao::utils::ssao_multibounce;

@if(MESHLET_MESH_MATERIAL_PASS)
import bevy_pbr::meshlet::visibility_buffer_resolve::VertexOutput;
@elif(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::VertexOutput;
@else
import bevy_pbr::render::forward_io::VertexOutput;

@if(BINDLESS)
import bevy_pbr::render::pbr_bindings::material_indices;

//[ ejs
struct DetailExtendedNormal {
    scale: vec2<f32>,
    blend: f32,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> detail_material: DetailExtendedNormal;

@group(constants::MATERIAL_BIND_GROUP) @binding(102) var detail_normal_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var detail_normal_texture: texture_2d<f32>;

fn blend_linear(n1: vec3<f32>, n2: vec3<f32>, blend: f32) -> vec3<f32>
{
    let r = n1 + n2 * blend;
    return normalize(r);
}

fn blend_whiteout(n1: vec3<f32>, n2: vec3<f32>, blend: f32) -> vec3<f32>
{
    let r = vec3<f32>(n1.xy + n2.xy * blend, n1.z * n2.z * blend);
    return normalize(r);
}

fn blend_udn(n1: vec3<f32>, n2: vec3<f32>, blend: f32) -> vec3<f32>
{
    let r = vec3<f32>(n1.xy + n2.xy * blend, n1.z);
    return normalize(r);
}

fn blend_rnm(n1: vec3<f32>, n2: vec3<f32>, blend: f32) -> vec3<f32>
{
    let t = vec3<f32>(n1.xy, n1.z + 1);
    let u = vec3<f32>(-n2.xy * blend, n2.z);
    let r = t * dot(t, u) - u * t.z;
    return normalize(r);
}

fn apply_normal_mapping_with_detail(
    standard_material_flags: u32,
    TBN: mat3x3<f32>,
    double_sided: bool,
    is_front: bool,
    in_Nt1: vec3<f32>,
    in_Nt2: vec3<f32>,
    blend: f32,
) -> vec3<f32> {
    // Unpack the TBN vectors.
    var T = TBN[0];
    var B = TBN[1];
    var N = TBN[2];

    // Nt is the tangent-space normal.
    var Nt1 = in_Nt1;
    if (standard_material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0u {
        // Only use the xy components and derive z for 2-component normal maps.
        Nt1 = vec3<f32>(Nt1.rg * 2.0 - 1.0, 0.0);
        Nt1.z = sqrt(1.0 - Nt1.x * Nt1.x - Nt1.y * Nt1.y);
    } else {
        Nt1 = Nt1 * 2.0 - 1.0;
    }
    // Normal maps authored for DirectX require flipping the y component
    if (standard_material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_FLIP_NORMAL_MAP_Y) != 0u {
        Nt1.y = -Nt1.y;
    }

    var Nt2 = in_Nt2;
    if false && (standard_material_flags & pbr_types::STANDARD_MATERIAL_FLAGS_TWO_COMPONENT_NORMAL_MAP) != 0u {
        // Only use the xy components and derive z for 2-component normal maps.
        Nt2 = vec3<f32>(Nt2.rg * 2.0 - 1.0, 0.0);
        Nt2.z = sqrt(1.0 - Nt2.x * Nt2.x - Nt2.y * Nt2.y);
    } else {
        Nt2 = (Nt2 * 2.0 - 1.0);
    }

    if double_sided && !is_front {
        Nt1 = -Nt1;
        Nt2 = -Nt2;
    }

    // let Nt = blend_linear(Nt1, Nt2, blend);
    // let Nt = blend_whiteout(Nt1, Nt2, blend);
    // let Nt = blend_udn(Nt1, Nt2, blend);
    let Nt = blend_rnm(Nt1, Nt2, blend);
    // let Nt = Nt2 * blend;

    // NOTE: The mikktspace method of normal mapping applies maps the tangent-space normal from
    // the normal map texture in this way to be an EXACT inverse of how the normal map baker
    // calculates the normal maps so there is no error introduced. Do not change this code
    // unless you really know what you are doing.
    // http://www.mikktspace.com/

    // (not unwrapping and rewrapping TBN here, so this is always going to be suspect)
    N = Nt.x * T + Nt.y * B + Nt.z * N;

    return normalize(N);
}
//]


@fragment
fn fragment(
@if(MESHLET_MESH_MATERIAL_PASS)
    @builtin(position) frag_coord: vec4<f32>,
@if(!MESHLET_MESH_MATERIAL_PASS)
    vertex_output: VertexOutput,
@if(!MESHLET_MESH_MATERIAL_PASS)
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
@if(MESHLET_MESH_MATERIAL_PASS)
    let vertex_output = resolve_vertex_output(frag_coord);
@if(MESHLET_MESH_MATERIAL_PASS)
    let is_front = true;

    var in = vertex_output;

    // If we're in the crossfade section of a visibility range, conditionally
    // discard the fragment according to the visibility pattern.
@if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position, in.visibility_range_dither);

@if(FORWARD_DECAL)
    let forward_decal_info = get_forward_decal_info(in);
@if(FORWARD_DECAL)
    in.world_position = forward_decal_info.world_position;
@if(FORWARD_DECAL)
    in.uv = forward_decal_info.uv;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color
    );

    //[ejs:

    // Apply the detail normal to the VertexOutput munging `world_normal`.
    if detail_material.blend != 0. {

        // Fill in the sample bias so we can sample from textures.
        var bias: SampleBias;
    @if(MESHLET_MESH_MATERIAL_PASS) {
        // Apply mip bias by scaling the gradients used by textureSampleGrad.
        let mip_scale = exp2(view.mip_bias);
        bias.ddx_uv = in.ddx_uv * mip_scale;
        bias.ddy_uv = in.ddy_uv * mip_scale;
    }
    @else {
        bias.mip_bias = view.mip_bias;
    }

        let flags = pbr_input.material.flags;
        let double_sided = (pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;
        let is_logically_front = pbr_functions::winding_corrected_front_facing(pbr_input.flags, is_front);

    @if(VERTEX_UVS && BINDLESS)
        let uv_transform = pbr_bindings::material_array[material_indices[slot].material].uv_transform;
    @elif(VERTEX_UVS)
        let uv_transform = pbr_bindings::material.uv_transform;

    @if(VERTEX_UVS && VERTEX_UVS_A)
        var uv = (uv_transform * vec3(in.uv, 1.0)).xy;

    // TODO: Transforming UVs mean we need to apply derivative chain rule for meshlet mesh material pass
    @if(VERTEX_UVS && VERTEX_UVS_B)
        var uv_b = (uv_transform * vec3(in.uv_b, 1.0)).xy;
    @elif(VERTEX_UVS)
        var uv_b = uv;


        // N (normal vector)
// @if(!LOAD_PREPASS_NORMALS) {     // need to override

        pbr_input.N = normalize(pbr_input.world_normal);

@if(VERTEX_UVS && VERTEX_TANGENTS)
        let TBN = pbr_functions::calculate_tbn_mikktspace(pbr_input.world_normal, in.world_tangent);

@if(VERTEX_UVS && VERTEX_TANGENTS && STANDARD_MATERIAL_NORMAL_MAP) {

        @if(BINDLESS)
        let texture = bindless_textures_2d[material_indices[slot].normal_map_texture];
        @else
        let texture = pbr_bindings::normal_map_texture;
        @if(BINDLESS)
        let sampler_ = bindless_samplers_filtering[material_indices[slot].normal_map_sampler];
        @else
        let sampler_ = pbr_bindings::normal_map_sampler;
        @if(STANDARD_MATERIAL_NORMAL_MAP_UV_B)
        let coords = uv_b;
        @else
        let coords = uv;
        @if(MESHLET_MESH_MATERIAL_PASS) {
        let Nt1 = textureSampleGrad(texture, sampler_, coords, bias.ddx_uv, bias.ddy_uv).rgb;
        let Nt2 = textureSample(detail_normal_texture, detail_normal_sampler, coords).rgb;
        } @else {
        let Nt1 = textureSampleBias(texture, sampler_, coords, bias.mip_bias).rgb;
        let Nt2 = textureSample(detail_normal_texture, detail_normal_sampler, coords * detail_material.scale).rgb;
        }

        // let Nt = blend_linear(Nt1, Nt2);
        // let Nt = blend_udn(Nt1, Nt2);
        //let Nt = blend_rnm(Nt1 * 2 - 1, Nt2 * 2 - 1) * 0.5 + 1;
        // let Nt = Nt1;

        pbr_input.N = apply_normal_mapping_with_detail(
            flags,
            TBN,
            double_sided,
            is_logically_front,
            Nt1,
            Nt2,
            detail_material.blend,
        );

    }

    }

    //]

    // clustered decals
    apply_decals(&pbr_input);

    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
@if(PREPASS_PIPELINE)
    let out = deferred_output(in, pbr_input);
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
@if(!PREPASS_PIPELINE)
    var out: FragmentOutput;
@if(!PREPASS_PIPELINE)
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
@if(!PREPASS_PIPELINE)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

@if(MATERIAL_OIT_ENABLED) {
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
        discard;
    }
    // Both `Premultiplied` and `Add` colors are premultiplied in `premultiply_alpha()`
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED
        || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
}

@if(FORWARD_DECAL)
        out.color.a = min(forward_decal_info.alpha, out.color.a);

        return out;
}
