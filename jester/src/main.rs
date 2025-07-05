use jester::prelude::*;
use rand::Rng;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Default)]
struct MainScene {
    player: Option<EntityId>,
}

impl Scene for MainScene {
    fn start(&mut self, ctx: &mut Ctx<'_>) {
        ctx.spawn_camera(Camera::pixel_perfect(ctx.screen_pos.x, ctx.screen_pos.y));

        let aseprite_id = ctx.load_asset("assets/aseprite.png");
        let samurai_id = ctx.load_asset("assets/samurai.png");

        let player_entity = ctx.spawn_sprite(Sprite {
            transform: Transform::from_xy(0.0, 0.0).with_scale(Vec2::splat(2.0)),
            tex: samurai_id,
            ..Default::default()
        });
        let mut rng = rand::rng();

        for _ in 0..2000 {
            let pos = Vec2::new(
                rng.random_range(-ctx.screen_pos.x..ctx.screen_pos.x),
                rng.random_range(-ctx.screen_pos.y..ctx.screen_pos.y),
            );
            let _ = ctx.spawn_sprite(Sprite {
                transform: Transform::from_xy(pos.x, pos.y).with_scale(Vec2::splat(2.0)),
                tex: aseprite_id,
                ..Default::default()
            });
        }
        self.player = Some(player_entity);
    }

    fn update(&mut self, ctx: &mut Ctx<'_>) {
        let Some(player) = self.player else {
            warn!("Player entity not found");
            return;
        };
        let Some(player_sprite) = ctx.pool.sprite_mut(player) else {
            return;
        };

        let Some(fps_timer) = ctx.resources.get_mut::<FpsTimer>() else {
            return;
        };

        if fps_timer.0.tick(Duration::from_secs_f32(ctx.dt)) {
            if let Some(stats) = ctx.resources.get::<FpsStats>() {
                info!(
                    "Avg FPS {:.1} – Avg frame {:.2} ms",
                    stats.fps, stats.frame_ms
                );
            }
        }

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

struct FpsTimer(Timer);

fn main() {
    tracing_subscriber::fmt::init();

    let mut app = App::new("cool game".to_string());
    app.add_scene(MainScene::default());
    app.set_start_scene::<MainScene>();

    app.add_resource(Timer::new(Duration::from_secs(5), TimerMode::Loop));
    app.add_resource(FpsTimer(Timer::new(
        Duration::from_secs(1),
        TimerMode::Loop,
    )));
    app.add_resource(FpsStats::default());

    app.run().unwrap();
}
