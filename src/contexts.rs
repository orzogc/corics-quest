use crate::actor::*;
use crate::async_utils::*;
use crate::audio::*;
use crate::direction::*;
use crate::enemy::*;
use crate::fade::*;
use crate::get_gctx;
use crate::input::*;
use crate::levels::*;
use crate::modes::*;
use crate::progress::*;
use crate::random::*;
use crate::resources::*;

use miniquad::GlContext;

macro_rules! update_mode {
    ($name:ident, $event:ident) => {
        pub async fn $name(&mut self) -> $event {
            let gctx = get_gctx();

            self.modes
                .$name(&mut ModeContext {
                    gctx,
                    res: &self.res,
                    input: &self.input,
                    audio: &mut self.audio,
                    rng: &mut self.rng,
                    progress: &mut self.progress,
                    level: &self.level,
                    actors: &mut self.actors,
                    fade: &mut self.fade,
                    encounter_steps: &mut self.encounter_steps,
                })
                .await
        }
    };
}

pub struct DrawContext<'a, 'g> {
    pub gctx: &'g mut GlContext,
    pub level: &'a SharedMut<Level>,
    pub actors: &'a SharedMut<Vec<Actor>>,
}

pub struct ModeContext<'a, 'g> {
    pub gctx: &'g mut GlContext,
    pub res: &'a Resources,
    pub input: &'a SharedMut<Input>,
    pub audio: &'a mut Audio,
    pub rng: &'a mut Rng,
    pub progress: &'a mut Progress,
    pub level: &'a SharedMut<Level>,
    pub actors: &'a mut SharedMut<Vec<Actor>>,
    pub fade: &'a mut SharedMut<Fade>,
    pub encounter_steps: &'a mut i32,
}

pub struct ScriptContext {
    pub res: Resources,
    pub input: SharedMut<Input>,
    pub audio: SharedMut<Audio>,
    pub modes: SharedMut<ModeStack>,
    pub rng: Rng,
    pub progress: Progress,
    pub level: SharedMut<Level>,
    pub actors: SharedMut<Vec<Actor>>,
    pub fade: SharedMut<Fade>,
    pub encounter_steps: i32,
    pub confirm_save_overwrite: bool,
}

impl ScriptContext {
    pub fn new(
        res: Resources,
        input: &SharedMut<Input>,
        audio: &SharedMut<Audio>,
        modes: &SharedMut<ModeStack>,
        level: &SharedMut<Level>,
        actors: &SharedMut<Vec<Actor>>,
        fade: &SharedMut<Fade>,
    ) -> Self {
        let mut rng = Rng::new(miniquad::date::now() as _);
        let encounter_steps = 15 + rng.random(26) as i32;

        // SAFETY: This is immediately sent into the async script function.
        // Access from outside that async function never goes through this.
        unsafe {
            Self {
                res,
                input: SharedMut::clone(input),
                audio: SharedMut::clone(audio),
                modes: SharedMut::clone(modes),
                rng,
                progress: Progress::new(),
                level: SharedMut::clone(level),
                actors: SharedMut::clone(actors),
                fade: SharedMut::clone(fade),
                encounter_steps,
                confirm_save_overwrite: true,
            }
        }
    }

    fn prepare_level_and_actors(&self, level: &mut Level, actors: &mut [Actor]) {
        let gctx = get_gctx();

        // display chest according to open/closed state in progress
        if let Some(chest) = actors.iter_mut().find(|a| a.identifier == ActorType::Chest) {
            let chest_opened = self
                .progress
                .collected_chests
                .iter()
                .map(String::as_str)
                .any(|s| s == level.identifier);
            chest.start_animation(if chest_opened { "open" } else { "closed" });
        }

        // show lever turned to its last position and update the map tiles it controls
        let lever_turned = self
            .progress
            .turned_levers
            .iter()
            .any(|l| l == level.identifier.as_str());
        sync_level_and_actors_with_lever(gctx, lever_turned, level, actors);
    }

    pub fn level_by_identifier(&self, identifier: &str) -> (Level, Vec<Actor>) {
        let gctx = get_gctx();

        let (mut level, mut actors) = self
            .res
            .levels
            .level_by_identifier(gctx, &self.res, identifier);
        self.prepare_level_and_actors(&mut level, &mut actors[..]);
        (level, actors)
    }

    pub fn level_by_neighbour(&self, dir: Direction) -> Option<(Level, Vec<Actor>)> {
        let gctx = get_gctx();

        let Actor { grid_x, grid_y, .. } = self.actors[0];
        let Level {
            px_world_x,
            px_world_y,
            ..
        } = *self.level;

        self.res
            .levels
            .level_by_neighbour(
                gctx,
                &self.res,
                &self.level.neighbours[..],
                px_world_x + grid_x * TILE_SIZE,
                px_world_y + grid_y * TILE_SIZE,
                dir,
            )
            .map(|(mut level, mut actors)| {
                self.prepare_level_and_actors(&mut level, &mut actors[..]);
                (level, actors)
            })
    }

