#import bevy_pbr::{
    pbr_types,
    pbr_functions,
    pbr_bindings,
    pbr_functions::SampleBias,
    mesh_bindings::mesh,
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
    decal::clustered::apply_decals,
    pbr_functions::prepare_world_normal,
    pbr_functions::apply_normal_mapping,
    pbr_functions::calculate_tbn_mikktspace,
    self,
    forward_io::Vertex,
}

#import bevy_render::bindless::{bindless_samplers_filtering, bindless_textures_2d}


#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

#ifdef VISIBILITY_RANGE_DITHER
#import bevy_pbr::pbr_functions::visibility_range_dither;
#endif

#ifdef MESHLET_MESH_MATERIAL_PASS
#import bevy_pbr::meshlet_visibility_buffer_resolve::resolve_vertex_output
#endif

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif // OIT_ENABLED

#ifdef FORWARD_DECAL
#import bevy_pbr::decal::forward::get_forward_decal_info
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

@group(#{MATERIAL_BIND_GROUP}) @binding(102) var detail_normal_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var detail_normal_texture: texture_2d<f32>;

@fragment
fn fragment(
#ifdef MESHLET_MESH_MATERIAL_PASS
    @builtin(position) frag_coord: vec4<f32>,
#else
    vertex_output: VertexOutput,
    @builtin(front_facing) is_front: bool,
#endif
) -> FragmentOutput {
#ifdef MESHLET_MESH_MATERIAL_PASS
    let vertex_output = resolve_vertex_output(frag_coord);
    let is_front = true;
#endif

    var in = vertex_output;

    // If we're in the crossfade section of a visibility range, conditionally
    // discard the fragment according to the visibility pattern.
#ifdef VISIBILITY_RANGE_DITHER
    visibility_range_dither(in.position, in.visibility_range_dither);
#endif

#ifdef FORWARD_DECAL
    let forward_decal_info = get_forward_decal_info(in);
    in.world_position = forward_decal_info.world_position;
    in.uv = forward_decal_info.uv;
#endif

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    //[ejs:

    // Apply the detail normal to the VertexOutput munging `world_normal`.
    if detail_material.blend != 0. {
        // We need to recompute the normal, meaning we redo most of the steps from
        // `pbr_input_from_standard_material` but making use of the detail normal to
        // tweak the `pbr_input.N` that eventually goes out.

#ifdef MESHLET_MESH_MATERIAL_PASS
    let slot = in.material_bind_group_slot;
#else   // MESHLET_MESH_MATERIAL_PASS
    let slot = mesh[in.instance_index].material_and_lightmap_bind_group_slot & 0xffffu;
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
    let flags = pbr_bindings::material_array[material_indices[slot].material].flags;
    let base_color = pbr_bindings::material_array[material_indices[slot].material].base_color;
    let deferred_lighting_pass_id =
        pbr_bindings::material_array[material_indices[slot].material].deferred_lighting_pass_id;
#else   // BINDLESS
    let flags = pbr_bindings::material.flags;
    let base_color = pbr_bindings::material.base_color;
    let deferred_lighting_pass_id = pbr_bindings::material.deferred_lighting_pass_id;
#endif

        let double_sided = (flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

        let world_normal = pbr_functions::prepare_world_normal(
            in.world_normal,
            double_sided,
            is_front,
        );

#ifdef BINDLESS
    let uv_transform = pbr_bindings::material_array[material_indices[slot].material].uv_transform;
#else   // BINDLESS
    let uv_transform = pbr_bindings::material.uv_transform;
#endif  // BINDLESS

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS
#ifdef STANDARD_MATERIAL_NORMAL_MAP

#ifdef VERTEX_UVS_A
    var uv = (uv_transform * vec3(in.uv, 1.0)).xy;
#endif

// TODO: Transforming UVs mean we need to apply derivative chain rule for meshlet mesh material pass
#ifdef VERTEX_UVS_B
    var uv_b = (uv_transform * vec3(in.uv_b, 1.0)).xy;
#else
    var uv_b = uv;
#endif

        // Fill in the sample bias so we can sample from textures.
        var bias: SampleBias;
#ifdef MESHLET_MESH_MATERIAL_PASS
        bias.ddx_uv = in.ddx_uv;
        bias.ddy_uv = in.ddy_uv;
#else   // MESHLET_MESH_MATERIAL_PASS
        // bias.mip_bias = view.mip_bias;
        bias.mip_bias = 0.;     // ?  mip_bias: mip_bias.unwrap_or(&MipBias(0.0)).0,  from bevy_render/src/view/mod.rs line 993
#endif  // MESHLET_MESH_MATERIAL_PASS

        let tbn = pbr_functions::calculate_tbn_mikktspace(world_normal, in.world_tangent);

        let base_nt =
#ifdef MESHLET_MESH_MATERIAL_PASS
            textureSampleGrad(
#else   // MESHLET_MESH_MATERIAL_PASS
            textureSampleBias(
#endif  // MESHLET_MESH_MATERIAL_PASS
#ifdef BINDLESS
                bindless_textures_2d[material_indices[slot].normal_map_texture],
                bindless_samplers_filtering[material_indices[slot].normal_map_sampler],
#else   // BINDLESS
                pbr_bindings::normal_map_texture,
                pbr_bindings::normal_map_sampler,
#endif  // BINDLESS
                uv,
#ifdef MESHLET_MESH_MATERIAL_PASS
                bias.ddx_uv,
                bias.ddy_uv,
#else   // MESHLET_MESH_MATERIAL_PASS
                bias.mip_bias,
#endif  // MESHLET_MESH_MATERIAL_PASS
            ).xyz;

        let base_normal = normalize(base_nt.xyz * 2.0 - vec3(1.0));

        // Modify normal using detail normal texture.
        let detail_nt = textureSample(
            detail_normal_texture,
            detail_normal_sampler,
            uv * detail_material.scale,
        );
        // from e.g. prepass_utils.wgsl: prepass_normal
        let detail_normal = normalize(detail_nt.xyz * 2.0 - vec3(1.0));

        let tweaked_normal = normalize(base_normal + detail_normal * detail_material.blend);
        let tweaked_nt = (tweaked_normal + vec3(1.0)) * 0.5;

        var normal = pbr_functions::apply_normal_mapping(
            flags,
            tbn,
            double_sided,
            is_front,
            tweaked_nt,
        );

#endif  // STANDARD_MATERIAL_NORMAL_MAP
#endif  // VERTEX_TANGENTS
#endif  // VERTEX_UVS

        pbr_input.N = normal;
    }

    //] ejs

    // clustered decals
    apply_decals(&pbr_input);

#ifdef PREPASS_PIPELINE
    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
    let out = deferred_output(in, pbr_input);
#else
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

#ifdef OIT_ENABLED
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
#endif // OIT_ENABLED

#ifdef FORWARD_DECAL
    out.color.a = min(forward_decal_info.alpha, out.color.a);
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
