use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    io,
    path::Path,
};

use bevy_ecs::{prelude::*, system::SystemParam};
use image::{io::Reader, DynamicImage, ImageError};
use modula_asset::{AssetId, Assets};
use modula_core::{DeviceRes, PreInit, QueueRes, ScheduleBuilder};
use modula_render::PreDraw;
use wgpu::{
    Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Texture, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// Systems that load textures during [PreDraw], anything that runs in [PreDraw] and needs textures should run after this
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextureLoadSet;

pub fn init_texture_loading(schedule_builder: &mut ScheduleBuilder) {
    modula_asset::init_assets::<Texture>(schedule_builder);
    schedule_builder.add_systems(PreInit, |mut commands: Commands| {
        commands.insert_resource(TextureQueue { queue: Vec::new() });
    });
    // doing in PreDraw because draw will need the textures, but PreDraw should only sync data
    schedule_builder.add_systems(PreDraw, load_textures.in_set(TextureLoadSet));
}

#[derive(Debug)]
pub enum ImageLoadError {
    IOError(io::Error),
    ImageError(ImageError),
}

impl Error for ImageLoadError {}

impl Display for ImageLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ImageLoadError::IOError(e) => write!(f, "Texture load IOError: {}", e),
            ImageLoadError::ImageError(e) => write!(f, "Texture load ImageError: {}", e),
        }
    }
}

impl From<io::Error> for ImageLoadError {
    fn from(value: io::Error) -> Self {
        return Self::IOError(value);
    }
}

impl From<ImageError> for ImageLoadError {
    fn from(value: ImageError) -> Self {
        return Self::ImageError(value);
    }
}

/// Actual representation of image data, not a GPU resource
#[derive(Clone)]
pub struct Image {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Image {
    /// Load from file data
    pub fn load_from_data(data: &[u8]) -> Result<Self, ImageLoadError> {
        Ok(image::load_from_memory(data)?.into())
    }
    /// Load from file
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, ImageLoadError> {
        Ok(Reader::open(path)?.decode()?.into())
    }
}

impl From<DynamicImage> for Image {
    fn from(value: DynamicImage) -> Self {
        Self {
            data: value.to_rgba8().into_vec(),
            width: value.width(),
            height: value.height(),
        }
    }
}

/// A collection of [Images](Image) to be used for mipmap layers
#[derive(Clone)]
pub struct MipMapImage {
    levels: Vec<Image>,
}

impl MipMapImage {
    /// Makes a new [MipMapImage] from its layers
    /// ## Panics
    /// If levels is empty
    pub fn new(levels: Vec<Image>) -> Self {
        if levels.is_empty() {
            panic!("levels may not be empty");
        }
        Self { levels }
    }

    #[inline]
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    #[inline]
    pub fn levels(&self) -> &[Image] {
        &self.levels
    }

    #[inline]
    pub fn to_levels(self) -> Vec<Image> {
        self.levels
    }

    pub fn sizes(&self) -> Vec<(u32, u32)> {
        self.levels
            .iter()
            .map(|img| (img.width, img.height))
            .collect()
    }
}

// FIXME maybe don't use image lib publicly, as web should maybe use a different implementation
// or maybe make another method for web...
impl From<Image> for MipMapImage {
    fn from(value: Image) -> Self {
        Self {
            levels: vec![value],
        }
    }
}

pub enum LayeredTextureError {
    /// Returned if a layered image was attempted, but there are no layers
    NoLayers,
    /// Returned if not all layers share the same size for every mipmap level
    InvalidLayer,
}

/// used to put textures in assets, if the goal is to just load a texture consider [TextureQueue]
#[derive(Resource)]
pub struct TextureQueue {
    queue: Vec<TextureLoadInfo>,
}

impl TextureQueue {
    /// writes a 2d texture to the given asset
    pub fn write(&mut self, image: impl Into<MipMapImage>, asset_id: AssetId<Texture>) {
        self.queue.push(TextureLoadInfo {
            data: TextureLoadData::Flat(image.into()),
            asset_id,
        });
    }

