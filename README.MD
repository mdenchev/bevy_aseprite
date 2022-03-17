# Bevy Aseprite Parser and Loader

This is a fork of TheNeikos/bevy_spicy_aseprite.

Works with Bevy 0.6


## How to use it without derives

```rust,ignore
    commands.spawn_bundle(AsepriteBundle {
        aseprite: asset_server.load("player.ase"),
        animation: AsepriteAnimation::from("walk"),
        transform: Transform {...},
        ..Default::default()
    });
```
    

## How to use it with derive (for compile time validation)

```rust,ignore
    mod sprites {
      use bevy_aseprite::aseprite;
      aseprite!(pub Player, "player.ase");
    }

    ...

    commands.spawn_bundle(AsepriteBundle {
        aseprite: asset_server.load(sprites::Player::PATH),
        animation: AsepriteAnimation::from(sprites::Player::tags::LEFT_WALK),
        transform: Transform {...},
        ..Default::default()
    });
```

## Examples

Check out the example to see how it could be used:

```bash
cargo run --example show_aseprite
```
