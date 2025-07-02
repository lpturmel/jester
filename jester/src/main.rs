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
        let aseprite_id = ctx.load_asset("assets/aseprite.png");
        let entity = ctx.spawn_sprite(Sprite {
            rect: [400.0, 300.0, 128.0, 128.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            tex: aseprite_id,
        });
        info!("Aseprite image has id {:?}", aseprite_id);
        self.player = Some(entity);
    }
    fn update(&mut self, ctx: &mut Ctx<'_>) {
        let Some(player) = self.player else {
            warn!("Player entity not found");
            return;
        };

        const SPEED: f32 = 150.0;
        if ctx.input.key_pressed(KeyCode::KeyW) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.rect[1] += SPEED * ctx.dt;
            }
        } else if ctx.input.key_pressed(KeyCode::KeyS) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.rect[1] -= SPEED * ctx.dt;
            }
        } else if ctx.input.key_pressed(KeyCode::KeyA) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.rect[0] -= SPEED * ctx.dt;
            }
        } else if ctx.input.key_pressed(KeyCode::KeyD) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.rect[0] += SPEED * ctx.dt;
            }
        }

        if ctx.input.just_pressed(KeyCode::Space) {
            info!("Switching to even scene");
            ctx.goto_scene::<EvenScene>();
        }
    }
}

#[derive(Default)]
struct EvenScene {
    timer: f32,
}

impl Scene for EvenScene {
    fn start(&mut self, ctx: &mut Ctx<'_>) {
        self.timer = 0.0;
        let samurai_id = ctx.load_asset("assets/samurai.png");
        ctx.spawn_sprite(Sprite {
            rect: [400.0, 300.0, 100.0, 170.0],
            uv: [0.0, 0.0, 1.0, 1.0],
            tex: samurai_id,
        });
    }
    fn update(&mut self, ctx: &mut Ctx<'_>) {
        if ctx.input.just_pressed(KeyCode::Space) {
            info!("Switching to odd scene");
            ctx.goto_scene::<OddScene>();
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut app = App::new("cool game".to_string());
    app.add_camera(Camera::default());
    app.add_scene(OddScene::default());
    app.add_scene(EvenScene::default());
    app.set_start_scene::<OddScene>();

    app.add_resource(Timer::new(Duration::from_secs(5), TimerMode::Loop));

    app.run().unwrap();
}
