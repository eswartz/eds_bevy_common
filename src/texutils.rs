use bevy::{
    prelude::*,
    asset::RenderAssetUsages,
    render::{
        render_resource::{Extent3d, TextureDimension},
    },
};
use bevy::{image::TextureFormatPixelInfo};
use image::{imageops::FilterType, DynamicImage, ImageBuffer};
use wgpu::TextureViewDescriptor;
// use noise::{NoiseFn, Perlin};
use thiserror::Error;

// /// Create an animated sprite sheet texture.
// ///
// /// Create a texture composed of individual sprites of size `size` pixels,
// /// arranged into a grid of `grid` size into the texture image. The final image
// /// has a pixel size of `size * grid`.
// ///
// /// The texture is based on a 3D Perlin noise scaled with `scale.xy`. Each
// /// sprite is a layer at a different height, scaled by `scale.z`, giving the
// /// impression of animation.
// ///
// /// This produces an R8Unorm texture where the R component is equal to the
// /// opacity, to be used with the [`ImageSampleMapping::ModulateOpacityFromR`]
// /// mode of the [`ParticleTextureModifier`].
// ///
// /// This code is a utility for examples. It's nowhere near efficient or clean as
// /// could be for production.
// pub fn make_anim_img(size: UVec2, grid: UVec2, scale: Vec3) -> Image {
//     let w = Perlin::new(42);
//     let tile_cols = size.x as usize;
//     let tile_rows = size.y as usize;
//     let grid_cols = grid.x as usize;
//     let grid_rows = grid.y as usize;
//     let tex_cols = tile_cols * grid_cols;
//     let tex_rows = tile_rows * grid_rows;
//     let tex_len = tex_cols * tex_rows * 4;
//     let mut data = vec![0; tex_len];
//     let mut k = 0.;
//     let dk = scale.z as f64;
//     for v in 0..grid.y as usize {
//         let index0 = v * tex_cols * tile_rows;
//         for u in 0..grid.x as usize {
//             let index1 = index0 + u * tile_cols;
//             for j in 0..size.y as usize {
//                 let index2 = index1 + j * tex_cols;
//                 for i in 0..size.x as usize {
//                     let index3 = (index2 + i) * 4;
//                     let pt = Vec2::new(i as f32 * scale.x, j as f32 * scale.y);
//                     let value = w.get([pt.x as f64, pt.y as f64, k]) * 256.; // * (1.0 - falloff as f64);
//                     let height = (value as u32).clamp(0, 255) as u8;
//                     data[index3] = 255;
//                     data[index3 + 1] = 255;
//                     data[index3 + 2] = 255;
//                     data[index3 + 3] = height;
//                 }
//             }
//             k += dk;
//         }
//     }
//     Image::new(
//         Extent3d {
//             width: tex_cols as u32,
//             height: tex_rows as u32,
//             depth_or_array_layers: 1,
//         },
//         TextureDimension::D2,
//         data,
//         TextureFormat::Rgba8Unorm,
//         RenderAssetUsages::RENDER_WORLD,
//     )
// }


#[derive(Debug, Error)]
#[allow(unused)]
pub enum ImageError {
    #[error("no data in image")]
    NoData,
    #[error("bad image size for cubemap: {0}")]
    BadSize(String),
    #[error("texture error: {0}")]
    TextureError(bevy::image::TextureAccessError),
}

/// This selects the ordering of frames in a cubemap
/// format image strip (see [Self::convert_strip_to_cubemap]).
#[derive(Debug, Clone, Copy, Reflect, Default)]
#[reflect(Clone)]
#[allow(non_camel_case_types)]
#[type_path = "game"]
pub enum CubemapMapping {
    /// -X, +X, -Y, +Y, -Z, +Z
    None,
    /// +X, -X, +Y (flipped on X and Y), -Y, +Z, -Z
    From1_0_2f_3r_4_5,
    /// +X, -X, +Y (flipped on X and Y), -Y (flipped on X and Y), +Z, -Z
    #[default]
    From1_0_2f_3f_4_5,
}

/// Workhorse of the mapper.
struct CubeTextureMapper<'a> {
// pub fn convert_strip_to_cubemap(image: &bevy::image::Image, mapping: CubemapMapping) -> Result<Image, ImageError> {
    image: &'a Image,
    mapping:  CubemapMapping,
    pixel_size: usize,
    in_image_data: &'a Vec<u8>,
    out_image_data: Vec<u8>,
}

