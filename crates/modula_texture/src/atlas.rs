use core::fmt::Debug;

use bevy_ecs::system::{Res, ResMut, Resource};
use modula_asset::{AssetId, Assets};
use modula_core::{DeviceRes, QueueRes, ScheduleBuilder};
use modula_render::PreDraw;
use wgpu::{
    Device, Extent3d, Origin3d, Queue, Texture, TextureDescriptor, TextureFormat, TextureUsages,
};

use crate::MipMapImage;

mod default_layouter;

pub use default_layouter::*;

/// Inits atlas loading using a custom atlas loader, for most cases you can just use [init_atlas_loading]
pub fn init_custom_atlas_loading<L: AtlasLayouter + 'static>(
    schedule_builder: &mut ScheduleBuilder,
) {
    schedule_builder.add_systems(PreDraw, handle_atlas_group_queue::<L>)
}

/// Inits atlas loading using [DefaultLayouter], use [init_custom_atlas_loading] to use a different [AtlasLayouter]
#[inline]
pub fn init_atlas_loading(schedule_builder: &mut ScheduleBuilder) {
    init_custom_atlas_loading::<DefaultLayouter>(schedule_builder);
}

/// A texture atlas, used to store many textures in a single texture that can be indexed
pub struct Atlas {
    texture: Texture,
    layout: AtlasLayout,
}

impl Atlas {
    pub fn new(texture: Texture, layout: AtlasLayout) -> Self {
        Self { texture, layout }
    }

    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    pub fn layout(&self) -> &AtlasLayout {
        &self.layout
    }

    // TODO bind group perhaps???
}

/// Layout of the atlas
pub struct AtlasLayout(pub Vec<SubTexture>);

/// A subsection of a texture atlas.
#[derive(Clone, Copy)]
pub struct SubTexture {
    pub layer: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A group of [Atlases](Atlas), this is useful to 'pretend' that multiple atlases are the same, as a single atlas may not be big enough for all [SubTextures](SubTexture)
pub struct AtlasGroup {
    atlases: Vec<Atlas>,
    entry_map: Vec<(usize, usize)>,
}

impl AtlasGroup {
    pub fn new(atlases: Vec<Atlas>) -> Self {
        let mut entry_map = Vec::new();
        for (i, atlas) in atlases.iter().enumerate() {
            let mut r = (0..atlas.layout.0.len()).map(|j| (i, j)).collect();
            entry_map.append(&mut r);
        }
        AtlasGroup {
            atlases: atlases,
            entry_map: entry_map,
        }
    }

    pub fn atlas_count(&self) -> usize {
        self.atlases.len()
    }

    pub fn atlases(&self) -> &[Atlas] {
        &self.atlases
    }
    /// used to map [AtlasGroupEntries](AtlasGroupEntry) to (atlas (index), subtexture (index))
    pub fn entry_map(&self) -> &[(usize, usize)] {
        &self.entry_map
    }
}

/// An entry into an [AtlasGroup]
pub struct AtlasGroupEntry(usize);

impl AtlasGroupEntry {
    /// index in the [entry_map](AtlasGroup::entry_map) of its [AtlasGroup]
    pub fn index(&self) -> usize {
        self.0
    }
}

/// Can be used to create an [AtlasGroup]
pub struct AtlasGroupBuilder {
    images: Vec<MipMapImage>,
    mip_levels: u32,
    usages: TextureUsages,
}

impl AtlasGroupBuilder {
    #[inline]
    pub fn new(mip_levels: u32) -> Self {
        Self::with_usages(TextureUsages::TEXTURE_BINDING, mip_levels)
    }

    pub fn with_usages(usages: TextureUsages, mip_levels: u32) -> Self {
        Self {
            images: Vec::new(),
            mip_levels,
            usages: usages | TextureUsages::COPY_DST,
        }
    }

