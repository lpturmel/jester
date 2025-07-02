use std::{
    path::PathBuf,
    sync::atomic::{AtomicU32, Ordering},
};

#[cfg(feature = "vulkan")]
pub use b_vk::VkBackend as DefaultBackend;
use jester_core::{Camera, Error, Renderer, SpriteBatch, SpriteInstance, TextureId};
use tracing::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub mod prelude {
    pub use super::App;
    pub use glam::Vec2;
    pub use jester_core::{Backend, Camera, Renderer, SpriteBatch};
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct App {
    app_name: String,
    win: Option<winit::window::Window>,
    renderer: Option<Renderer<DefaultBackend>>,
    batches: Vec<SpriteBatch>,
    pending: Vec<Job>,
    next_tex: AtomicU32,
    cameras: Vec<Camera>,
}

impl App {
    pub fn new(app_name: String) -> Self {
        Self {
            app_name,
            win: None,
            renderer: None,
            batches: Vec::new(),
            pending: Vec::new(),
            next_tex: AtomicU32::new(0),
            cameras: Vec::new(),
        }
    }

    fn push_sprite(&mut self, pos_size: [f32; 4], uv: [f32; 4], tex: TextureId) {
        match self.batches.iter_mut().find(|b| b.tex == tex) {
            Some(batch) => batch.instances.push(SpriteInstance { pos_size, uv }),
            None => self.batches.push(SpriteBatch {
                tex,
                instances: vec![SpriteInstance { pos_size, uv }],
            }),
        }
    }
    pub fn add_sprite(&mut self, pos_size: [f32; 4], uv: [f32; 4], tex: TextureId) {
        if self.renderer.is_some() {
            self.push_sprite(pos_size, uv, tex);
            return;
        }

        self.pending.push(Box::new(move |app: &mut App| {
            app.push_sprite(pos_size, uv, tex);
        }));
    }
    pub fn add_camera(&mut self, camera: Camera) -> usize {
        self.cameras.push(camera);
        self.cameras.len() - 1
    }
    pub fn load_asset<P>(&mut self, path: P) -> Result<TextureId>
    where
        P: Into<PathBuf> + Send + 'static,
    {
        let path_buf = path.into();

        if let Some(r) = &mut self.renderer {
            return Ok(r.load_texture_sync(&path_buf)?);
        }

        let reserved = TextureId(self.next_tex.fetch_add(1, Ordering::Relaxed));

        self.pending.push(Box::new(move |app: &mut App| {
            let real = app
                .renderer
                .as_mut()
                .expect("renderer must exist")
                .load_texture_sync(&path_buf)
                .expect("texture upload failed");

            debug_assert_eq!(real, reserved);
        }));

        Ok(reserved)
    }
    pub fn run(&mut self) -> Result<()> {
        let eloop = EventLoop::new()?;
        eloop.set_control_flow(ControlFlow::Poll);

        eloop.run_app(self)?;
        Ok(())
    }
}

type Job = Box<dyn FnOnce(&mut App) + Send + 'static>;

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let win = event_loop
            .create_window(Window::default_attributes().with_title(&self.app_name))
            .unwrap();
        info!("Creating renderer");
        // TODO: expose camera to user
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
        match event {
            WindowEvent::CloseRequested => {
                info!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if self.cameras.is_empty() {
                    let r = self.renderer.as_mut().unwrap();
                    r.begin_frame();
                    r.end_frame();
                    return;
                }
                let r = self.renderer.as_mut().unwrap();
                r.begin_frame();

                for cam in &self.cameras {
                    r.bind_camera(cam);
                    for batch in &self.batches {
                        r.draw_sprites(batch);
                    }
                }

                r.end_frame();
                self.win.as_ref().unwrap().request_redraw();
            }
            WindowEvent::Resized(size) => {
                let Some(r) = &mut self.renderer else { return };
                r.handle_resize(size);
            }
            _ => (),
        }
    }
}
