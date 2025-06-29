pub use error::Error;
use error::Result;
pub use render::{Backend, Renderer};
pub use sprite::SpriteBatch;
use tracing::info;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

mod error;
mod render;
mod sprite;

#[derive(Default)]
pub struct App;

impl App {
    pub fn run(&self) -> Result<()> {
        let eloop = EventLoop::new()?;
        eloop.set_control_flow(ControlFlow::Poll);

        let mut inner = Inner::default();
        eloop.run_app(&mut inner)?;
        Ok(())
    }
}

#[derive(Default)]
struct Inner {
    win: Option<winit::window::Window>,
}

impl ApplicationHandler for Inner {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        self.win = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
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
                self.win.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}
