use wgpu::{
    Color, CommandEncoder, Device, Extent3d, LoadOp, Operations, RenderPass,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp,
    SurfaceTexture, Texture, TextureDescriptor, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

#[derive(Clone, PartialEq)]
pub struct RenderTargetDepthStencilConfig {
    /// The clear depth of the render target
    pub clear_depth: f32,
    /// The clear stencil of the render target
    pub clear_stencil: u32,
    /// The usages of the depth/stencil texture, [RENDER_ATTACHMENT](TextureUsages::RENDER_ATTACHMENT) always set
    pub usages: TextureUsages,
    /// The format of the depth/stencil texture
    pub format: TextureFormat,
}

impl Default for RenderTargetDepthStencilConfig {
    fn default() -> Self {
        RenderTargetDepthStencilConfig {
            clear_depth: 1.0,
            clear_stencil: 0,
            usages: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Depth24PlusStencil8,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct RenderTargetMultisampleConfig {
    /// sample count of the internal Texture
    pub sample_count: u32,
}

impl Default for RenderTargetMultisampleConfig {
    #[inline]
    fn default() -> Self {
        RenderTargetMultisampleConfig { sample_count: 4 }
    }
}

#[derive(Clone, PartialEq)]
pub struct RenderTargetColorConfig {
    // TODO maybe move multisample config to here, as it is only allowed when color is used
    /// The clear color of the render target
    pub clear_color: Color,
    /// The usages of the main texture, [RENDER_ATTACHMENT](TextureUsages::RENDER_ATTACHMENT) always set
    pub usages: TextureUsages,
    /// The format of the color texture
    pub format: TextureFormat,
}

impl Default for RenderTargetColorConfig {
    #[inline]
    fn default() -> Self {
        RenderTargetColorConfig {
            clear_color: Color::BLACK,
            usages: TextureUsages::RENDER_ATTACHMENT,
            format: TextureFormat::Rgba8UnormSrgb,
        }
    }
}

#[derive(Clone)]
pub struct RenderTargetConfig {
    /// The size of the textures
    pub size: (u32, u32),
    /// The multisample config of the texture, if None the texture will not be multisampled
    pub multisample_config: Option<RenderTargetMultisampleConfig>,
    /// The depth/stencil config of the texture, if None the texture will not have a depth/stencil buffer
    pub depth_stencil_config: Option<RenderTargetDepthStencilConfig>,
    /// The color config of the texture, if None the texture will not have a color buffer
    pub color_config: Option<RenderTargetColorConfig>,
}

impl Default for RenderTargetConfig {
    fn default() -> Self {
        RenderTargetConfig {
            size: (1, 1),
            multisample_config: None,
            depth_stencil_config: Some(Default::default()),
            color_config: Some(Default::default()),
        }
    }
}

pub struct RenderTarget {
    current_config: Option<RenderTargetConfig>,
    scheduled_config: Option<RenderTargetConfig>,

    main_texture: Option<TextureWithView>,
    multisampled_texture: Option<TextureWithView>,
    depth_stencil_texture: Option<TextureWithView>,

    resolve_next: bool,
    clear_next: bool,
    clear_next_depth_stencil: bool,
}

impl RenderTarget {
    /// Create a new RenderTarget with the given config.  
    /// Config needs to be applied using [apply](Self::apply) before it can be used
    pub fn new(config: RenderTargetConfig) -> Self {
        RenderTarget {
            current_config: None,
            scheduled_config: Some(config),
            main_texture: None,
            multisampled_texture: None,
            depth_stencil_texture: None,
            resolve_next: false,
            clear_next: false,
            clear_next_depth_stencil: false,
        }
    }

    /// The currently used config of the RenderTarget, if not initialized this will return the planned config
    pub fn current_config(&self) -> &RenderTargetConfig {
        self.current_config
            .as_ref()
            .or_else(|| self.scheduled_config.as_ref())
            .expect("No current config, this should not happen")
    }

    /// The planned config of the RenderTarget, if no change is planned this will return None
    pub fn scheduled_config(&self) -> Option<&RenderTargetConfig> {
        self.scheduled_config.as_ref()
    }

    pub fn is_surface(&self) -> bool {
        self.main_texture
            .as_ref()
            .map(|t| match t.texture {
                InnerTexture::Normal(_) => false,
                InnerTexture::Surface(_) => true,
            })
            .unwrap_or(false)
    }

    /// Mutable version of [scheduled_config](Self::scheduled_config), if there are no changes planned this will return a copy of the current config
    pub fn scheduled_config_mut(&mut self) -> &mut RenderTargetConfig {
        if self.scheduled_config.is_none() {
            self.scheduled_config = Some(self.current_config().clone());
        }
        self.scheduled_config.as_mut().unwrap()
    }

    /// The size of the textures
    #[inline]
    pub fn size(&self) -> (u32, u32) {
        self.current_config().size
    }

    /// Sample count of the internal Texture, will be 1 if not multisampled
    #[inline]
    pub fn sample_count(&self) -> u32 {
        match &self.current_config().multisample_config {
            Some(t) => t.sample_count,
            None => 1,
        }
    }

    /// The clear color of the render target, if no color buffer is used this will return None
    #[inline]
    pub fn clear_color(&self) -> Option<Color> {
        self.current_config()
            .color_config
            .as_ref()
            .map(|c| c.clear_color)
    }

    /// The clear depth of the render target, if no depth/stencil buffer is used this will return None
    #[inline]
    pub fn clear_depth(&self) -> Option<f32> {
        self.current_config()
            .depth_stencil_config
            .as_ref()
            .map(|c| c.clear_depth)
    }

    /// The clear stencil of the render target, if no depth/stencil buffer is used this will return None
    #[inline]
    pub fn clear_stencil(&self) -> Option<u32> {
        self.current_config()
            .depth_stencil_config
            .as_ref()
            .map(|c| c.clear_stencil)
    }

    /// The primary texture of the RenderTarget, might be changed when the RenderTarget is resized (and possibly in other saturations)
    #[inline]
    pub fn texture(&self) -> Option<&Texture> {
        self.main_texture.as_ref().map(|t| t.texture())
    }

    /// The primary texture view of the RenderTarget, might be changed when the RenderTarget is resized (and possibly in other saturations)
    #[inline]
    pub fn texture_view(&self) -> Option<&TextureView> {
        self.main_texture.as_ref().map(|t| &t.view)
    }

    /// The depth/stencil texture of the RenderTarget, might be changed when the RenderTarget is resized (and possibly in other saturations)
    #[inline]
    pub fn depth_stencil(&self) -> Option<&Texture> {
        self.depth_stencil_texture.as_ref().map(|t| t.texture())
    }

    /// The depth/stencil texture view of the RenderTarget, might be changed when the RenderTarget is resized (and possibly in other saturations)
    #[inline]
    pub fn depth_stencil_view(&self) -> Option<&TextureView> {
        self.depth_stencil_texture.as_ref().map(|t| &t.view)
    }

    /// Resize the RenderTarget when config is applied, should not be called on the RenderTarget of the surface
    pub fn resize(&mut self, size: (u32, u32)) {
        self.scheduled_config_mut().size = size;
    }

    /// Set the planned clear color of the render target, if no color buffer is used this will do nothing.  
    #[inline]
    pub fn set_clear_color(&mut self, color: Color) {
        let config = self.scheduled_config_mut();
        if config.color_config.is_some() {
            config.color_config.as_mut().unwrap().clear_color = color;
        }
    }

    /// Set the planned clear depth of the render target, if no depth/stencil buffer is used this will do nothing.  
    #[inline]
    pub fn set_clear_depth(&mut self, depth: f32) {
        let config = self.scheduled_config_mut();
        if config.depth_stencil_config.is_some() {
            config.depth_stencil_config.as_mut().unwrap().clear_depth = depth;
        }
    }

    /// Set the planned clear stencil of the render target, if no depth/stencil buffer is used this will do nothing.  
    #[inline]
    pub fn set_clear_stencil(&mut self, stencil: u32) {
        let config = self.scheduled_config_mut();
        if config.depth_stencil_config.is_some() {
            config.depth_stencil_config.as_mut().unwrap().clear_stencil = stencil;
        }
    }

    /// The next [RenderPass] created with [begin_pass](Self::begin_pass) will clear the main texture.  
    /// Note that if the render target is multisampled the multisampled texture will be cleared, and the main texture will not be cleared before the next resolve.  
    #[inline]
    pub fn schedule_clear_color(&mut self) {
        self.clear_next = true;
    }

    /// The next [RenderPass] created with [begin_pass](Self::begin_pass) will clear the depth/stencil texture.
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

    /// Apply the changes to the RenderTarget, this will recreate the textures if needed
    #[inline]
    pub fn apply(&mut self, device: &Device) {
        self.apply_changes(device, self.changes());
    }

    pub(crate) fn apply_surface(&mut self, device: &Device, surface_texture: SurfaceTexture) {
        let size = surface_texture.texture.size();
        self.resize((size.width, size.height));
        self.main_texture = Some(TextureWithView::from_surface_texture(surface_texture));
        let mut changes = self.changes();
        changes.color_changed = false;
        self.apply_changes(device, changes);
    }

    pub(crate) fn present(&mut self) {
        match self
            .main_texture
            .take()
            .expect("no main texture while presenting surface")
            .texture
        {
            InnerTexture::Normal(_) => panic!("main texture was not a surface texture"),
            InnerTexture::Surface(s) => s.present(),
        }
    }

    fn apply_changes(&mut self, device: &Device, changes: RenderTargetChanges) {
        self.current_config = self.scheduled_config.take();
        if !changes.color_changed && !changes.depth_stencil_changed && !changes.multisample_changed
        {
            return;
        }
        let mut desc = TextureDescriptor {
            label: None,
            size: Extent3d {
                width: self.current_config().size.0,
                height: self.current_config().size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        };

        // the order of the following if statements is important, as they modify and use desc

        if changes.color_changed {
            if self.is_surface() {
                eprintln!("tried to change surface texture, most likely by resizing...");
                return;
            }

            // funky map abuse
            self.main_texture = self.current_config().color_config.as_ref().map(|c| {
                desc.usage = c.usages | TextureUsages::RENDER_ATTACHMENT;
                desc.format = c.format;
                TextureWithView::from_texture(device.create_texture(&desc))
            });
        }

        if changes.multisample_changed {
            self.multisampled_texture =
                self.current_config().multisample_config.as_ref().map(|c| {
                    // format left same as color
                    desc.usage = TextureUsages::RENDER_ATTACHMENT;
                    desc.sample_count = c.sample_count;
                    TextureWithView::from_texture(device.create_texture(&desc))
                });
        }

        if changes.depth_stencil_changed {
            self.depth_stencil_texture =
                self.current_config()
                    .depth_stencil_config
                    .as_ref()
                    .map(|c| {
                        // threading the needle with those side effects
                        desc.sample_count = 1;
                        desc.usage = c.usages | TextureUsages::RENDER_ATTACHMENT;
                        desc.format = c.format;
                        TextureWithView::from_texture(device.create_texture(&desc))
                    });
        }
    }

    // inline because it's only used in apply
    #[inline]
    fn changes(&self) -> RenderTargetChanges {
        if self.current_config.is_none() {
            return RenderTargetChanges {
                color_changed: true,
                depth_stencil_changed: true,
                multisample_changed: true,
            };
        }
        if self.scheduled_config.is_none() {
            return RenderTargetChanges {
                color_changed: false,
                depth_stencil_changed: false,
                multisample_changed: false,
            };
        }
        let current = self.current_config.as_ref().unwrap();
        let scheduled = self.scheduled_config.as_ref().unwrap();
        let resized = current.size != scheduled.size;
        RenderTargetChanges {
            color_changed: resized
                || different(
                    current.color_config.as_ref(),
                    scheduled.color_config.as_ref(),
                    |c| c.usages,
                ),
            depth_stencil_changed: resized
                || different(
                    current.depth_stencil_config.as_ref(),
                    scheduled.depth_stencil_config.as_ref(),
                    |c| (c.usages, c.format),
                ),
            multisample_changed: resized
                // only field is sample count
                || (current.multisample_config != scheduled.multisample_config),
        }
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
            color_attachments: &[self.main_texture.as_ref().map(|tex_with_view| {
                RenderPassColorAttachment {
                    view: &tex_with_view.view,
                    resolve_target: self
                        .multisampled_texture
                        .as_ref()
                        // only resolve if resolve is true, kinda sus
                        .filter(|_| resolve)
                        .map(|t| &t.view),
                    ops: Operations {
                        load: if clear {
                            LoadOp::Clear(
                                self.current_config()
                                    .color_config
                                    .as_ref()
                                    .expect("texture but no color config")
                                    .clear_color,
                            )
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    },
                }
            })],
            // maybe fix DRY
            depth_stencil_attachment: self.depth_stencil_texture.as_ref().map(|tex_with_view| {
                RenderPassDepthStencilAttachment {
                    view: &tex_with_view.view,
                    depth_ops: Some(Operations {
                        load: if clear_depth_stencil {
                            LoadOp::Clear(
                                self.current_config()
                                    .depth_stencil_config
                                    .as_ref()
                                    .expect("texture but no depth/stencil config")
                                    .clear_depth,
                            )
                        } else {
                            LoadOp::Load
                        },
                        store: StoreOp::Store,
                    }),
                    stencil_ops: Some(Operations {
                        load: if clear_depth_stencil {
                            LoadOp::Clear(
                                self.current_config()
                                    .depth_stencil_config
                                    .as_ref()
                                    .expect("texture but no depth/stencil config")
                                    .clear_stencil,
                            )
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

fn different<T, R: PartialEq>(a: Option<T>, b: Option<T>, val: impl Fn(T) -> R) -> bool {
    if a.is_none() && b.is_none() {
        return false;
    }
    a.map(|a| val(a)) != b.map(|b| val(b))
}

struct RenderTargetChanges {
    color_changed: bool,
    depth_stencil_changed: bool,
    multisample_changed: bool,
}

enum InnerTexture {
    Normal(Texture),
    Surface(SurfaceTexture),
}

// better name?
struct TextureWithView {
    texture: InnerTexture,
    view: TextureView,
}

impl TextureWithView {
    #[inline]
    fn texture(&self) -> &Texture {
        match &self.texture {
            InnerTexture::Normal(t) => t,
            InnerTexture::Surface(t) => &t.texture,
        }
    }

    fn from_texture(texture: Texture) -> Self {
        let view = texture.create_view(&TextureViewDescriptor::default());
        Self {
            texture: InnerTexture::Normal(texture),
            view,
        }
    }

    fn from_surface_texture(texture: SurfaceTexture) -> Self {
        let view = texture
            .texture
            .create_view(&TextureViewDescriptor::default());
        Self {
            texture: InnerTexture::Surface(texture),
            view,
        }
    }
}