    /// If image has 1 mipmap level, it will be drawn to the first mip level.  
    /// Otherwise it should match the mip levels of the [AtlasGroupDesciptor]
    pub fn add_image(&mut self, img: impl Into<MipMapImage>) -> AtlasGroupEntry {
        self.images.push(img.into());
        AtlasGroupEntry(self.images.len() - 1)
    }

    #[inline]
    pub fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    /// Returns the sizes of the elements, useful for layouting
    pub fn sizes(&self) -> Vec<(u32, u32)> {
        self.images.iter().map(|e| e.sizes()[0]).collect()
    }

    /// Builds an atlas, for basic cases use [AtlasGroupQueue]
    pub fn build<L: AtlasLayouter>(
        &self,
        device: &Device,
        queue: &Queue,
    ) -> Result<AtlasGroup, L::Error> {
        let lim = device.limits();
        let output = L::layout(
            self.sizes(),
            MaxAtlasSize {
                max_width_hight: lim.max_texture_dimension_2d,
                max_layers: lim.max_texture_array_layers,
            },
        )?;
        let mut atlases = Vec::with_capacity(output.atlases.len());
        for layout in output.atlases {
            let tex = create_atlas_texture(&device, &layout, self);
            atlases.push(Atlas::new(tex, layout.1));
        }
        for (img_idx, (atlas_idx, el_idx)) in output.entry_map.iter().enumerate() {
            let atlas = &atlases[*atlas_idx];
            let subtex = atlas.layout.0[*el_idx];
            let img = &self.images[img_idx];
            img.write_to_texture(
                queue,
                Origin3d {
                    x: subtex.x,
                    y: subtex.y,
                    z: subtex.layer,
                },
                atlas.texture(),
            )
        }
        Ok(AtlasGroup::new(atlases))
    }
}

/// Used to layout and create [AtlasGroup]s, to manually layout groups you can directly create [AtlasGroup]s
#[derive(Resource)]
pub struct AtlasGroupQueue(Vec<(AssetId<AtlasGroup>, AtlasGroupBuilder)>);

impl AtlasGroupQueue {
    pub fn init_group(&mut self, group: AssetId<AtlasGroup>, descriptor: AtlasGroupBuilder) {
        self.0.push((group, descriptor));
    }
}

pub trait AtlasLayouter {
    type Error: Debug + Sized;
    /// Layouts an [AtlasGroup] by taking a vec of image sizes and returning the sizes and layouts of atlases in a group
    fn layout(
        sizes: Vec<(u32, u32)>,
        max_atlas_size: MaxAtlasSize,
    ) -> Result<AtlasLayouterOutput, Self::Error>;
}

pub struct AtlasLayouterOutput {
    /// This should map texture indices to (atlas_idx, subtex_idx)
    pub entry_map: Vec<(usize, usize)>,
    /// A vec of atlas sizes and layouts
    pub atlases: Vec<((u32, u32, u32), AtlasLayout)>,
}

pub struct MaxAtlasSize {
    /// Maximum width and height
    pub max_width_hight: u32,
    pub max_layers: u32,
}

fn handle_atlas_group_queue<L: AtlasLayouter>(
    mut in_queue: ResMut<AtlasGroupQueue>,
    mut atlas_groups: ResMut<Assets<AtlasGroup>>,
    device: Res<DeviceRes>,
    queue: Res<QueueRes>,
) {
    for (group, builder) in in_queue.0.drain(..) {
        atlas_groups.replace(
            group,
            builder
                .build::<L>(&device.0, &queue.0)
                .expect("error during atlas layout"),
        );
    }
}

fn create_atlas_texture(
    device: &Device,
    layout: &((u32, u32, u32), AtlasLayout),
    descriptor: &AtlasGroupBuilder,
) -> Texture {
    let size = Extent3d {
        width: layout.0 .0,
        height: layout.0 .1,
        depth_or_array_layers: layout.0 .2,
    };
    let tex = device.create_texture(&TextureDescriptor {
        label: Some("Atlas Texture"),
        size,
        mip_level_count: descriptor.mip_levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: descriptor.usages,
        view_formats: &[],
    });
    tex
}
