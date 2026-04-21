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
use thiserror::Error;

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
    image: &'a Image,
    mapping:  CubemapMapping,
    in_image_data: &'a Vec<u8>,
    out_image_data: Vec<u8>,
    pixel_size: usize,
    side_width: usize,
    side_height: usize,
    side_row_stride: usize,
    pub(crate) side_byte_size: usize,
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
            side_width,
            side_height,
            side_row_stride,
            side_byte_size,
        })
    }

    // Write everything from input to output as-is.
    pub fn write_all(&mut self) {
        self.out_image_data.extend_from_slice(&self.in_image_data[..]);
    }

    // Write an entire side (e.g. `side_byte_size * N`) from input to output as-is.
    pub fn write_side(&mut self, side_offset: usize) {
        let out_image_data = Vec::<u8>::with_capacity(self.side_byte_size);
        self.out_image_data.extend_from_slice(&self.in_image_data[side_offset..side_offset + self.side_byte_size]);
    }

    // Write an entire side by transforming (e.g. `side_byte_size * N`) from input to output as-is.
    // The `map` function maps the input (x,y) to the output (x,y).
    pub fn write_side_map(&mut self, side_offset: usize, map_x_y: impl Fn(usize, usize) -> (usize, usize)) {
        let width = self.image.width() as usize;
        let height = self.image.height() as usize;

        for o_row in 0..self.side_height {
            for o_col in 0..self.side_width {
                let (i_col, i_row) = map_x_y(o_col, o_row);
                // Copy the pixel... yes, just one pixel here.
                let pixel_offs = side_offset + i_row * self.side_row_stride + i_col * self.pixel_size;
                // Wrap to buffer to avoid panic (so we see the error)
                let pixel_offs = pixel_offs % self.side_byte_size;
                self.out_image_data.extend_from_slice(&self.in_image_data[pixel_offs..pixel_offs + self.pixel_size]);
            }
        }
    }

    fn extract_cubemap(self) -> Image {
        assert_eq!(self.out_image_data.len(), self.in_image_data.len());

        let mut cubemap_image = Image::new(
            Extent3d {
                width: self.side_width as u32,
                height: self.side_height as u32,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            self.out_image_data,
            self.image.texture_descriptor.format,
            RenderAssetUsages::all(),
        );

        // Mark as cubemap.
        // (This is crucial for ensuring the Skybox shader will accept this image.)
        cubemap_image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..default()
        });

        cubemap_image
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

    let mut mapper = CubeTextureMapper::new(image, mapping)?;
    let side_byte_size = mapper.side_byte_size;
    let side_width = mapper.side_width;
    let side_height = mapper.side_height;

    match mapping {
        CubemapMapping::None => {
            mapper.write_all();
        }
        CubemapMapping::From1_0_2f_3r_4_5 => {
            // +X side.
            let in_plus_x = side_byte_size;
            mapper.write_side(in_plus_x);

            // -X side.
            let in_minus_x = 0;
            mapper.write_side(in_minus_x);

            // +Y side, but flipped on both axes.
            let in_plus_y = side_byte_size * 2;
            mapper.write_side_map(in_plus_y, &|col, row| {
                    (side_height - row - 1, side_width - col - 1)
            });

            // -Y side, but rotated.
            let in_minus_y = side_byte_size * 3;
            mapper.write_side_map(in_minus_y, &|col, row| {
                (side_height - col - 1, side_width - row - 1)
            });

            // +Z side.
            let in_plus_z = side_byte_size * 4;
            mapper.write_side(in_plus_z);

            // -Z side.
            let in_minus_z = side_byte_size * 5;
            mapper.write_side(in_minus_z);
        }
        CubemapMapping::From1_0_2f_3f_4_5 => {
            // +X side.
            let in_plus_x = side_byte_size;
            mapper.write_side(in_plus_x);

            // -X side.
            let in_minus_x = 0;
            mapper.write_side(in_minus_x);

            // +Y side, but flipped on both axes.
            let in_plus_y = side_byte_size * 2;
            mapper.write_side_map(in_plus_y, &|col, row| {
                (side_width - col - 1, side_height - row - 1)
            });

            // -Y side, but flipped on both axes.
            let in_minus_y = side_byte_size * 3;
            mapper.write_side_map(in_minus_y, &|col, row| {
                (side_width - col - 1, side_height - row - 1)
            });

            // +Z side.
            let in_plus_z = side_byte_size * 4;
            mapper.write_side(in_plus_z);

            // -Z side.
            let in_minus_z = side_byte_size * 5;
            mapper.write_side(in_minus_z);
        }
    }

    let cubemap_image = mapper.extract_cubemap();

    Ok(cubemap_image)
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
    } else {
        None
    }
}
