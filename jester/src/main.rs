use jester::prelude::*;
use std::time::Duration;
use tracing::{info, warn};
use winit::keyboard::KeyCode;

#[derive(Default)]
struct OddScene {
    player: Option<EntityId>,
}

impl Scene for OddScene {
    fn start(&mut self, ctx: &mut Ctx<'_>) {
        ctx.spawn_camera(Camera::pixel_perfect(ctx.screen_pos.x, ctx.screen_pos.y));
        let aseprite_id = ctx.load_asset("assets/aseprite.png");
        info!("Aseprite image has id {:?}", aseprite_id);

        let entity = ctx.spawn_sprite(Sprite {
            transform: Transform::from_xy(0.0, 0.0),
            tex: aseprite_id,
            ..Default::default()
        });
        self.player = Some(entity);
    }
    fn update(&mut self, ctx: &mut Ctx<'_>) {
        let Some(player) = self.player else {
            warn!("Player entity not found");
            return;
        };
        let Some(player_sprite) = ctx.pool.sprite_mut(player) else {
            return;
        };

        const SPEED: f32 = 150.0;
        if ctx.input.key_pressed(KeyCode::KeyW) {
            player_sprite.transform.translation.y += SPEED * ctx.dt;
        }
        if ctx.input.key_pressed(KeyCode::KeyS) {
            player_sprite.transform.translation.y -= SPEED * ctx.dt;
        }
        if ctx.input.key_pressed(KeyCode::KeyA) {
            player_sprite.transform.translation.x -= SPEED * ctx.dt;
        }
        if ctx.input.key_pressed(KeyCode::KeyD) {
            player_sprite.transform.translation.x += SPEED * ctx.dt;
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut app = App::new("cool game".to_string());
    app.add_scene(OddScene::default());
    app.set_start_scene::<OddScene>();

    app.add_resource(Timer::new(Duration::from_secs(5), TimerMode::Loop));

    app.run().unwrap();
}
