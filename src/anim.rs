use std::time::Duration;

use bevy::prelude::*;

use crate::{AsepriteInfo, Aseprite};
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

#[derive(Debug, Default, Component, PartialEq, Eq)]
pub struct AsepriteAnimation {
    pub is_playing: bool,
    pub tag: Option<&'static str>,
    pub current_frame: usize,
    pub forward: bool,
    pub time_elapsed: u64,
}

impl AsepriteAnimation {
    /// Return the first frame of the tag or 0 if no tag
    pub fn get_first_frame(&self, info: &AsepriteInfo) -> usize {
        match self.tag {
            Some(tag) => {
                let tag = match info.tags.get(tag) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", tag);
                        return 0;
                    }
                };

                let range = tag.frames.clone();
                range.start as usize
            }
            _ => 0,
        }
    }

    fn next_frame(&mut self, info: &AsepriteInfo) {
        match self.tag {
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
                            if range.contains(&(next_frame as u16)) {
                                self.current_frame = next_frame;
                            }
                        } else {
                            // TODO check -1 is correct
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

    //pub fn current_frame_duration(&self, info: &AsepriteInfo) -> Duration {
    //    //let frame_info = ase.info.frame_duration(self.current_frame)
    //}

    pub fn update(&mut self, info: &AsepriteInfo, dt: Duration) {
        //self.time_elapsed += dt.as_millis();
        //while time_elapsed >= aseprite.frame_time()

        //    let mut time_elapsed = ase_anim.time_elpased + delta_millis;
        //    let mut current_frame = ase_anim.current_frame;
        //    while time_elapsed >= frame_info.delay_ms {
        //        time_elapsed -= frame_info.delay_ms;
        //        current_frame
        //    }
    }

    /// Check if the current animation tag is the one provided
    //pub fn is_tag(&self, tag: &str) -> bool {
    //    self.tag == Some(tag)
    //}

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
}

pub(crate) fn update_animations(
    time: Res<Time>,
    aseprites: Res<Assets<Aseprite>>,
    mut aseprites_query: Query<(&Handle<Aseprite>, &mut AsepriteAnimation)>,
) {
    for (handle, mut anim_state) in aseprites_query.iter_mut() {
        //anim_state.update(ase, time.delta());
    }
}