    // writes a layered / 3d texture to the given asset
    pub fn write_layered(
        &mut self,
        layers: Vec<impl Into<MipMapImage>>,
        asset_id: AssetId<Texture>,
    ) -> Result<(), LayeredTextureError> {
        let layers = layers.into_iter().map(|img| img.into()).collect();
        if let Some(err) = validate_layers(&layers) {
            return Err(err);
        }
        self.queue.push(TextureLoadInfo {
            //map from generic into to concrete MipMapImage
            data: TextureLoadData::Layered(layers),
            asset_id,
        });
        Ok(())
    }
}

#[derive(SystemParam)]
pub struct TextureLoader<'w> {
    texture_queue: ResMut<'w, TextureQueue>,
    texture_assets: ResMut<'w, Assets<Texture>>,
}

impl TextureLoader<'_> {
    /// loads a texture
    pub fn load_texture(&mut self, image: impl Into<MipMapImage>) -> AssetId<Texture> {
        let asset_id = self.texture_assets.add_empty();
        self.texture_queue.write(image, asset_id);
        asset_id
    }

    /// loads a layered image, all layers must be same size
    pub fn load_layered_texture(
        &mut self,
        layers: Vec<MipMapImage>,
    ) -> Result<AssetId<Texture>, LayeredTextureError> {
        // checking to not allocate asset in case of error
        if let Some(err) = validate_layers(&layers) {
            return Err(err);
        }
        let asset_id = self.texture_assets.add_empty();
        // should never give error, because layers is already checked
        self.texture_queue.write_layered(layers, asset_id)?;
        Ok(asset_id)
    }
}

fn validate_layers(images: &Vec<MipMapImage>) -> Option<LayeredTextureError> {
    if images.is_empty() {
        return Some(LayeredTextureError::NoLayers);
    }
    let first_size = images[0].sizes();
    if images[1..].iter().all(|img| img.sizes() == first_size) {
        None
    } else {
        Some(LayeredTextureError::InvalidLayer)
    }
}

enum TextureLoadData {
    Flat(MipMapImage),
    Layered(Vec<MipMapImage>),
}

struct TextureLoadInfo {
    data: TextureLoadData,
    asset_id: AssetId<Texture>,
}

fn load_textures(
    mut texture_queue: ResMut<TextureQueue>,
    mut texture_assets: ResMut<Assets<Texture>>,
    device: Res<DeviceRes>,
    queue: Res<QueueRes>,
) {
    for TextureLoadInfo { data, asset_id } in texture_queue.queue.drain(..) {
        // WTF
        let (first, depth, dimension) = match &data {
            TextureLoadData::Flat(img) => (img, 1, TextureDimension::D2),
            TextureLoadData::Layered(imgs) => (&imgs[0], imgs.len() as u32, TextureDimension::D3),
        };

        let img = &first.levels[0];

        let size = Extent3d {
            width: img.width,
            height: img.height,
            depth_or_array_layers: depth,
        };

        let texture = device.0.create_texture(&TextureDescriptor {
            label: None,
            size,
            mip_level_count: first.level_count() as u32,
            sample_count: 1,
            dimension,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // amazing
        for (layer, mip_image) in match data {
            TextureLoadData::Flat(tex) => vec![tex].into_iter(),
            TextureLoadData::Layered(textures) => textures.into_iter(),
        }
        .enumerate()
        {
            for (mip_level, image) in mip_image.levels.into_iter().enumerate() {
                queue.0.write_texture(
                    ImageCopyTexture {
                        texture: &texture,
                        mip_level: mip_level as u32,
                        origin: Origin3d {
                            x: 0,
                            y: 0,
                            z: layer as u32,
                        },
                        aspect: TextureAspect::All,
                    },
                    &image.data,
                    ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * image.width),
                        rows_per_image: Some(image.height),
                    },
                    size,
                );
            }
        }
        texture_assets.replace(asset_id, texture);
    }
}
