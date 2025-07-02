use jester::prelude::*;

fn main() {
    tracing_subscriber::fmt::init();

    let mut app = App::new("cool game".to_string());
    app.add_camera(Camera::default());
    let aseprite_id = app.load_asset("assets/aseprite.png").unwrap();
    let samurai_id = app.load_asset("assets/samurai.png").unwrap();

    app.add_sprite(
        [256.0, 160.0, 200.0, 340.0],
        [0.0, 0.0, 1.0, 1.0],
        samurai_id,
    );
    app.add_sprite(
        [64.0, 160.0, 128.0, 128.0],
        [0.0, 0.0, 1.0, 1.0],
        aseprite_id,
    );

    app.run().unwrap();
}
