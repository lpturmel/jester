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
}

impl App {
    pub fn new(app_name: String) -> Self {
        Self { app_name }
    }
    pub fn run(&self) -> Result<()> {
        let eloop = EventLoop::new()?;
        eloop.set_control_flow(ControlFlow::Poll);

        let mut inner = Inner {
            app_name: self.app_name.clone(),
            ..Default::default()
        };
        eloop.run_app(&mut inner)?;
        Ok(())
    }
}

#[derive(Default)]
struct Inner {
    app_name: String,
    win: Option<winit::window::Window>,
    renderer: Option<Renderer<DefaultBackend>>,
}

impl ApplicationHandler for Inner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let win = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        info!("Creating renderer");
        let rend = Renderer::<DefaultBackend>::new(&self.app_name, &win, &win)
            .expect("Failed to create renderer");
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
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                // self.win.as_ref().unwrap().request_redraw();
                let Some(r) = &mut self.renderer else { return };
                r.begin_frame();
                r.end_frame();
            }
            _ => (),
        }
    }
}
