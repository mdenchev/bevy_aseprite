#![deny(
    missing_docs,
    non_camel_case_types,
    non_snake_case,
    path_statements,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_allocation,
    unused_import_braces,
    unused_imports,
    unused_must_use,
    unused_mut,
    while_true,
    array_into_iter,
    clippy::panic,
    clippy::print_stdout,
    clippy::todo,
    clippy::unwrap_used
)]
#![doc = include_str!("../README.MD")]

use std::ops::DerefMut;
use std::path::{Path, PathBuf};

use aseprite_reader::{Aseprite, AsepriteSliceImage, NineSlice};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::{
    asset::{AssetLoader, AssetServerSettings, LoadedAsset},
    utils::HashMap,
};

use bevy::render::texture::Extent3d;
use bevy::sprite::TextureAtlasBuilder;
pub use bevy_spicy_aseprite_derive::aseprite;

/// The required plugin to fully use your aseprite files
pub struct AsepritePlugin;

#[derive(Debug, SystemLabel, Clone, Hash, PartialEq, Eq)]
enum AsepriteSystems {
    UpdateAnim,
}

impl Plugin for AsepritePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_asset::<AsepriteImage>()
            .add_asset_loader(AsepriteLoader)
            .add_system(
                check_aseprite_data
                    .system()
                    .before(AsepriteSystems::UpdateAnim),
            )
            .add_system(load_aseprites)
            .add_system(
                update_animations
                    .system()
                    .label(AsepriteSystems::UpdateAnim),
            )
            .add_system(
                update_spritesheet_anim
                    .system()
                    .after(AsepriteSystems::UpdateAnim),
            );
    }
}

fn update_animations(
    time: Res<Time>,
    aseprite_image_assets: Res<Assets<AsepriteImage>>,
    mut aseprites_query: Query<(
        &Handle<AsepriteImage>,
        &AsepriteAnimation,
        &mut AsepriteAnimationState,
    )>,
) {
    for (aseprite_handle, aseprite_animation, mut aseprite_animation_state) in
        aseprites_query.iter_mut()
    {
        let image = if let Some(image) = aseprite_image_assets.get(aseprite_handle.clone_weak()) {
            image
        } else {
            continue;
        };

        let mut added_time = Some(time.delta().as_millis() as u64);

        loop {
            let (current_frame_idx, forward, rest_time) = match &mut *aseprite_animation_state {
                AsepriteAnimationState::Paused { .. } => break,
                AsepriteAnimationState::Playing {
                    current_frame,
                    forward,
                    time_elapsed,
                } => (current_frame, forward, time_elapsed),
            };

            let frame_info =
                if let Some(info) = image.aseprite.frame_infos().get(*current_frame_idx) {
                    info
                } else {
                    break;
                };

            if let Some(added_time) = added_time.take() {
                *rest_time += added_time;
            }

            if *rest_time >= frame_info.delay_ms as u64 {
                *rest_time -= frame_info.delay_ms as u64;

                let (next_frame_idx, switch_direction) =
                    aseprite_animation.get_next_frame(image, *current_frame_idx, *forward);

                *current_frame_idx = next_frame_idx;
                if switch_direction {
                    *forward = !*forward;
                }
            } else {
                break;
            }
        }
    }
}

fn load_aseprites(
    mut commands: Commands,
    asset_server_settings_folder: Res<AssetServerSettings>,
    asset_server: Res<AssetServer>,
    new_aseprites: Query<(Entity, &AsepriteInfo), Added<AsepriteInfo>>,
) {
    for (entity, ase_info) in new_aseprites.iter() {
        let path = ase_info
            .path()
            .strip_prefix(&asset_server_settings_folder.asset_folder)
            .unwrap();

        let handle: Handle<AsepriteImage> = asset_server.load(path);

        commands.entity(entity).insert(handle);
    }
}

