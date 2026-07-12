use bevy::prelude::*;
use bevy_seedling::sample::AudioSample;
use crate::SurfaceMaterial;
use rustc_hash::FxHashMap;

use super::CommonFxAssets;

pub fn sounds_for_surface_impact(fx: &CommonFxAssets) -> FxHashMap<SurfaceMaterial, Vec<Handle<AudioSample>>> {
    let unk_synth = &[
        &fx.thump_1a,
        &fx.thump_2a,
        &fx.thump_3a,
        &fx.thump_4a,
        &fx.thump_5a,
        &fx.thump_6a,
        &fx.thump_7a,
        &fx.thump_8a,
        &fx.thump_8b,
        &fx.thump_8c,
        &fx.thump_8d,
        &fx.thump_8g,
        &fx.thump_8h,
        &fx.thump_8i,
        &fx.thump_8j,
    ];

    let water = &[
        &fx.water_1a,
        &fx.water_1b,
        &fx.water_1c,
        &fx.water_1d,
        &fx.water_1e,
        &fx.water_1f,
        &fx.water_1g,
        &fx.water_1h,
        &fx.water_1i,
        &fx.water_1j,
        &fx.water_1k,
        &fx.water_1l,
    ];

    let wood = &[
        &fx.wood1a,
        &fx.wood1b,
        &fx.wood1c,
        &fx.wood1d,
        &fx.thump_7a,
        &fx.thump_8e,
        &fx.wood_metal_1a,
        &fx.wood_metal_1b,
        &fx.wood_metal_1e,
        &fx.wood_metal_1g,
        &fx.wood_metal_1h,
        &fx.wood_metal_1j,
        &fx.wood_metal_1l,
        &fx.wood_metal_1m,
        &fx.wood_metal_1n,
        &fx.wood_metal_1o,
        &fx.wood_metal_1q,
    ];

    let metal = &[
        &fx.metal_clang_1a,
        &fx.metal_clang_1b,
        &fx.metal_clang_1c,
        &fx.metal_clang_1d,
        &fx.metal_clang_1e,
        &fx.metal_clang_2a,
        &fx.metal_clang_3a,
        &fx.thump_8f,
        &fx.metal_bowl_1e,
        &fx.metal_bowl_1g,
        &fx.metal_slide_1b,

        &fx.metal_hammer_1b,
        &fx.metal_hammer_1d,
        &fx.metal_hammer_1e,
        &fx.metal_hammer_1f,
        &fx.metal_hammer_1j,

        &fx.metal_1a,
        &fx.metal_1b,
        &fx.metal_1c,
        &fx.metal_1d,
        &fx.metal_1e,
        &fx.metal_1f,
        &fx.metal_1g,
        &fx.metal_1h,
        &fx.metal_1i,
        &fx.metal_1j,
    ];

    let glass = &[
        &fx.glass_1a,
        &fx.glass_1b,
        &fx.glass_1d,
        &fx.glass_1f,
        &fx.glass_1g,
        &fx.glass_1h,
        &fx.glass_1i,
        &fx.glass_1k,
        &fx.glass_1n,
        &fx.glass_1p,
        &fx.glass_1q,
        &fx.glass_1r,
    ];

    let earth = &[
        // &fx.earth_1a,
        &fx.earth_1b,
        &fx.earth_1c,
        &fx.earth_1d,
        &fx.earth_1e,
        &fx.earth_1f,
        // &fx.earth_1g,
        // &fx.earth_1h,
        // &fx.earth_1i,
    ];

    let stone = &[
        &fx.stone_1b,
        &fx.stone_1c,
        &fx.stone_1f,
        &fx.stone_1i,
        &fx.stone_2c,
        &fx.stone_2d,
        &fx.stone_2e,
        &fx.stone_2f,
        &fx.rock_1a,
        &fx.rock_1b,
        &fx.rock_1c,
        &fx.rock_1d,
        &fx.rock_1e,
        &fx.rock_2a,
        &fx.rock_2c,
    ];

    let sand = &[
        &fx.gravel_1c,
        &fx.gravel_1d,
        &fx.gravel_1e,
        &fx.gravel_1j,
        &fx.gravel_1k,
    ];

    let gravel = &[
        &fx.gravel_1b,
        &fx.gravel_1c,
        &fx.gravel_1d,
        &fx.gravel_1f,
        &fx.gravel_1g,
        &fx.gravel_1h,
        &fx.gravel_1i,
    ];

    let fabric = &[
        &fx.gravel_1i,
    ];

    let mut map : FxHashMap::<SurfaceMaterial, Vec<Handle<AudioSample>>> = default();

    let to_vec = |arr: &[&Handle<AudioSample>]| -> Vec<Handle<AudioSample>> {
        arr.iter().map(|h| (*h).clone()).collect::<Vec<_>>()
    };

    map.insert(SurfaceMaterial::Unknown, to_vec(unk_synth));
    map.insert(SurfaceMaterial::Synthetic, to_vec(unk_synth));
    map.insert(SurfaceMaterial::Water, to_vec(water));
    map.insert(SurfaceMaterial::Wood, to_vec(wood));
    map.insert(SurfaceMaterial::Metal, to_vec(metal));
    map.insert(SurfaceMaterial::Glass, to_vec(glass));
    map.insert(SurfaceMaterial::Earth, to_vec(earth));
    map.insert(SurfaceMaterial::Stone, to_vec(stone));
    map.insert(SurfaceMaterial::Sand, to_vec(sand));
    map.insert(SurfaceMaterial::Gravel, to_vec(gravel));
    map.insert(SurfaceMaterial::Fabric, to_vec(fabric));

    map
}