    pub fn lever_is_turned(&self) -> bool {
        self.progress
            .turned_levers
            .iter()
            .any(|l| l == self.level.identifier.as_str())
    }

    pub fn toggle_lever(&mut self) {
        let gctx = get_gctx();

        if self.lever_is_turned() {
            let turned_lever_pos = self
                .progress
                .turned_levers
                .iter()
                .position(|l| l == self.level.identifier.as_str())
                .expect("turned lever position");
            self.progress.turned_levers.swap_remove(turned_lever_pos);
        } else {
            self.progress
                .turned_levers
                .push(self.level.identifier.clone());
        }

        sync_level_and_actors_with_lever(
            gctx,
            self.lever_is_turned(),
            &mut self.level,
            &mut self.actors[..],
        );
    }

    pub fn place_gates(&mut self, tile_x: i32, tile_y: i32) {
        let gctx = get_gctx();

        self.level.place_gates(gctx, tile_x, tile_y);
    }

    pub fn pop_mode(&mut self) {
        self.modes.pop();
    }

    pub fn push_battle_mode(&mut self, enemy: Enemy, boss_fight: bool) {
        let gctx = get_gctx();

        self.modes.push(Battle::new(
            gctx,
            &self.res,
            self.progress.max_hp,
            self.progress.max_mp,
            enemy,
            boss_fight,
        ));
    }

    pub fn push_credits_mode(&mut self) {
        let gctx = get_gctx();

        self.modes.push(Credits::new(gctx, &self.res));
    }

    pub fn push_debug_menu_mode(&mut self) {
        let gctx = get_gctx();

        self.modes.push(DebugMenu::new(gctx, &self.res));
    }

    pub fn push_ending_mode(&mut self) {
        let gctx = get_gctx();

        self.modes.push(Ending::new(gctx, &self.res));
    }

    pub fn push_intro_mode(&mut self) {
        let gctx = get_gctx();

        self.modes.push(Intro::new(gctx, &self.res));
    }

    pub fn push_main_menu_mode(&mut self, status_only: bool) {
        let gctx = get_gctx();

        self.modes
            .push(MainMenu::new(gctx, &self.res, &self.progress, status_only));
    }

    pub fn push_options_mode(&mut self, base_x: i32, base_y: i32, preview_music: bool) {
        let gctx = get_gctx();

        self.modes
            .push(Options::new(gctx, &self.res, base_x, base_y, preview_music));
    }

    pub fn push_text_box_mode(&mut self, s: &str) {
        let gctx = get_gctx();

        self.modes.push(TextBox::new(gctx, &self.res, s));
    }

    pub fn push_title_mode(&mut self) {
        let gctx = get_gctx();

        self.modes.push(Title::new(gctx, &self.res));
    }

    pub fn push_walk_around_mode(&mut self) {
        self.modes.push(WalkAround::new());
    }

    pub fn push_yes_no_prompt_mode(
        &mut self,
        prompt: &str,
        yes_label: &str,
        no_label: &str,
        initial_choice: bool,
    ) {
        let gctx = get_gctx();

        self.modes.push(YesNoPrompt::new(
            gctx,
            &self.res,
            prompt,
            yes_label,
            no_label,
            initial_choice,
        ));
    }

    update_mode!(update_battle_mode, BattleEvent);
    update_mode!(update_credits_mode, CreditsEvent);
    update_mode!(update_debug_menu_mode, DebugMenuEvent);
    update_mode!(update_ending_mode, EndingEvent);
    update_mode!(update_intro_mode, IntroEvent);
    update_mode!(update_main_menu_mode, MainMenuEvent);
    update_mode!(update_options_mode, OptionsEvent);
    update_mode!(update_text_box_mode, TextBoxEvent);
    update_mode!(update_title_mode, TitleEvent);
    update_mode!(update_walk_around_mode, WalkAroundEvent);
    update_mode!(update_yes_no_prompt_mode, YesNoPromptEvent);
}

fn sync_level_and_actors_with_lever(
    gctx: &mut GlContext,
    lever_turned: bool,
    level: &mut Level,
    actors: &mut [Actor],
) {
    if let Some(lever) = actors.iter_mut().find(|a| a.identifier == ActorType::Lever) {
        lever.start_animation(if lever_turned { "right" } else { "left" });
    }

    level.sync_props_with_lever(gctx, lever_turned);
}
