# Aseprite Reader

> ❕ Note: This, `aseprite-reader2`, is a fork of https://github.com/TheNeikos/aseprite-reader.

`aseprite-reader2` is a parsing crate for `.aseprite` files, made by the [Aseprite Editor](https://www.aseprite.org/).

It's focus is on speed and completeness[^1].

It exports a main [`Aseprite`] type, through which the parsed contents can be accessed.



[^1]: Currently embedded ICC profiles are not supported



## Examples

```rust
use aseprite_reader::Aseprite;

fn load_character() {
    let aseprite = Aseprite::from_path("assets/sprites/character.aseprite")
        .expect("Could not read aseprite file.");

    let tags = aseprite.tags();

    let walk_tag = &tags["walk"];
    println!("This tag uses these frames: {:?}", walk_tag.frames); // `.frames` is a range

    let all_frames = aseprite.frames();
    let frames = all_frames.get_for(&walk_tag.frames);
    let images = frames.get_images();
}
```