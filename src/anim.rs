use std::{ops::Range, time::Duration};

use bevy::prelude::*;

use crate::{Aseprite, AsepriteInfo};
use bevy_aseprite_reader as reader;

/// A tag representing an animation
#[derive(Debug, Default, Component, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Component, PartialEq, Eq)]
pub struct AsepriteAnimation {
    pub is_playing: bool,
    tag: Option<String>,
    frame_range: Range<usize>,
    pub current_frame: usize,
    just_finished: bool,
    forward: bool,
    time_elapsed: Duration,
    tag_changed: bool,
}

impl Default for AsepriteAnimation {
    fn default() -> Self {
        Self {
            is_playing: true,
            tag: Default::default(),
            frame_range: 0..2,
            current_frame: Default::default(),
            forward: Default::default(),
            time_elapsed: Default::default(),
            tag_changed: true,
            just_finished: false,
        }
    }
}

impl AsepriteAnimation {
    fn reset(&mut self, info: &AsepriteInfo) {
        self.tag_changed = false;
        match &self.tag {
            Some(tag) => {
                let tag = match info.tags.get(tag) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", tag);
                        return;
                    }
                };

                self.frame_range = (tag.frames.start as usize)..(tag.frames.end as usize);
                use reader::raw::AsepriteAnimationDirection;
                match tag.animation_direction {
                    AsepriteAnimationDirection::Forward | AsepriteAnimationDirection::PingPong => {
                        self.current_frame = self.frame_range.start;
                        self.forward = true;
                    }
                    AsepriteAnimationDirection::Reverse => {
                        self.current_frame = self.frame_range.end - 1;
                        self.forward = false;
                    }
                }
            }
            None => {
                self.current_frame = 0;
                self.forward = true;
                self.frame_range = 0..info.frame_count;
            }
        }
    }

    fn next_frame(&mut self, info: &AsepriteInfo) {
        match &self.tag {
            Some(tag) => {
                let tag = match info.tags.get(tag) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", tag);
                        return;
                    }
                };

                let range = tag.frames.clone();
                match tag.animation_direction {
                    reader::raw::AsepriteAnimationDirection::Forward => {
                        let next_frame = self.current_frame + 1;
                        if range.contains(&(next_frame as u16)) {
                            self.current_frame = next_frame;
                        } else {
                            self.current_frame = range.start as usize;
                        }
                    }
                    reader::raw::AsepriteAnimationDirection::Reverse => {
                        let next_frame = self.current_frame.checked_sub(1);
                        if let Some(next_frame) = next_frame {
                            if range.contains(&((next_frame) as u16)) {
                                self.current_frame = next_frame;
                            } else {
                                self.current_frame = range.end as usize - 1;
                            }
                        } else {
                            self.current_frame = range.end as usize - 1;
                        }
                    }
                    reader::raw::AsepriteAnimationDirection::PingPong => {
                        if self.forward {
                            let next_frame = self.current_frame + 1;
                            if range.contains(&(next_frame as u16)) {
                                self.current_frame = next_frame;
                            } else {
                                self.current_frame = next_frame.saturating_sub(1);
                                self.forward = false;
                            }
                        } else {
                            let next_frame = self.current_frame.checked_sub(1);
                            if let Some(next_frame) = next_frame {
                                if range.contains(&(next_frame as u16)) {
                                    self.current_frame = next_frame
                                }
                            }
                            self.current_frame += 1;
                            self.forward = true;
                        }
                    }
                }
            }
            None => {
                self.current_frame = (self.current_frame + 1) % info.frame_count;
            }
        }
    }

    pub fn current_frame_duration(&self, info: &AsepriteInfo) -> Duration {
        Duration::from_millis(info.frame_infos[self.current_frame].delay_ms as u64)
    }

    // Returns whether the frame was changed
    pub fn update(&mut self, info: &AsepriteInfo, dt: Duration) -> bool {
        if self.tag_changed {
            self.reset(info);
            return true;
        }

        if self.is_paused() {
            return false;
        }

        self.time_elapsed += dt;
        let mut current_frame_duration = self.current_frame_duration(info);
        let mut frame_changed = false;
        while self.time_elapsed >= current_frame_duration {
            self.time_elapsed -= current_frame_duration;
            self.next_frame(info);
            self.just_finished = (self.frame_range.end - 1) == self.current_frame;
            current_frame_duration = self.current_frame_duration(info);
            frame_changed = true;
        }

        frame_changed
    }

    /// Get the current frame
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Start or resume playing an animation
    pub fn play(&mut self) {
        self.is_playing = true;
    }

    /// Pause the current animation
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// Returns `true` if the animation is playing
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Returns `true` if the animation is paused
    pub fn is_paused(&self) -> bool {
        !self.is_playing
    }

    /// Toggle state between playing and pausing
    pub fn toggle(&mut self) {
        self.is_playing = !self.is_playing;
    }

    /// Returns `true` at the very end of current animation
    pub fn just_finished(&self) -> bool {
        self.just_finished
    }
}

pub(crate) fn update_animations(
    time: Res<Time>,
    aseprites: Res<Assets<Aseprite>>,
    mut aseprites_query: Query<(
        &Handle<Aseprite>,
        &mut AsepriteAnimation,
        &mut TextureAtlasSprite,
    )>,
) {
    for (handle, mut animation, mut sprite) in aseprites_query.iter_mut() {
        animation.just_finished = false;
        let aseprite = match aseprites.get(handle) {
            Some(aseprite) => aseprite,
            None => {
                error!("Aseprite handle is invalid");
                continue;
            }
        };
        let info = match &aseprite.info {
            Some(info) => info,
            None => {
                error!("Aseprite info is None");
                continue;
            }
        };
        if animation.update(info, time.delta()) {
            sprite.index = aseprite.frame_to_idx[animation.current_frame];
        }
    }
}

impl From<&str> for AsepriteAnimation {
    fn from(tag: &str) -> AsepriteAnimation {
        AsepriteAnimation {
            tag: Some(tag.to_owned()),
            ..Default::default()
        }
    }
}

impl From<String> for AsepriteAnimation {
    fn from(tag: String) -> AsepriteAnimation {
        AsepriteAnimation {
            tag: Some(tag),
            ..Default::default()
        }
    }
}
