# Jester

Minimal 2D game engine written in Rust.

## Inspirations

- [Bevy](https://bevyengine.org/)


## Why?

This project was made to understand better how to use Vulkan and graphics programming.
A big part of the implementation is naive and probably not optimal.

do not use! :)


## Usage

**Warning the API is changing and is really rough at the moment**

```rust
use jester::prelude::*;
use tracing::{warn};

#[derive(Default)]
struct MainScene {
    player: Option<EntityId>,
}

impl Scene for MainScene {
    fn start(&mut self, ctx: &mut Ctx<'_>) {
        ctx.spawn_camera(Camera::pixel_perfect(ctx.screen_pos.x, ctx.screen_pos.y));
        let player_tex = ctx.load_asset("assets/aseprite.png");
        let entity = ctx.spawn_sprite(Sprite {
            tex: player_tex,
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
    app.add_scene(MainScene::default());
    app.set_start_scene::<MainScene>(); // Optional

    app.run().unwrap();
}
```