impl<'a> CubeTextureMapper<'a> {
    pub fn new(image: &'a bevy::image::Image, mapping: CubemapMapping) -> Result<Self, ImageError> {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let pixel_size = image.texture_descriptor.format.pixel_size().map_err(|e| ImageError::TextureError(e))?;

        // Images are stacked in a long vertical strip.
        if width * 6 != height {
            return Err(ImageError::BadSize(format!("size of image is {}x{}, expecting Nx(N*6)", width, height)));
        }

        // Copy out the data, face-by-face.
        let in_image_data = image.data.as_ref().ok_or(ImageError::NoData)?;

        let side_width = width;
        let side_height = width;
        let side_row_stride = side_width * pixel_size;
        let side_byte_size = side_height * side_row_stride;
        let out_image_data = Vec::<u8>::with_capacity(side_byte_size);

        assert_eq!(side_byte_size * 6, in_image_data.len());

        Ok(Self {
            image,
            mapping,
            pixel_size,
            in_image_data,
            out_image_data,
        })
    }

}

/// Process a 6-cube-sided image into a cubemap texture.
///
/// When finding an equirectangular image on the internet,
/// convert first like so, using the `openexr` package:
///
/// ```bash
/// exrenvmap [-w 1200 | ... ] input.exr output.exr
/// ```
///
/// Attempt to convert the content of a given `image` to a cubemap texture
/// for use with a skybox. That image is expected to be in Rgba32F format
/// and be N by N*6 pixels, laying out images in order dictated by `mapping`.
///
pub fn convert_strip_to_cubemap(image: &bevy::image::Image, mapping: CubemapMapping) -> Result<Image, ImageError> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let pixel_size = image.texture_descriptor.format.pixel_size().map_err(|e| ImageError::TextureError(e))?;

    // Images are stacked in a long vertical strip.
    if width * 6 != height {
        return Err(ImageError::BadSize(format!("size of image is {}x{}, expecting Nx(N*6)", width, height)));
    }

    // Copy out the data, face-by-face.
    let in_image_data = image.data.as_ref().ok_or(ImageError::NoData)?;

    let side_width = width;
    let side_height = width;    // yes, a square
    let side_row_stride = side_width * pixel_size;
    let side_byte_size = side_height * side_row_stride;
    let mut out_image_data = Vec::<u8>::with_capacity(side_byte_size);

    assert_eq!(side_byte_size * 6, in_image_data.len());

    // dbg!((side_width, side_height, side_row_stride, side_byte_size));

    // Write everything from input to output as-is.
    let write_all = |out_image_data: &mut Vec<u8>| {
        out_image_data.extend_from_slice(&in_image_data[..]);
    };
    // Write an entire side (e.g. `side_byte_size * N`) from input to output as-is.
    let write_side = |out_image_data: &mut Vec<u8>, side_offset| {
        out_image_data.extend_from_slice(&in_image_data[side_offset..side_offset + side_byte_size]);
    };
    match mapping {
        CubemapMapping::None => {
            write_all(&mut out_image_data);
        }
        CubemapMapping::From1_0_2f_3r_4_5 => {
            {
                // +X side.
                let in_plus_x = side_byte_size;
                write_side(&mut out_image_data, in_plus_x);

            }
            {
                // -X side.
                let in_minus_x = 0;
                write_side(&mut out_image_data, in_minus_x);
            }
            {
                // +Y side, but flipped on both axes.
                let in_plus_y = side_byte_size * 2;
                write_side_map(
                    in_image_data, side_width, side_height, pixel_size,
                    &mut out_image_data, in_plus_y, &|col, row| {
                        (side_height - row - 1, side_width - col - 1)
                    }
                );
            }
            {
                // -Y side, but rotated.
                let in_minus_y = side_byte_size * 3;
                write_side_map(
                    in_image_data, side_width, side_height, pixel_size,
                    &mut out_image_data, in_minus_y, &|col, row| {
                        (side_height - col - 1, side_width - row - 1)
                    }
                );
            }
            {
                // +Z side.
                let in_plus_z = side_byte_size * 4;
                write_side(&mut out_image_data, in_plus_z);
            }
            {
                // -Z side.
                let in_minus_z = side_byte_size * 5;
                write_side(&mut out_image_data, in_minus_z);
            }
        }
        CubemapMapping::From1_0_2f_3f_4_5 => {
            {
                // +X side.
                let in_plus_x = side_byte_size;
                out_image_data.extend_from_slice(&in_image_data[in_plus_x..in_plus_x + side_byte_size]);
            }
            {
                // -X side.
                let in_minus_x = 0;
                out_image_data.extend_from_slice(&in_image_data[in_minus_x..in_minus_x + side_byte_size]);
            }
            {
                // +Y side, but flipped on both axes.
                let in_plus_y = side_byte_size * 2;
                for row in 0..side_height {
                    for col in 0..side_width {
                        let pixel_offs = in_plus_y + (side_height - row - 1) * side_row_stride + (side_width - col - 1) * pixel_size;
                        out_image_data.extend_from_slice(&in_image_data[pixel_offs..pixel_offs + pixel_size]);
                    }
                }
            }
            {
                // -Y side, but flipped on both axes.
                let in_minus_y = side_byte_size * 3;
                for row in 0..side_height {
                    for col in 0..side_width {
                        let pixel_offs = in_minus_y + (side_height - row - 1) * side_row_stride + (side_width - col - 1) * pixel_size;
                        out_image_data.extend_from_slice(&in_image_data[pixel_offs..pixel_offs + pixel_size]);
                    }
                }
            }
            {
                // +Z side.
                let in_plus_z = side_byte_size * 4;
                out_image_data.extend_from_slice(&in_image_data[in_plus_z..in_plus_z + side_byte_size]);
            }
            {
                // -Z side.
                let in_minus_z = side_byte_size * 5;
                out_image_data.extend_from_slice(&in_image_data[in_minus_z..in_minus_z + side_byte_size]);
            }
        }
    }

    assert_eq!(out_image_data.len(), in_image_data.len());

    let mut cubemap_image = Image::new(
        Extent3d {
            width: side_width as u32,
            height: side_height as u32,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        out_image_data,
        image.texture_descriptor.format,
        RenderAssetUsages::all(),
    );

    // Mark as cubemap.
    // (This is crucial for ensuring the Skybox shader will accept this image.)
    cubemap_image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::Cube),
        ..default()
    });

    Ok(cubemap_image)
}