pub fn sounds_for_surface_slide(fx: &CommonFxAssets) -> FxHashMap<SurfaceMaterial, Vec<Handle<AudioSample>>> {
    let unk_synth_fabric = &[
        &fx.brush1a,
        &fx.brush1b,
        &fx.brush1c,
        &fx.brush1d,
        &fx.brush1e,
        &fx.brush1f,
    ];

    let water = &[
        &fx.water_1b,
        &fx.water_1c,
        &fx.water_1f,
        &fx.water_1g,
        &fx.water_1h,
        &fx.water_1i,
        &fx.water_1j,
        &fx.water_1k,
        &fx.water_1l,
    ];

    let wood = &[
        &fx.wood1a,
        &fx.wood1b,
        &fx.wood1c,
        &fx.wood1d,
        &fx.wood_metal_1c,
        &fx.wood_metal_1d,
        &fx.wood_metal_1f,
        &fx.wood_metal_1i,
        &fx.wood_metal_1k,
        &fx.wood_metal_1p,
    ];

    let metal = &[
        &fx.metal_bowl_1a,
        &fx.metal_bowl_1b,
        &fx.metal_bowl_1c,
        &fx.metal_bowl_1d,
        &fx.metal_bowl_1f,
        &fx.metal_slide_1a,

        &fx.metal_hammer_1a,
        &fx.metal_hammer_1c,
        &fx.metal_hammer_1g,
        &fx.metal_hammer_1h,
        &fx.metal_hammer_1i,

        &fx.metal_1k,
        &fx.metal_1l,
        &fx.metal_1m,
        &fx.metal_1n,
        &fx.metal_1o,
        &fx.metal_1p,
        &fx.metal_1q,
    ];

    let glass = &[
        &fx.glass_scrape1a,
        &fx.glass_scrape1b,
        &fx.glass_scrape1c,
        &fx.glass_scrape1d,
        &fx.glass_scrape1e,
        &fx.glass_scrape1f,
        &fx.glass_scrape1g,
        &fx.glass_scrape1h,
        &fx.glass_scrape1i,
        &fx.glass_1c,
        &fx.glass_1e,
        &fx.glass_1j,
        &fx.glass_1l,
        &fx.glass_1m,
        &fx.glass_1o,
        &fx.glass_roll_1a,
        &fx.glass_roll_1b,
        &fx.glass_roll_1c,
        &fx.glass_roll_1d,
        &fx.glass_roll_1e,
    ];

    let earth = &[
        &fx.earth_1a,
        &fx.earth_1b,
        &fx.earth_1g,
        &fx.earth_1h,
        &fx.earth_1i,
    ];

    let stone = &[
        &fx.stone_1a,
        &fx.stone_1d,
        &fx.stone_1e,
        &fx.stone_1g,
        &fx.stone_1i,
        &fx.stone_2c,
        &fx.stone_2d,
        &fx.stone_2e,
        &fx.stone_2f,
        &fx.rock_2b,
        &fx.rock_2d,
    ];

    let sand = &[
        &fx.sand_1a,
        &fx.sand_1b,
        &fx.sand_1f,
        &fx.sand_1i,
    ];

    let gravel = &[
        &fx.gravel_1a,
        &fx.gravel_1b,
        &fx.gravel_1h,
        &fx.gravel_1i,
        &fx.gravel_1k,
        &fx.gravel_1l,
        &fx.footsteps_4a,
        &fx.footsteps_4b,
        &fx.footsteps_4c,
        &fx.footsteps_4d,
        &fx.footsteps_4e,
        &fx.footsteps_4f,
        &fx.footsteps_4g,
        &fx.footsteps_4h,
    ];

    let mut map : FxHashMap::<SurfaceMaterial, Vec<Handle<AudioSample>>> = default();

    let to_vec = |arr: &[&Handle<AudioSample>]| -> Vec<Handle<AudioSample>> {
        arr.iter().map(|h| (*h).clone()).collect::<Vec<_>>()
    };

    map.insert(SurfaceMaterial::Unknown, to_vec(unk_synth_fabric));
    map.insert(SurfaceMaterial::Synthetic, to_vec(unk_synth_fabric));
    map.insert(SurfaceMaterial::Water, to_vec(water));
    map.insert(SurfaceMaterial::Wood, to_vec(wood));
    map.insert(SurfaceMaterial::Metal, to_vec(metal));
    map.insert(SurfaceMaterial::Glass, to_vec(glass));
    map.insert(SurfaceMaterial::Earth, to_vec(earth));
    map.insert(SurfaceMaterial::Stone, to_vec(stone));
    map.insert(SurfaceMaterial::Sand, to_vec(sand));
    map.insert(SurfaceMaterial::Gravel, to_vec(gravel));
    map.insert(SurfaceMaterial::Fabric, to_vec(unk_synth_fabric));

    map
}

