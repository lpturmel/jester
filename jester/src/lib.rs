use std::{any::TypeId, time::Instant};

#[cfg(feature = "vulkan")]
pub use b_vk::VkBackend as DefaultBackend;
use glam::Vec2;
use hashbrown::HashMap;
use jester_core::{
    Camera, Commands, Ctx, EntityPool, Error, InputState, Renderer, Resources, Scene, SceneKey,
    SpriteBatch, SpriteInstance,
};
use tracing::{info, warn};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::Window,
};

mod timer;

pub mod prelude {
    pub use super::App;
    pub use crate::timer::{Timer, TimerMode};
    pub use glam::Vec2;
    pub use jester_core::{
        Backend, Camera, Commands, Ctx, EntityId, Renderer, Scene, Sprite, SpriteBatch, Transform,
    };
    pub use winit::keyboard::KeyCode;
}

type Result<T> = std::result::Result<T, Error>;

pub struct App {
    app_name: String,
    win: Option<winit::window::Window>,
    renderer: Option<Renderer<DefaultBackend>>,
    batches: Vec<SpriteBatch>,
    pending: Vec<Job>,
    cameras: Vec<Camera>,

    active_scene: SceneKey,
    scene_lookup: HashMap<TypeId, SceneKey>,
    dt: f32,
    prev: Instant,
    scenes: Vec<SceneSlot>,
    resources: Resources,
    input_state: InputState,
    pool: EntityPool,
}

impl App {
    pub fn new(app_name: String) -> Self {
        Self {
            app_name,
            win: None,
            renderer: None,
            batches: Vec::new(),
            pending: Vec::new(),
            cameras: Vec::new(),
            active_scene: SceneKey::new(usize::MAX),
            dt: 0.0,
            prev: Instant::now(),
            scenes: Vec::new(),
            resources: Resources::default(),
            pool: EntityPool::default(),
            scene_lookup: HashMap::new(),
            input_state: InputState::default(),
        }
    }

    /// Explicitly mark which scene type should start first.
    ///
    /// Call this **once** after all your `add_scene`s if you want to
    /// override the “first added starts” convention.
    pub fn set_start_scene<S: Scene + 'static>(&mut self) {
        use std::any::TypeId;

        match self.scene_lookup.get(&TypeId::of::<S>()) {
            Some(&key) => self.active_scene = key,
            None => panic!(
                "set_start_scene::<{}> called before add_scene::<{}>",
                std::any::type_name::<S>(),
                std::any::type_name::<S>()
            ),
        }
    }
    pub fn add_resource<T: Send + Sync + 'static>(&mut self, t: T) {
        self.resources.insert(t);
    }
    pub fn add_scene<S: Scene + 'static>(&mut self, scene: S) {
        use std::any::TypeId;

        let key = SceneKey::new(self.scenes.len());

        self.scene_lookup.insert(TypeId::of::<S>(), key);

        self.scenes.push(SceneSlot {
            scene: Box::new(scene),
            must_start: true,
        });

        if *self.active_scene == usize::MAX {
            self.active_scene = key;
        }
    }

    fn apply_commands(&mut self, mut cmds: Commands) {
        for (tex_id, p) in cmds.assets_to_load.drain(..) {
            if let Some(r) = &mut self.renderer {
                let _ = r.load_texture_sync(tex_id, &p);
            }
        }
        for (id, mut s) in cmds.sprites_to_spawn.drain(..) {
            if let Some(renderer) = &mut self.renderer {
                if let Some(meta) = renderer.texture_meta(s.tex) {
                    info!("Found texture meta for {:?}", s.tex);
                    info!("New size: {:?}", meta);
                    s.transform = s.transform.with_size(meta.w as f32, meta.h as f32);
                }
            }
            self.pool.entities.insert(id, s);
        }

        for c in cmds.cameras_to_spawn.drain(..) {
            self.cameras.push(c);
        }

        if let Some(target_type) = cmds.scene_switch.take() {
            if let Some(&key) = self.scene_lookup.get(&target_type) {
                self.pool.entities.clear();
                self.scenes[*key].must_start = true;
                self.active_scene = key;
            } else {
                warn!("goto_scene::<…>() asked for a scene that is not registered");
            }
        }
    }
    pub fn run(&mut self) -> Result<()> {
        let eloop = EventLoop::new()?;
        eloop.set_control_flow(ControlFlow::Poll);

        eloop.run_app(self)?;
        Ok(())
    }
    fn rebuild_batches(&mut self) {
        self.batches.clear();
        for s in self.pool.entities.values() {
            match self.batches.iter_mut().find(|b| b.tex == s.tex) {
                Some(b) => b.instances.push(SpriteInstance {
                    pos_size: s.transform.into(),
                    uv: s.uv,
                }),
                None => self.batches.push(SpriteBatch {
                    tex: s.tex,
                    instances: vec![SpriteInstance {
                        pos_size: s.transform.into(),
                        uv: s.uv,
                    }],
                }),
            }
        }
    }
}
struct SceneSlot {
    scene: Box<dyn Scene>,
    must_start: bool,
}

