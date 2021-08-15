# Bevy Compiletime Aseprite Integration

This crate allows you to integrate your aseprite files into your game project at compile time. This means that changes will break the build should you reference something nonexistent.


Here's what's included:

- Compile Time representation of your aseprite files, this includes your tags (animations) and slices (regions)!
  - Play animations by filling out the `animation` field
  - Only show a slice using the `slice` field
- Full animation capacity that's included in Aseprite, you've tweaked your animation just so? They will be shown in the same fashion in your game!
- Bevy automated reload integrated
  - Some colors are off, or an animation doesn't have that pizzaz you wanted? Simply change it in your running game, and watch as `bevy-spicy-aseprite` takes care of showing you the updated version!


**Of note**: Should you encounter any bugs or hard to use or confusing interfaces, please post an issue or bug report so that we may all profit from it!

## How to use it

1. Include this repository in your game
    `cargo add https://github.com/TheNeikos/bevy-spicy-aseprite`
2. Import your aseprite file:
    ```rust,ignore
    aseprite!(pub Player, "assets/sprites/player.aseprite");
    ```
3. This transform the macro invocation into something similar to this:
    ```rust,ignore
    mod Player {
        mod tags {
            pub const Walk = ...;
            pub const Run = ...;
        }
    }
    ```
4. Use it in your games:
    ```rust,ignore
    commands.spawn_bundle(AsepriteBundle {
        aseprite: Player::sprite(),
        animation: AsepriteAnimation::from(Player::tags::Walk),
        transform: Transform {
            scale: Vec3::splat(4.),
            translation: Vec3::new(0., 0., 0.),
            ..Default::default()
        },
        ..Default::default()
    });
    ```

## Examples

Check out the example to see how it could be used:

```bash
cargo run --example show-aseprite
```