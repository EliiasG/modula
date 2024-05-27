use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use wgpu::{
    Adapter, Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PowerPreference, Queue, RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages,
};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, Event as WinitEvent, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

mod world_ext;
pub use world_ext::WorldExt;

pub struct ScheduleBuilder {
    world: World,
}

impl ScheduleBuilder {
    pub fn new() -> Self {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        return Self { world };
    }

    /// ## Warning
    /// be careful not to add the same system multiple times.  
    pub fn add_systems<M>(
        &mut self,
        // not sure how to do without clone, but ScheduleLabels usually implement clone anyway - so should be fine
        schedule: impl ScheduleLabel + Clone,
        systems: impl IntoSystemConfigs<M>,
    ) {
        let mut schedules = self.world.resource_mut::<Schedules>();
        if !schedules.contains(schedule.clone()) {
            schedules.insert(Schedule::new(schedule.clone()));
        }
        // should just be inserted if it didn't exist, so unwrap is ok
        schedules.get_mut(schedule).unwrap().add_systems(systems);
    }

    pub fn finish(self) -> World {
        self.world
    }
}

#[derive(Resource)]
pub struct InstanceRes(pub Instance);

#[derive(Resource)]
pub struct WindowRes(pub &'static Window);

#[derive(Resource)]
pub struct SurfaceRes(pub Surface<'static>);

#[derive(Resource)]
pub struct SurfaceConfigRes(pub SurfaceConfiguration);

#[derive(Resource)]
pub struct AdapterRes(pub Adapter);

#[derive(Resource)]
pub struct DeviceRes(pub Device);

#[derive(Resource)]
pub struct QueueRes(pub Queue);

#[derive(Resource)]
pub struct EventRes(pub WinitEvent<()>);

/// when added to world, app will exit
#[derive(Resource)]
pub struct ShuoldExit;

pub struct GraphicsInitializerResult {
    pub window: &'static Window,
    pub surface: Surface<'static>,
    pub surface_config: SurfaceConfiguration,
    pub instance: Instance,
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

/// Runs before WGPU and window is set up, can be used to load stuff before the window
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct PreInit;

/// Runs once WGPU and window resources are created
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct Init;

/// Runs when there is a window event, event is placed in the [`EventRes`] resource
#[derive(ScheduleLabel, Clone, Hash, PartialEq, Eq, Debug)]
pub struct EventOccured;

pub struct App {
    pub schedule_builder: ScheduleBuilder,
}

struct InitializerData<
    F: FnOnce(PowerPreference, WindowAttributes, &ActiveEventLoop) -> GraphicsInitializerResult,
> {
    initializer: F,
    power_preference: PowerPreference,
    window_attribs: WindowAttributes,
}

struct WinitApp<
    F: FnOnce(PowerPreference, WindowAttributes, &ActiveEventLoop) -> GraphicsInitializerResult,
> {
    world: World,
    initializer_data: Option<InitializerData<F>>,
}

impl<
        F: FnOnce(PowerPreference, WindowAttributes, &ActiveEventLoop) -> GraphicsInitializerResult,
    > WinitApp<F>
{
    fn register_event(&mut self, event_loop: &ActiveEventLoop, event: WinitEvent<()>) {
        // return if not initialized
        if self.initializer_data.is_some() || !self.world.contains_resource::<SurfaceRes>() {
            return;
        }
        self.world.insert_resource(EventRes(event));
        self.world.run_and_apply_deferred(EventOccured);

        if self.world.contains_resource::<ShuoldExit>() {
            event_loop.exit();
        }
    }
}

impl<
        F: FnOnce(PowerPreference, WindowAttributes, &ActiveEventLoop) -> GraphicsInitializerResult,
    > ApplicationHandler for WinitApp<F>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(InitializerData {
            initializer,
            power_preference,
            window_attribs,
        }) = self.initializer_data.take()
        {
            let init_res = initializer(power_preference.clone(), window_attribs, &event_loop);
            add_resources(&mut self.world, init_res);
            self.world.run_and_apply_deferred(Init);
        }
        self.register_event(event_loop, WinitEvent::Resumed);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.register_event(
            event_loop,
            WinitEvent::WindowEvent {
                window_id: window_id,
                event,
            },
        );
    }

    // Provided methods
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.register_event(event_loop, WinitEvent::NewEvents(cause))
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        self.register_event(event_loop, WinitEvent::DeviceEvent { device_id, event })
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.register_event(event_loop, WinitEvent::AboutToWait)
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.register_event(event_loop, WinitEvent::Suspended)
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.register_event(event_loop, WinitEvent::LoopExiting)
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        self.register_event(event_loop, WinitEvent::MemoryWarning)
    }
}

