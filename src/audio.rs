#[cfg(target_arch = "wasm32")]
use crate::async_utils::wait_once;

use quad_snd::{AudioContext, PlaySoundParams, Sound};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Music {
    Battle,
    Boss,
    Dungeon,
    Ending,
    Intro,
    Overworld,
    Town,
}

pub struct Audio {
    audio_context: AudioContext,
    music: Option<(Music, Sound)>,
}

impl Music {
    fn sound_data(&self) -> &'static [u8] {
        match self {
            Self::Battle => include_bytes!("../assets/battle.ogg"),
            Self::Boss => include_bytes!("../assets/boss.ogg"),
            Self::Dungeon => include_bytes!("../assets/dungeon.ogg"),
            Self::Ending => include_bytes!("../assets/ending.ogg"),
            Self::Intro => include_bytes!("../assets/intro.ogg"),
            Self::Overworld => include_bytes!("../assets/overworld.ogg"),
            Self::Town => include_bytes!("../assets/town.ogg"),
        }
    }

    fn base_volume(&self) -> f32 {
        match self {
            Self::Boss => 0.544,
            _ => 1.0,
        }
    }
}

impl From<&str> for Music {
    fn from(s: &str) -> Self {
        match s {
            "Battle" => Self::Battle,
            "Boss" => Self::Boss,
            "Dungeon" => Self::Dungeon,
            "Ending" => Self::Ending,
            "Intro" => Self::Intro,
            "Overworld" => Self::Overworld,
            "Town" => Self::Town,
            _ => panic!("unknown music: {s}"),
        }
    }
}

impl Audio {
    pub fn new() -> Self {
        Self {
            audio_context: AudioContext::new(),
            music: None,
        }
    }

    pub async fn play_music(&mut self, music: Option<Music>) {
        if let Some((current, sound)) = &self.music {
            if music.map(|m| m != *current).unwrap_or(true) {
                sound.stop(&self.audio_context);
                sound.delete(&self.audio_context);
                self.music = None;
            }
        }

        if let Some(music) = music {
            if let Some((current_music, _)) = &self.music {
                if music == *current_music {
                    return;
                }
            }

            let sound = Sound::load(&self.audio_context, music.sound_data());
            #[cfg(target_arch = "wasm32")]
            while !sound.is_loaded() {
                wait_once().await;
            }

            sound.play(
                &self.audio_context,
                PlaySoundParams {
                    looped: true,
                    volume: music.base_volume(),
                },
            );
            self.music = Some((music, sound));
        }
    }
}