type Job = Box<dyn FnOnce(&mut App) + Send + 'static>;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let win = event_loop
            .create_window(Window::default_attributes().with_title(&self.app_name))
            .unwrap();
        info!("Creating renderer");
        let rend = Renderer::<DefaultBackend>::new(&self.app_name, &win)
            .expect("Failed to create renderer");

        self.win = Some(win);
        self.renderer = Some(rend);
        let queued: Vec<Job> = std::mem::take(&mut self.pending);

        for job in queued {
            job(self);
        }
        self.win.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let win_size = self.win.as_ref().unwrap().inner_size();
        match event {
            WindowEvent::CloseRequested => {
                info!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    self.input_state
                        .set_key_down(key, event.state == ElementState::Pressed);
                }
            }
            WindowEvent::MouseInput { button, state, .. } => {
                self.input_state
                    .set_mouse_btn(button, state == ElementState::Pressed);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = glam::Vec2::new(position.x as f32, position.y as f32);
                self.input_state.set_mouse_pos(pos);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                self.dt = (now - self.prev).as_secs_f32();
                self.prev = now;

                if *self.active_scene == usize::MAX {
                    warn!("No active scene");
                    if let Some(r) = &mut self.renderer {
                        r.begin_frame();
                        r.end_frame();
                    }
                    return;
                }
                {
                    let slot = &mut self.scenes[*self.active_scene];
                    if slot.must_start {
                        let mut startup_cmds = Commands::default();
                        let mut ctx = Ctx {
                            dt: 0.0,
                            resources: &mut self.resources,
                            commands: &mut startup_cmds,
                            pool: &mut self.pool,
                            input: &self.input_state,
                            screen_pos: Vec2::new(win_size.width as f32, win_size.height as f32),
                        };
                        slot.scene.start(&mut ctx);
                        slot.must_start = false;
                        self.apply_commands(startup_cmds);
                    }
                }

                let mut cmds = Commands::default();
                {
                    let slot = &mut self.scenes[*self.active_scene];
                    let mut ctx = Ctx {
                        screen_pos: Vec2::new(win_size.width as f32, win_size.height as f32),
                        dt: self.dt,
                        resources: &mut self.resources,
                        commands: &mut cmds,
                        pool: &mut self.pool,
                        input: &self.input_state,
                    };
                    slot.scene.update(&mut ctx);
                }
                self.apply_commands(cmds);

                self.rebuild_batches();

                let r = self.renderer.as_mut().expect("renderer is live");

                r.begin_frame();

                if self.cameras.is_empty() {
                } else {
                    for cam in &self.cameras {
                        r.bind_camera(cam);
                        for batch in &self.batches {
                            r.draw_sprites(batch);
                        }
                    }
                }

                r.end_frame();

                self.input_state.begin_frame();
                self.win.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                for c in &mut self.cameras {
                    c.update_pixel_perfect(size.width as f32, size.height as f32);
                }
                let Some(r) = &mut self.renderer else { return };
                r.handle_resize(size);
            }
            _ => (),
        }
    }
}
