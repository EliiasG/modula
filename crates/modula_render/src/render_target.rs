use wgpu::{
    Color, CommandEncoder, Device, Extent3d, LoadOp, Operations, RenderPass,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp,
    SurfaceTexture, Texture, TextureDescriptor, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

pub struct RenderTarget {
    // could use some more fields...
    main_texture: InnerTexture,
    main_view: TextureView,
    multisampled_texture: Option<Texture>,
    multisampled_view: Option<TextureView>,
    depth_stencil_texture: Option<Texture>,
    depth_stencil_view: Option<TextureView>,
    resolve_next: bool,
    clear_next: bool,
    clear_color: Color,
    clear_next_depth_stencil: bool,
    clear_depth: f32,
    clear_stencil: u32,
}

impl RenderTarget {
    #[inline]
    pub fn size(&self) -> (u32, u32) {
        (self.texture().width(), self.texture().height())
    }

    /// Sample count of the internal Texture, will be 1 if not multisampled
    #[inline]
    pub fn sample_count(&self) -> u32 {
        match &self.multisampled_texture {
            Some(t) => t.sample_count(),
            None => 1,
        }
    }

    #[inline]
    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    #[inline]
    pub fn clear_depth(&self) -> f32 {
        self.clear_depth
    }

    #[inline]
    pub fn clear_stencil(&self) -> u32 {
        self.clear_stencil
    }

    /// The primary texture of the RenderTarget, might be changed when the RenderTarget is resized (and possibly in other saturations)
    #[inline]
    pub fn texture(&self) -> &Texture {
        match &self.main_texture {
            InnerTexture::Normal(tex) => tex,
            InnerTexture::Surface(surface_tex) => &surface_tex.texture,
        }
    }

    #[inline]
    pub fn texture_view(&self) -> &TextureView {
        &self.main_view
    }

    /// The depth/stencil texture of the RenderTarget, might be changed when the RenderTarget is resized (and possibly in other saturations)
    #[inline]
    pub fn depth_stencil(&self) -> Option<&Texture> {
        self.depth_stencil_texture.as_ref()
    }

    #[inline]
    pub fn depth_stencil_view(&self) -> Option<&TextureView> {
        self.depth_stencil_view.as_ref()
    }

    /// Resize the RenderTarget, should not be called on the RenderTarget of the surface
    pub fn resize(&mut self, device: &Device, size: (u32, u32)) {
        if let InnerTexture::Surface(_) = self.main_texture {
            eprintln!("tried to resize surface texture...");
            return;
        }
        let mut desc = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        self.main_texture = InnerTexture::Normal(device.create_texture(&desc));
        self.main_view = self
            .texture()
            .create_view(&TextureViewDescriptor::default());
        if self.depth_stencil_texture.is_some() {
            let texture = device.create_texture(&desc);
            self.depth_stencil_view = Some(texture.create_view(&TextureViewDescriptor::default()));
            self.depth_stencil_texture = Some(texture);
        }
        if self.multisampled_texture.is_some() {
            // TODO maybe make customizable?
            desc.sample_count = 4;
            let texture = device.create_texture(&desc);
            self.multisampled_view = Some(texture.create_view(&TextureViewDescriptor::default()));
            self.multisampled_texture = Some(texture);
        }
    }

    #[inline]
    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    #[inline]
    pub fn set_clear_depth(&mut self, depth: f32) {
        self.clear_depth = depth;
    }

    #[inline]
    pub fn set_clear_stencil(&mut self, stencil: u32) {
        self.clear_stencil = stencil;
    }

    #[inline]
    pub fn schedule_clear_color(&mut self) {
        self.clear_next = true;
    }

    #[inline]
    pub fn schedule_clear_depth_stencil(&mut self) {
        self.clear_next_depth_stencil = true;
    }

    /// Next [RenderPass] created with [begin_pass](Self::begin_pass) will be resolving, this method will in most cases automatically be called by the [Sequence](super::Sequence)
    #[inline]
    pub fn schedule_resolve(&mut self) {
        self.resolve_next = true;
    }

    /// Begins a render pass, the pass will be resolving if [resolve_next](Self::resolve_next) was called after the last call to this method
    #[inline]
    pub fn begin_pass<'a>(&'a mut self, command_encoder: &'a mut CommandEncoder) -> RenderPass {
        let old = self.resolve_next;
        self.resolve_next = false;
        self.create_pass(command_encoder, old)
    }

    /// Begins a render pass, the pass will be resolving
    #[inline]
    pub fn begin_resolving_pass<'a>(
        &'a mut self,
        command_encoder: &'a mut CommandEncoder,
    ) -> RenderPass {
        self.create_pass(command_encoder, true)
    }

    /// Begins a render pass, the pass will not be resolving, this should be used for every pass except for the last if a [Operation](super::Operation) needs multiple passes
    #[inline]
    pub fn begin_non_resolving_pass<'a>(
        &'a mut self,
        command_encoder: &'a mut CommandEncoder,
    ) -> RenderPass {
        self.create_pass(command_encoder, false)
    }

    fn create_pass<'a>(
        &'a mut self,
        command_encoder: &'a mut CommandEncoder,
        resolve: bool,
    ) -> RenderPass {
        let clear = self.clear_next;
        let clear_depth_stencil = self.clear_next_depth_stencil;
        self.clear_next = false;
        self.clear_next_depth_stencil = false;
        command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &self.main_view,
                resolve_target: self.multisampled_view.as_ref().filter(|_| resolve),
                ops: Operations {
                    load: if clear {
                        LoadOp::Clear(self.clear_color)
                    } else {
                        LoadOp::Load
                    },
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: self.depth_stencil_view.as_ref().map(|view| {
                RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(Operations {
                        load: if clear_depth_stencil {
                            LoadOp::Clear(self.clear_depth)
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    }),
                    stencil_ops: Some(Operations {
                        load: if clear_depth_stencil {
                            LoadOp::Clear(self.clear_stencil)
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    }),
                }
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}

enum InnerTexture {
    Normal(Texture),
    Surface(SurfaceTexture),
}