impl App {
    pub fn run(self, power_preference: PowerPreference, window_attribs: WindowAttributes) {
        self.run_with_graphics_initializer(power_preference, window_attribs, default_initializer);
    }

    pub fn run_with_graphics_initializer<F>(
        self,
        power_preference: PowerPreference,
        window_attribs: WindowAttributes,
        initializer: F,
    ) where
        F: Fn(PowerPreference, WindowAttributes, &ActiveEventLoop) -> GraphicsInitializerResult,
    {
        let mut world = self.schedule_builder.finish();
        world.try_add_schedule(PreInit);
        world.try_add_schedule(Init);
        world.try_add_schedule(EventOccured);
        world.run_and_apply_deferred(PreInit);
        let event_loop = EventLoop::new().expect("Failed to make event loop");
        event_loop
            .run_app(&mut WinitApp {
                world,
                initializer_data: Some(InitializerData {
                    initializer,
                    power_preference,
                    window_attribs,
                }),
            })
            .expect("failed to run loop");
    }
}

fn add_resources(world: &mut World, init_res: GraphicsInitializerResult) {
    world.insert_resource(WindowRes(init_res.window));
    world.insert_resource(SurfaceRes(init_res.surface));
    world.insert_resource(SurfaceConfigRes(init_res.surface_config));
    world.insert_resource(InstanceRes(init_res.instance));
    world.insert_resource(AdapterRes(init_res.adapter));
    world.insert_resource(DeviceRes(init_res.device));
    world.insert_resource(QueueRes(init_res.queue));
}

fn default_initializer(
    power_preference: PowerPreference,
    window_attribs: WindowAttributes,
    event_loop: &ActiveEventLoop,
) -> GraphicsInitializerResult {
    //env_logger::init();
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });

    let window = event_loop
        .create_window(window_attribs.clone())
        .expect("failed to create window");
    // must be static because it has to be a bevy resource
    let window: &'static Window = Box::leak(Box::new(window));

    let surface = instance.create_surface(window).expect("no surface?");

    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference,
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .expect("no adapter?");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &DeviceDescriptor {
            label: None,
            required_features: Features::default(),
            required_limits: Limits::default(),
        },
        None,
    ))
    .expect("no device?");
    let caps = surface.get_capabilities(&adapter);
    let size = window.inner_size();
    let surface_config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .expect("SRGB not supported, this is strange..."),
        width: size.width,
        height: size.height,
        present_mode: caps.present_modes[0],
        desired_maximum_frame_latency: 2,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
    };
    surface.configure(&device, &surface_config);
    return GraphicsInitializerResult {
        window,
        surface,
        surface_config,
        instance,
        adapter,
        device,
        queue,
    };
}

// FIXME maybe move to some util crate instead?
pub fn init_window_closing(schedule_builder: &mut ScheduleBuilder) {
    schedule_builder.add_systems(EventOccured, handle_window_close)
}

fn handle_window_close(mut commands: Commands, event: Res<EventRes>) {
    match event.0 {
        WinitEvent::WindowEvent {
            window_id: _,
            event: WindowEvent::CloseRequested,
        } => commands.insert_resource(ShuoldExit),
        _ => {}
    }
}
