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
        let player_tex = ctx.load_asset("assets/aseprite.png");
        let entity = ctx.spawn_sprite(Sprite {
            transform: Transform {
                translation: Vec2::new(400.0, 300.0),
                scale: Vec2::new(128.0, 128.0),
                ..Default::default()
            },
            uv: [0.0, 0.0, 1.0, 1.0],
            tex: player_tex,
        });
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
                sprite.transform.translation.y += SPEED * ctx.dt;
            }
        }
        if ctx.input.key_pressed(KeyCode::KeyS) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.transform.translation.y -= SPEED * ctx.dt;
            }
        }
        if ctx.input.key_pressed(KeyCode::KeyA) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.transform.translation.x -= SPEED * ctx.dt;
            }
        }
        if ctx.input.key_pressed(KeyCode::KeyD) {
            if let Some(sprite) = ctx.pool.sprite_mut(player) {
                sprite.transform.translation.x += SPEED * ctx.dt;
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut app = App::new("cool game".to_string());
    app.add_camera(Camera::default());
    app.add_scene(MainScene::default());
    app.set_start_scene::<MainScene>(); // Optional

    app.run().unwrap();
}
```