fn update_spritesheet_anim(
    aseprite_assets: Res<Assets<AsepriteImage>>,
    texture_atlas_assets: Res<Assets<TextureAtlas>>,
    mut atlas_sprite: Query<(&Handle<TextureAtlas>, &mut TextureAtlasSprite)>,
    aseprites_query: Query<
        (
            &AsepriteAnimationState,
            &Handle<AsepriteImage>,
            &AsepriteSheetEntity,
        ),
        Changed<AsepriteAnimationState>,
    >,
) {
    for (aseprite_animation_state, aseprite_handle, sheet_entity) in aseprites_query.iter() {
        let frame_idx = aseprite_animation_state.get_current_frame();

        let aseprite = if let Some(aseprite) = aseprite_assets.get(aseprite_handle.clone_weak()) {
            aseprite
        } else {
            continue;
        };

        let texture = if let Some(tex) = aseprite.frames.get(frame_idx) {
            tex
        } else {
            continue;
        };

        let (atlas_handle, mut atlas_sprite) =
            if let Ok(sprite) = atlas_sprite.get_mut(sheet_entity.0) {
                sprite
            } else {
                continue;
            };

        let atlas = if let Some(atlas) = texture_atlas_assets.get(atlas_handle.clone_weak()) {
            atlas
        } else {
            continue;
        };

        atlas_sprite.index = if let Some(idx) = atlas.get_texture_index(&texture) {
            if atlas_sprite.index == idx as u32 {
                continue;
            }
            idx as u32
        } else {
            continue;
        };
    }
}

struct AsepriteSheetEntity(Entity);

fn check_aseprite_data(
    mut commands: Commands,
    mut aseprite_image_events: EventReader<AssetEvent<AsepriteImage>>,
    mut aseprite_image_assets: ResMut<Assets<AsepriteImage>>,
    mut texture_assets: ResMut<Assets<Texture>>,
    mut texture_atlas_assets: ResMut<Assets<TextureAtlas>>,
    mut existing_aseprites: Query<(
        Entity,
        Option<&AsepriteSheetEntity>,
        &mut Handle<AsepriteImage>,
    )>,
) {
    for event in aseprite_image_events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                {
                    if let Some(img) = aseprite_image_assets.get(handle) {
                        if img.atlas.is_handle() {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                let image = if let Some(img) = aseprite_image_assets.get_mut(handle) {
                    img
                } else {
                    continue;
                };

                image
                    .atlas
                    .load(&mut texture_atlas_assets, &mut texture_assets);

                for (aseprite_entity, aseprite_sheet, mut aseprite_handle) in
                    existing_aseprites.iter_mut()
                {
                    if &*aseprite_handle != handle {
                        info!("Not the same handle");
                        continue;
                    }

                    let image = if let Some(image) = aseprite_image_assets.get(&*aseprite_handle) {
                        image
                    } else {
                        info!("Not aseprite image");
                        continue;
                    };

                    let atlas_handle = if let Some(atlas_handle) = image.atlas.get_atlas() {
                        atlas_handle
                    } else {
                        info!("No texture atlas");
                        continue;
                    };

                    // Pretend we updated the handle, so we can listen to changes
                    aseprite_handle.deref_mut();

                    let sheet_entity = match &aseprite_sheet {
                        Some(AsepriteSheetEntity(entity)) => *entity,
                        None => {
                            let entity = commands
                                .spawn_bundle(SpriteSheetBundle {
                                    texture_atlas: atlas_handle.clone(),
                                    ..Default::default()
                                })
                                .id();

                            commands
                                .entity(aseprite_entity)
                                .push_children(&[entity.clone()])
                                .insert(AsepriteSheetEntity(entity));

                            entity
                        }
                    };

                    commands
                        .entity(sheet_entity)
                        .insert(TextureAtlasSprite::new(0));

                    info!("Finished inserting entities for aseprite");
                }
            }
            AssetEvent::Removed { .. } => (),
        }
    }
}