pub fn sounds_for_footsteps_impact(fx: &CommonFxAssets) -> FxHashMap<SurfaceMaterial, Vec<Handle<AudioSample>>> {
    let unk_synth = &[
        &fx.footsteps_1a,
        &fx.footsteps_1b,
        &fx.footsteps_1c,
        &fx.footsteps_1d,
        &fx.footsteps_1e,
        &fx.footsteps_1f,
        &fx.footsteps_1g,
        &fx.footsteps_1h,
        &fx.footsteps_1i,
        &fx.footsteps_1j,
        &fx.footsteps_1k,
        &fx.footsteps_1m,
        &fx.footsteps_1n,
        &fx.footsteps_1o,
        &fx.footsteps_1p,
        &fx.footsteps_1q,
        &fx.footsteps_1r,
        &fx.footsteps_1s,
        &fx.footsteps_1t,
        &fx.footsteps_1u,
        &fx.footsteps_1v,
    ];

    let earth = &[
        &fx.footsteps_3a,
        &fx.footsteps_3b,
        &fx.footsteps_3c,
        &fx.footsteps_3d,
        &fx.footsteps_3e,
        &fx.footsteps_3f,
        &fx.footsteps_3g,
        &fx.footsteps_3h,
        &fx.footsteps_3i,
        &fx.footsteps_3j,
    ];

    let water = &[
        &fx.water_1a,
        &fx.water_1b,
        &fx.water_1c,
        &fx.water_1d,
        &fx.water_1e,
        &fx.water_1f,
        &fx.water_1g,
        &fx.water_1h,
        &fx.water_1i,
        &fx.water_1j,
        &fx.water_1k,
        &fx.water_1l,
    ];

    let wood = &[
        &fx.wood1a,
        &fx.wood1b,
        &fx.wood1c,
        &fx.wood1d,
        &fx.thump_7a,
        &fx.thump_8e,
    ];

    let metal = &[
        &fx.brush1a,
        &fx.brush1b,
        &fx.brush1c,
        &fx.brush1d,
        &fx.brush1e,
        &fx.brush1f,

        &fx.metal_1k,
        &fx.metal_1l,
        &fx.metal_1m,
        &fx.metal_1n,
        &fx.metal_1o,
        &fx.metal_1p,
        &fx.metal_1q,
    ];

    let glass = &[
        &fx.footsteps_1a,
        &fx.footsteps_1b,
        &fx.footsteps_1c,
        &fx.footsteps_1d,
        &fx.footsteps_1e,

        &fx.footsteps_5a,
        &fx.footsteps_5b,
        &fx.footsteps_5g,

        &fx.footsteps_6a,
        &fx.footsteps_6b,
        &fx.footsteps_6c,
        &fx.footsteps_6d,
    ];

    let sand = &[
        &fx.gravel_1c,
        &fx.gravel_1d,
        &fx.gravel_1e,
        &fx.gravel_1j,
        &fx.gravel_1k,
    ];

    let gravel = &[
        &fx.gravel_1b,
        &fx.gravel_1c,
        &fx.gravel_1d,
        &fx.gravel_1f,
        &fx.gravel_1g,
        &fx.gravel_1h,
        &fx.gravel_1i,
        &fx.footsteps_4a,
        &fx.footsteps_4b,
        &fx.footsteps_4c,
        &fx.footsteps_4d,
        &fx.footsteps_4e,
        &fx.footsteps_4f,
        &fx.footsteps_4g,
        &fx.footsteps_4h,
    ];

    let stone = &[
        &fx.footsteps_5a,
        &fx.footsteps_5b,
        &fx.footsteps_5c,
        &fx.footsteps_5d,
        &fx.footsteps_5e,
        &fx.footsteps_5f,
        &fx.footsteps_5g,
        &fx.footsteps_5h,
        &fx.footsteps_5i,
    ];

    let fabric = &[
        &fx.footsteps_2a,
        &fx.footsteps_2b,
        &fx.footsteps_2c,
        &fx.footsteps_2d,
        &fx.footsteps_2e,
        &fx.footsteps_2f,
        &fx.footsteps_2g,
        &fx.footsteps_2h,
        &fx.footsteps_2i,
        &fx.footsteps_2j,
        &fx.footsteps_2k,
        &fx.footsteps_2m,
    ];

    let mut map : FxHashMap::<SurfaceMaterial, Vec<Handle<AudioSample>>> = default();

    let to_vec = |arr: &[&Handle<AudioSample>]| -> Vec<Handle<AudioSample>> {
        arr.iter().map(|h| (*h).clone()).collect::<Vec<_>>()
    };

    map.insert(SurfaceMaterial::Unknown, to_vec(unk_synth));
    map.insert(SurfaceMaterial::Synthetic, to_vec(unk_synth));
    map.insert(SurfaceMaterial::Water, to_vec(water));
    map.insert(SurfaceMaterial::Wood, to_vec(wood));
    map.insert(SurfaceMaterial::Metal, to_vec(metal));
    map.insert(SurfaceMaterial::Glass, to_vec(glass));
    map.insert(SurfaceMaterial::Earth, to_vec(earth));
    map.insert(SurfaceMaterial::Stone, to_vec(stone));
    map.insert(SurfaceMaterial::Sand, to_vec(sand));
    map.insert(SurfaceMaterial::Gravel, to_vec(gravel));
    map.insert(SurfaceMaterial::Fabric, to_vec(fabric));

    map
}

