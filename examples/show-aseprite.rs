use bevy::prelude::*;
use bevy_spicy_aseprite::{AsepriteAnimation, AsepriteBundle, AsepritePlugin};

mod sprites {
    use bevy_spicy_aseprite::aseprite;

    aseprite!(pub Crow, "assets/crow.aseprite");
}

fn main() {
    tracing_log::LogTracer::init().unwrap();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(AsepritePlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    asset_server.watch_for_changes().unwrap();

    let font = asset_server.load("Share-Regular.ttf");

    let text_style = TextStyle {
        font,
        font_size: 30.,
        ..Default::default()
    };

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(AsepriteBundle {
        aseprite: sprites::Crow::sprite(),
        animation: AsepriteAnimation::from(sprites::Crow::tags::FlapWings),
        transform: Transform {
            scale: Vec3::splat(4.),
            translation: Vec3::new(0., -200., 0.),
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn_bundle(AsepriteBundle {
        aseprite: sprites::Crow::sprite(),
        animation: AsepriteAnimation::from(sprites::Crow::tags::Groove),
        transform: Transform {
            scale: Vec3::splat(4.),
            translation: Vec3::new(0., 0., 0.),
            ..Default::default()
        },
        ..Default::default()
    });
    commands.spawn_bundle(Text2dBundle {
        text: Text {
            alignment: TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
            sections: vec![
                TextSection {
                    value: String::from("The crow was made by "),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from("meitdev"),
                    style: TextStyle {
                        color: Color::LIME_GREEN,
                        ..text_style.clone()
                    },
                },
                TextSection {
                    value: String::from(" on itch.io"),
                    style: TextStyle {
                        color: Color::WHITE,
                        ..text_style.clone()
                    },
                },
            ],
            ..Default::default()
        },
        transform: Transform::from_translation(Vec3::new(0., 300., 0.)),
        ..Default::default()
    });
}
// Made by https://meitdev.itch.io/crow