// Write an entire side by transforming (e.g. `side_byte_size * N`) from input to output as-is.
// The `map` function maps the input (x,y) to the output (x,y).
pub fn write_side_map (in_image_data: &Vec<u8>, side_width: usize, side_height: usize, pixel_size: usize,
    out_image_data: &mut Vec<u8>, side_offset: usize, map_x_y: impl Fn(usize, usize) -> (usize, usize)) {
    let side_row_stride = side_width * pixel_size;
    let side_byte_size = side_height * side_row_stride;
    for o_row in 0..side_height {
        for o_col in 0..side_width {
            let (i_col, i_row) = map_x_y(o_col, o_row);
            // Copy the pixel... yes, just one pixel here.
            let pixel_offs = side_offset + i_row * side_row_stride + i_col * pixel_size;
            // Wrap to buffer to avoid panic (so we see the error)
            let pixel_offs = pixel_offs % side_byte_size;
            out_image_data.extend_from_slice(&in_image_data[pixel_offs..pixel_offs + pixel_size]);
        }
    }
}

pub fn resize_for_quality(input_image: &Image, new_width: u32, new_height: u32, filter: FilterType) -> Option<DynamicImage>
{
    let width = input_image.width();
    let height = input_image.height();
    if input_image.width() != new_width || input_image.height() != new_height {
        // Manually convert since Bevy image only supports 8-bit formats.
        let Some(data) = input_image.data.clone() else {
            warn!("cannot convert uninitialized skybox");
            return None
        };
        let bytesfmt: &[f32] = bytemuck::cast_slice(&data[..]);
        let vecfmt: Vec<f32> = bytesfmt.to_vec();
        let dyn_image_fmt = DynamicImage::ImageRgba32F;
        let Some(dyn_image) = ImageBuffer::from_raw(width, height, vecfmt).map(dyn_image_fmt) else {
            warn!("cannot convert skybox image");
            return None
        };
        Some(dyn_image.resize(new_width, new_height, filter))
        // &Image::from_dynamic(
        //     dyn_image.resize(side_res, side_res * 6, image::imageops::FilterType::CatmullRom),
        //     true,
        //     RenderAssetUsages::RENDER_WORLD
        // )
    } else {
        None
    }
}