pub fn sounds_for_footsteps_slide(fx: &CommonFxAssets) -> FxHashMap<SurfaceMaterial, Vec<Handle<AudioSample>>> {
    let unk_synth_stone = &[
        &fx.footsteps_1a,
        &fx.footsteps_1b,
        &fx.footsteps_1c,
        &fx.footsteps_1d,
        &fx.footsteps_1e,
        &fx.footsteps_1f,
        &fx.footsteps_1g,
        &fx.footsteps_1h,
        &fx.footsteps_1i,
        &fx.footsteps_1j,
        &fx.footsteps_1k,
        &fx.footsteps_1m,
        &fx.footsteps_1n,
        &fx.footsteps_1o,
        &fx.footsteps_1p,
        &fx.footsteps_1q,
        &fx.footsteps_1r,
        &fx.footsteps_1s,
        &fx.footsteps_1t,
        &fx.footsteps_1u,
        &fx.footsteps_1v,
    ];

    let earth = &[
        &fx.footsteps_3a,
        &fx.footsteps_3b,
        &fx.footsteps_3c,
        &fx.footsteps_3d,
        &fx.footsteps_3e,
        &fx.footsteps_3f,
        &fx.footsteps_3g,
        &fx.footsteps_3h,
        &fx.footsteps_3i,
        &fx.footsteps_3j,
    ];

    let water = &[
        &fx.water_1a,
        &fx.water_1b,
        &fx.water_1c,
        &fx.water_1d,
        &fx.water_1e,
        &fx.water_1f,
        &fx.water_1g,
        &fx.water_1h,
        &fx.water_1i,
        &fx.water_1j,
        &fx.water_1k,
        &fx.water_1l,
    ];

    let wood = &[
        &fx.wood1a,
        &fx.wood1b,
        &fx.wood1c,
        &fx.wood1d,
        &fx.thump_7a,
        &fx.thump_8e,
    ];

    let metal = &[
        &fx.brush1a,
        &fx.brush1b,
        &fx.brush1c,
        &fx.brush1d,
        &fx.brush1e,
        &fx.brush1f,

        &fx.metal_1k,
        &fx.metal_1l,
        &fx.metal_1m,
        &fx.metal_1n,
        &fx.metal_1o,
        &fx.metal_1p,
        &fx.metal_1q,
    ];

    let glass = &[
        &fx.footsteps_1a,
        &fx.footsteps_1b,
        &fx.footsteps_1c,
        &fx.footsteps_1d,
        &fx.footsteps_1e,
        &fx.footsteps_1f,
        &fx.footsteps_1g,
        &fx.footsteps_1h,
        &fx.footsteps_1i,
        &fx.footsteps_1j,
        &fx.footsteps_1k,
        &fx.footsteps_1m,
        &fx.footsteps_1n,
        &fx.footsteps_1o,
        &fx.footsteps_1p,
        &fx.footsteps_1q,
        &fx.footsteps_1r,
        &fx.footsteps_1s,
        &fx.footsteps_1t,
        &fx.footsteps_1u,
        &fx.footsteps_1v,

        &fx.glass_scrape1a,
        &fx.glass_scrape1b,
        &fx.glass_scrape1c,
        &fx.glass_scrape1d,
        &fx.glass_scrape1e,
        &fx.glass_scrape1f,
        &fx.glass_scrape1g,
        &fx.glass_scrape1h,
        &fx.glass_scrape1i,
        &fx.glass_1c,
        &fx.glass_1e,
        &fx.glass_1j,
        &fx.glass_1l,
        &fx.glass_1m,
        &fx.glass_1o,
    ];

    let stone = &[
        &fx.stone_1a,
        &fx.stone_1d,
        &fx.stone_1e,
        &fx.stone_1g,
        &fx.stone_1i,
        &fx.stone_2c,
        &fx.stone_2d,
        &fx.stone_2e,
        &fx.stone_2f,
        &fx.rock_2b,
        &fx.rock_2d,
    ];

    let sand = &[
        &fx.gravel_1c,
        &fx.gravel_1d,
        &fx.gravel_1e,
        &fx.gravel_1j,
        &fx.gravel_1k,
    ];

    let gravel = &[
        &fx.gravel_1a,
        &fx.gravel_1b,
        &fx.gravel_1d,
        &fx.gravel_1f,
        &fx.gravel_1g,
        &fx.gravel_1h,
        &fx.gravel_1i,
        &fx.footsteps_4a,
        &fx.footsteps_4b,
        &fx.footsteps_4c,
        &fx.footsteps_4d,
        &fx.footsteps_4e,
        &fx.footsteps_4f,
        &fx.footsteps_4g,
        &fx.footsteps_4h,
    ];

    let fabric = &[
        &fx.footsteps_2a,
        &fx.footsteps_2b,
        &fx.footsteps_2c,
        &fx.footsteps_2d,
        &fx.footsteps_2e,
        &fx.footsteps_2f,
        &fx.footsteps_2g,
        &fx.footsteps_2h,
        &fx.footsteps_2i,
        &fx.footsteps_2j,
        &fx.footsteps_2k,
        &fx.footsteps_2m,
    ];

    let mut map : FxHashMap::<SurfaceMaterial, Vec<Handle<AudioSample>>> = default();

    let to_vec = |arr: &[&Handle<AudioSample>]| -> Vec<Handle<AudioSample>> {
        arr.iter().map(|h| (*h).clone()).collect::<Vec<_>>()
    };

    map.insert(SurfaceMaterial::Unknown, to_vec(unk_synth_stone));
    map.insert(SurfaceMaterial::Synthetic, to_vec(unk_synth_stone));
    map.insert(SurfaceMaterial::Water, to_vec(water));
    map.insert(SurfaceMaterial::Wood, to_vec(wood));
    map.insert(SurfaceMaterial::Metal, to_vec(metal));
    map.insert(SurfaceMaterial::Glass, to_vec(glass));
    map.insert(SurfaceMaterial::Earth, to_vec(earth));
    map.insert(SurfaceMaterial::Stone, to_vec(stone));
    map.insert(SurfaceMaterial::Sand, to_vec(sand));
    map.insert(SurfaceMaterial::Gravel, to_vec(gravel));
    map.insert(SurfaceMaterial::Fabric, to_vec(fabric));

    map
}