/// A tag representing an animation
#[derive(Debug, Default, Copy, Clone)]
pub struct AsepriteTag(&'static str);

impl std::ops::Deref for AsepriteTag {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsepriteTag {
    /// Create a new tag
    pub const fn new(id: &'static str) -> AsepriteTag {
        AsepriteTag(id)
    }
}

/// A tag representing an animation
#[derive(Debug, Default, Copy, Clone)]
pub struct AsepriteSlice(&'static str);

impl std::ops::Deref for AsepriteSlice {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsepriteSlice {
    /// Create a new tag
    pub const fn new(id: &'static str) -> AsepriteSlice {
        AsepriteSlice(id)
    }
}

#[derive(Debug)]
enum Atlas {
    Builder(TextureAtlasBuilder),
    Handle(Handle<TextureAtlas>),
}

impl Atlas {
    fn load(&mut self, texture_atlases: &mut Assets<TextureAtlas>, textures: &mut Assets<Texture>) {
        let handle = Handle::default();
        match self {
            Atlas::Builder(_) => {
                if let Atlas::Builder(builder) =
                    std::mem::replace(self, Atlas::Handle(handle.clone()))
                {
                    let texture_atlas = builder.finish(textures).unwrap();

                    let new_handle = texture_atlases.set(handle, texture_atlas);

                    self.set_atlas(new_handle);
                }
            }
            Atlas::Handle(_) => (),
        }
    }

    fn set_atlas(&mut self, new_handle: Handle<TextureAtlas>) {
        match self {
            Atlas::Builder(_) => (),
            Atlas::Handle(ref mut handle) => *handle = new_handle,
        }
    }

    fn get_atlas(&self) -> Option<&Handle<TextureAtlas>> {
        match self {
            Atlas::Builder(_) => None,
            Atlas::Handle(handle) => Some(handle),
        }
    }

    /// Returns `true` if the atlas is [`Handle`].
    fn is_handle(&self) -> bool {
        matches!(self, Self::Handle(..))
    }
}

#[derive(Debug)]
/// The textures in a slice
#[allow(missing_docs)]
pub struct AsepriteSliceTextures {
    pub texture_handle: Handle<Texture>,
    pub nine_patch_handles: Option<HashMap<NineSlice, Handle<Texture>>>,
}

/// An internal type containing the different images the associated aseprite file has
#[derive(Debug, TypeUuid)]
#[uuid = "8da03a16-d6d5-42c3-b4c7-fc68f53e0769"]
pub struct AsepriteImage {
    aseprite: Aseprite,
    atlas: Atlas,
    frames: Vec<Handle<Texture>>,
    slices: HashMap<String, AsepriteSliceTextures>,
}

impl AsepriteImage {
    /// Get the texture handles associated to the frames
    pub fn frames(&self) -> &[Handle<Texture>] {
        &self.frames
    }

    /// Get the slice handles associated to this aseprite
    pub fn slices(&self) -> &HashMap<String, AsepriteSliceTextures> {
        &self.slices
    }
}

/// The loader of aseprite files
#[derive(Debug, Default)]
pub struct AsepriteLoader;

impl AssetLoader for AsepriteLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::asset::BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            info!("Loading aseprite at {:?}", load_context.path());
            let aseprite = Aseprite::from_bytes(bytes)?;

            let frames = aseprite.frames();
            let images = frames
                .get_for(&(0..frames.count() as u16))
                .get_images()
                .unwrap();

            let mut aseprite_atlas = TextureAtlasBuilder::default()
                .format(bevy::render::texture::TextureFormat::Rgba8UnormSrgb);

            let mut frame_textures = vec![];

            for (idx, image) in images.into_iter().enumerate() {
                let texture = Texture::new(
                    Extent3d::new(image.width(), image.height(), 1),
                    bevy::render::texture::TextureDimension::D2,
                    image.into_raw(),
                    bevy::render::texture::TextureFormat::Rgba8UnormSrgb,
                );
                let label = format!("Frame{}", idx);
                let texture_handle =
                    load_context.set_labeled_asset(&label, LoadedAsset::new(texture.clone()));

                aseprite_atlas.add_texture(texture_handle.clone(), &texture);
                frame_textures.push(texture_handle);
            }

            info!("Finished loading aseprite");

            let slices = aseprite.slices();

            let slice_textures = slices
                .get_all()
                .map(|slice| slice.name.clone())
                .zip(
                    slices
                        .get_images(slices.get_all())?
                        .into_iter()
                        .zip(slices.get_all())
                        .map(|(AsepriteSliceImage { image, nine_slices }, slice)| {
                            let texture = Texture::new(
                                Extent3d::new(image.width(), image.height(), 1),
                                bevy::render::texture::TextureDimension::D2,
                                image.into_raw(),
                                bevy::render::texture::TextureFormat::Rgba8UnormSrgb,
                            );
                            let label = format!("Slice/{}", slice.name);
                            let texture_handle = load_context
                                .set_labeled_asset(&label, LoadedAsset::new(texture.clone()));

                            aseprite_atlas.add_texture(texture_handle.clone(), &texture);

                            let nine_patch_handles = nine_slices.map(|nine_slices| {
                                nine_slices
                                    .into_iter()
                                    .map(|(key, image_buffer)| {
                                        let texture = Texture::new(
                                            Extent3d::new(
                                                image_buffer.width(),
                                                image_buffer.height(),
                                                1,
                                            ),
                                            bevy::render::texture::TextureDimension::D2,
                                            image_buffer.into_raw(),
                                            bevy::render::texture::TextureFormat::Rgba8UnormSrgb,
                                        );

                                        let label = format!("{}{:?}", label, key);
                                        let texture_handle = load_context.set_labeled_asset(
                                            &label,
                                            LoadedAsset::new(texture.clone()),
                                        );

                                        aseprite_atlas
                                            .add_texture(texture_handle.clone(), &texture);

                                        (key, texture_handle)
                                    })
                                    .collect()
                            });

                            AsepriteSliceTextures {
                                texture_handle,
                                nine_patch_handles,
                            }
                        }),
                )
                .collect();

            load_context.set_default_asset(LoadedAsset::new(AsepriteImage {
                aseprite,
                atlas: Atlas::Builder(aseprite_atlas),
                frames: frame_textures,
                slices: slice_textures,
            }));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["ase", "aseprite"]
    }
}

/// All the info about a specific aseprite
#[derive(Debug, Default)]
pub struct AsepriteInfo {
    /// The path to the aseprite file, relative to the crate root
    pub path: PathBuf,
}

impl AsepriteInfo {
    fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug)]
/// An aseprite animation
pub enum AsepriteAnimation {
    /// The animation is defined as in this tag
    Tag {
        /// The tag defining the animation
        tag: AsepriteTag,
    },
    /// No animation playing
    None,
}

impl Default for AsepriteAnimation {
    fn default() -> Self {
        Self::None
    }
}

impl AsepriteAnimation {
    /// Calculate the next frame from the current one
    pub fn get_next_frame(
        &self,
        aseprite: &AsepriteImage,
        current_frame: usize,
        forward: bool,
    ) -> (usize, bool) {
        match self {
            AsepriteAnimation::Tag {
                tag: AsepriteTag(name),
            } => {
                let tags = aseprite.aseprite.tags();
                let tag = if let Some(tag) = tags.get_by_name(name) {
                    tag
                } else {
                    return (0, false);
                };

                let range = tag.frames.clone();
                match tag.animation_direction {
                    aseprite_reader::raw::AsepriteAnimationDirection::Forward => {
                        let next_frame = current_frame + 1;
                        if range.contains(&(next_frame as u16)) {
                            return (next_frame, false);
                        } else {
                            return (0, false);
                        }
                    }
                    aseprite_reader::raw::AsepriteAnimationDirection::Reverse => {
                        let next_frame = current_frame.checked_sub(1);
                        if let Some(next_frame) = next_frame {
                            if range.contains(&(next_frame as u16)) {
                                return (next_frame, false);
                            }
                        }
                        return (range.end as usize, false);
                    }
                    aseprite_reader::raw::AsepriteAnimationDirection::PingPong => {
                        if forward {
                            let next_frame = current_frame + 1;
                            if range.contains(&(next_frame as u16)) {
                                return (next_frame, false);
                            } else {
                                return (next_frame.saturating_sub(1), true);
                            }
                        } else {
                            let next_frame = current_frame.checked_sub(1);
                            if let Some(next_frame) = next_frame {
                                if range.contains(&(next_frame as u16)) {
                                    return (next_frame, false);
                                }
                            }
                            return (current_frame + 1, true);
                        }
                    }
                }
            }
            AsepriteAnimation::None => (0, false),
        }
    }
}

impl From<AsepriteTag> for AsepriteAnimation {
    fn from(tag: AsepriteTag) -> Self {
        AsepriteAnimation::Tag { tag }
    }
}

#[derive(Debug, Clone)]
/// Defines the current state of the animation
///
/// # Note
///
/// The default is stopped!
#[allow(missing_docs)]
pub enum AsepriteAnimationState {
    Playing {
        current_frame: usize,
        forward: bool,
        time_elapsed: u64,
    },
    Paused {
        current_frame: usize,
        forward: bool,
    },
}

impl AsepriteAnimationState {
    /// Get the current frame to be shown
    pub fn get_current_frame(&self) -> usize {
        match self {
            Self::Playing { current_frame, .. } => *current_frame,
            Self::Paused {
                current_frame: frame,
                ..
            } => *frame,
        }
    }

    /// Start playing an animation
    pub fn start(&mut self) {
        match self {
            AsepriteAnimationState::Playing { .. } => (),
            AsepriteAnimationState::Paused {
                current_frame,
                forward,
            } => {
                *self = AsepriteAnimationState::Playing {
                    current_frame: *current_frame,
                    forward: *forward,
                    time_elapsed: 0,
                }
            }
        }
    }

    /// Pause the current animation
    pub fn pause(&mut self) {
        match self {
            AsepriteAnimationState::Paused { .. } => (),
            AsepriteAnimationState::Playing {
                current_frame,
                forward,
                ..
            } => {
                *self = AsepriteAnimationState::Paused {
                    current_frame: *current_frame,
                    forward: *forward,
                }
            }
        }
    }

    /// Returns `true` if the aseprite_animation_state is [`Playing`].
    pub fn is_playing(&self) -> bool {
        matches!(self, Self::Playing { .. })
    }

    /// Returns `true` if the aseprite_animation_state is [`Paused`].
    pub fn is_paused(&self) -> bool {
        matches!(self, Self::Paused { .. })
    }

    /// Toggle state between playing and pausing
    pub fn toggle(&mut self) {
        match self {
            AsepriteAnimationState::Playing {
                current_frame,
                forward,
                ..
            } => {
                *self = Self::Paused {
                    current_frame: *current_frame,
                    forward: *forward,
                };
            }
            AsepriteAnimationState::Paused {
                current_frame,
                forward,
            } => {
                *self = Self::Playing {
                    current_frame: *current_frame,
                    forward: *forward,
                    time_elapsed: 0,
                };
            }
        }
    }
}

impl Default for AsepriteAnimationState {
    fn default() -> Self {
        AsepriteAnimationState::Playing {
            current_frame: 0,
            forward: true,
            time_elapsed: 0,
        }
    }
}

#[derive(Debug)]
/// Defines if this aseprite should be treated as a grid
pub struct AsepriteGrid {
    padding: u64,
}

/// A bundle defining a drawn aseprite
#[derive(Debug, Bundle, Default)]
#[allow(missing_docs)]
pub struct AsepriteBundle {
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub aseprite: AsepriteInfo,
    pub animation: AsepriteAnimation,
    pub animation_state: AsepriteAnimationState,
    pub handle: Handle<AsepriteImage>,
    pub grid_info: Option<AsepriteGrid>,
}
