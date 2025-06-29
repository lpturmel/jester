#[cfg(feature = "vulkan")]
pub use b_vk::VkBackend as DefaultBackend;
use jester_core::{Error, Renderer};
use tracing::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub mod prelude {
    pub use jester_core::{Backend, Renderer, SpriteBatch};
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct App {
    app_name: String,
    win: Option<winit::window::Window>,
    renderer: Option<Renderer<DefaultBackend>>,
}

impl App {
    pub fn new(app_name: String) -> Self {
        Self {
            app_name,
            win: None,
            renderer: None,
        }
    }
    pub fn run(&mut self) -> Result<()> {
        let eloop = EventLoop::new()?;
        eloop.set_control_flow(ControlFlow::Poll);

        eloop.run_app(self)?;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let win = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        info!("Creating renderer");
        let rend = Renderer::<DefaultBackend>::new(&self.app_name, &win)
            .expect("Failed to create renderer");

        win.request_redraw();
        self.win = Some(win);
        self.renderer = Some(rend);
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
                let Some(r) = &mut self.renderer else { return };
                r.begin_frame();
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
