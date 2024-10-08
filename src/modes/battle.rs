use crate::async_utils::wait_once;
use crate::audio::*;
use crate::contexts::*;
use crate::enemy::*;
use crate::input::*;
use crate::meter::*;
use crate::progress::*;
use crate::random::*;
use crate::resources::*;
use crate::sprite::*;
use crate::text::*;
use crate::window::*;

use miniquad::GlContext;

const ENEMY_X: i32 = 104;
const ENEMY_Y: i32 = 32;
const MESSAGE_X: i32 = 18;
const MESSAGE_Y: i32 = 116;
const STATUS_X: i32 = 250;
const STATUS_Y: i32 = 116;

pub struct Battle {
    enemy_window: Window,
    enemy_sprite: Sprite,
    enemy_hp_meter: Meter,
    enemy_visible: bool,
    message_window: Window,
    message_text: Text,
    menu_text: Text,
    cursor: Text,
    menu_visible: bool,
    status_window: Window,
    status_visible: bool,
    hp_text: Text,
    mp_text: Text,
    hp_meter: Meter,
    mp_meter: Meter,
    change_window: Window,
    change_text: Text,
    change_visible: bool,
    enemy: Enemy,
    enemy_max_hp: i32,
    boss_fight: bool,
}

pub enum BattleEvent {
    Defeat,
    RanAway,
    Victory,
}

enum PlayerChoice {
    Fight,
    Magic(usize),
    Item(usize),
    Run,
}

impl Battle {
    pub fn new(
        gctx: &mut GlContext,
        res: &Resources,
        max_hp: i32,
        max_mp: i32,
        enemy: Enemy,
        boss_fight: bool,
    ) -> Self {
        let mut enemy_sprite = Sprite::new(gctx, res, enemy.sprite_path);
        enemy_sprite.start_animation("idle");

        let enemy_max_hp = enemy.hp;

        Self {
            enemy_window: Window::new(gctx, res, ENEMY_X, ENEMY_Y, 112, 80),
            enemy_sprite,
            enemy_hp_meter: Meter::new(
                gctx,
                res,
                ENEMY_X + 40,
                ENEMY_Y + 18,
                32,
                [0, 192, 0],
                enemy.hp,
            ),
            enemy_visible: true,
            message_window: Window::new(gctx, res, MESSAGE_X, MESSAGE_Y, 232, 48),
            message_text: Text::new(res, MESSAGE_X + 8, MESSAGE_Y + 8),
            menu_text: Text::new(res, MESSAGE_X + 8, MESSAGE_Y + 24),
            cursor: Text::from_str(gctx, res, MESSAGE_X + 8, MESSAGE_Y + 32, "►"),
            menu_visible: false,
            status_window: Window::new(gctx, res, STATUS_X, STATUS_Y, 52, 48),
            status_visible: true,
            hp_text: Text::new(res, STATUS_X + 8, STATUS_Y + 8),
            mp_text: Text::new(res, STATUS_X + 8, STATUS_Y + 24),
            hp_meter: Meter::new(
                gctx,
                res,
                STATUS_X + 8,
                STATUS_Y + 17,
                36,
                [0, 192, 0],
                max_hp,
            ),
            mp_meter: Meter::new(
                gctx,
                res,
                STATUS_X + 8,
                STATUS_Y + 33,
                36,
                [0, 192, 192],
                max_mp,
            ),
            change_window: Window::new(gctx, res, 0, 0, 16, 24),
            change_text: Text::new(res, 0, 0),
            change_visible: false,
            enemy,
            enemy_max_hp,
            boss_fight,
        }
    }

    async fn action_menu(
        &mut self,
        mctx: &mut ModeContext<'_, '_>,
        has_follow_up: bool,
    ) -> PlayerChoice {
        fn update_action_cursor(cursor: &mut Text, which: usize) {
            //           1         2         3
            // 012345678901234567890123456789012345
            // .Fight     .Magic    .Item    .Run
            const ACTION_POSITIONS: [i32; 4] = [0, 11, 21, 30];
            let x = ACTION_POSITIONS[which] * 6;
            let y = 8;
            cursor.set_offset(MESSAGE_X + 8 + x, MESSAGE_Y + 24 + y);
        }

        fn set_all_text(
            mctx: &mut ModeContext,
            message_text: &mut Text,
            menu_text: &mut Text,
            enemy_name: &str,
            has_follow_up: bool,
        ) {
            message_text.set_text(
                mctx.gctx,
                mctx.res,
                &format!(
                    "{} prepares to fight!\nChoose your course of action:",
                    enemy_name,
                ),
            );

            menu_text.set_text(
                mctx.gctx,
                mctx.res,
                &format!(
                    "\n {:8}   Magic     Item     Run",
                    if has_follow_up { "FollowUp" } else { "Fight" },
                ),
            );
        }

        set_all_text(
            mctx,
            &mut self.message_text,
            &mut self.menu_text,
            self.enemy.name,
            has_follow_up,
        );
        self.message_text.reveal().await;
        self.menu_visible = true;

        let mut selection = 0;
        update_action_cursor(&mut self.cursor, selection);

        loop {
            self.enemy_sprite.animate();

            wait_once().await;

            if mctx.input.is_key_pressed(GameKey::Confirm) {
                let choice = match selection {
                    0 => Some(PlayerChoice::Fight),
                    1 => self.magic_menu(mctx).await.map(PlayerChoice::Magic),
                    2 => self.item_menu(mctx).await.map(PlayerChoice::Item),
                    3 => Some(PlayerChoice::Run).filter(|_| !self.boss_fight),
                    _ => None,
                };
                if let Some(choice) = choice {
                    self.menu_visible = false;
                    return choice;
                } else {
                    mctx.audio.play_sfx(Sfx::Cancel);
                    set_all_text(
                        mctx,
                        &mut self.message_text,
                        &mut self.menu_text,
                        self.enemy.name,
                        has_follow_up,
                    );
                    update_action_cursor(&mut self.cursor, selection);
                }
            } else if mctx.input.is_key_pressed(GameKey::Left) {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    0 => selection = 3,
                    _ => selection -= 1,
                }
                update_action_cursor(&mut self.cursor, selection);
            } else if mctx.input.is_key_pressed(GameKey::Right) {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    3 => selection = 0,
                    _ => selection += 1,
                }
                update_action_cursor(&mut self.cursor, selection);
            }
        }
    }

    pub fn draw(&self, dctx: &mut DrawContext) {
        self.enemy_window.draw(dctx.gctx);
        if self.enemy_visible {
            self.enemy_sprite
                .draw(dctx.gctx, ENEMY_X + 40, ENEMY_Y + 24);
            self.enemy_hp_meter.draw(dctx.gctx);
        }
        self.message_window.draw(dctx.gctx);
        self.message_text.draw(dctx.gctx);
        if self.menu_visible {
            self.menu_text.draw(dctx.gctx);
            self.cursor.draw(dctx.gctx);
        }
        if self.status_visible {
            self.status_window.draw(dctx.gctx);
            self.hp_text.draw(dctx.gctx);
            self.mp_text.draw(dctx.gctx);
            self.hp_meter.draw(dctx.gctx);
            self.mp_meter.draw(dctx.gctx);
        }
        if self.change_visible {
            self.change_window.draw(dctx.gctx);
            self.change_text.draw(dctx.gctx);
        }
    }

    async fn enemy_hit_animation(&mut self, mctx: &mut ModeContext<'_, '_>, damage: i32) {
        for _ in 0..5 {
            self.enemy_visible = false;
            wait_once().await;
            wait_once().await;
            self.enemy_visible = true;
            wait_once().await;
            wait_once().await;
        }
        self.enemy_hp_meter
            .set_value(mctx.gctx, self.enemy.hp - damage);
        self.show_change_text_at(mctx, ENEMY_X + 56, ENEMY_Y + 16, &format!("{damage}"));
    }

    async fn item_menu(&mut self, mctx: &mut ModeContext<'_, '_>) -> Option<usize> {
        fn update_item_cursor(cursor: &mut Text, which: usize) {
            //           1         2         3
            // 012345678901234567890123456789012345
            // .Back  .Salve     1   .Tonic     2
            //        .XSalve    3   .XTonic    4
            const ITEM_POSITIONS: [(i32, i32); 5] = [(0, 0), (7, 0), (22, 0), (7, 1), (22, 1)];
            let x = ITEM_POSITIONS[which].0 * 6;
            let y = ITEM_POSITIONS[which].1 * 8;
            cursor.set_offset(MESSAGE_X + 8 + x, MESSAGE_Y + 24 + y);
        }

        mctx.audio.play_sfx(Sfx::Confirm);

        self.message_text.set_text(mctx.gctx, mctx.res, "Use item:");
        self.menu_text.set_text(
            mctx.gctx,
            mctx.res,
            &format!(
                " Back   {:11.11}    {:11.11}\n        {:11.11}    {:11.11}",
                mctx.progress.items[0].battle_menu_entry(),
                mctx.progress.items[1].battle_menu_entry(),
                mctx.progress.items[2].battle_menu_entry(),
                mctx.progress.items[3].battle_menu_entry(),
            ),
        );

        let mut selection = 0;
        update_item_cursor(&mut self.cursor, selection);

        loop {
            self.enemy_sprite.animate();

            wait_once().await;

            if mctx.input.is_key_pressed(GameKey::Cancel) {
                return None;
            } else if mctx.input.is_key_pressed(GameKey::Confirm) {
                if selection == 0 {
                    return None;
                } else {
                    let choice = selection - 1;
                    if mctx.progress.items[choice].amount > 0 {
                        return Some(choice);
                    } else {
                        mctx.audio.play_sfx(Sfx::Cancel);
                    }
                }
            } else if mctx.input.is_key_pressed(GameKey::Up)
                || mctx.input.is_key_pressed(GameKey::Down)
            {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    0 => {}
                    1 | 2 => selection += 2,
                    3 | 4 => selection -= 2,
                    _ => unreachable!(),
                }
                update_item_cursor(&mut self.cursor, selection);
            } else if mctx.input.is_key_pressed(GameKey::Left) {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    0 => selection = 2,
                    3 => selection = 4,
                    1 | 2 | 4 => selection -= 1,
                    _ => unreachable!(),
                }
                update_item_cursor(&mut self.cursor, selection);
            } else if mctx.input.is_key_pressed(GameKey::Right) {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    2 => selection = 0,
                    4 => selection = 3,
                    0 | 1 | 3 => selection += 1,
                    _ => unreachable!(),
                }
                update_item_cursor(&mut self.cursor, selection);
            }
        }
    }

    async fn magic_menu(&mut self, mctx: &mut ModeContext<'_, '_>) -> Option<usize> {
        fn update_magic_cursor(cursor: &mut Text, which: usize) {
            //           1         2         3
            // 012345678901234567890123456789012345
            // .Back  .Heal      5MP .EarthEdge 2MP
            //        .WaterEdge 5MP .FireEdge  2MP
            const MAGIC_POSITIONS: [(i32, i32); 5] = [(0, 0), (7, 0), (22, 0), (7, 1), (22, 1)];
            let x = MAGIC_POSITIONS[which].0 * 6;
            let y = MAGIC_POSITIONS[which].1 * 8;
            cursor.set_offset(MESSAGE_X + 8 + x, MESSAGE_Y + 24 + y);
        }

        mctx.audio.play_sfx(Sfx::Confirm);

        self.message_text
            .set_text(mctx.gctx, mctx.res, "Cast magic:");
        self.menu_text.set_text(
            mctx.gctx,
            mctx.res,
            &format!(
                " Back   {:13.13}  {:13.13}\n        {:13.13}  {:13.13}",
                mctx.progress.magic[0].battle_menu_entry(),
                mctx.progress.magic[1].battle_menu_entry(),
                mctx.progress.magic[2].battle_menu_entry(),
                mctx.progress.magic[3].battle_menu_entry(),
            ),
        );

        let mut selection = 0;
        update_magic_cursor(&mut self.cursor, selection);

        loop {
            self.enemy_sprite.animate();

            wait_once().await;

            if mctx.input.is_key_pressed(GameKey::Cancel) {
                return None;
            } else if mctx.input.is_key_pressed(GameKey::Confirm) {
                if selection == 0 {
                    return None;
                } else {
                    let choice = selection - 1;
                    let magic_known = mctx.progress.magic[choice].known;
                    let mp_cost = mctx.progress.magic[choice].magic.mp_cost();
                    if magic_known && mctx.progress.mp >= mp_cost {
                        return Some(choice);
                    } else {
                        mctx.audio.play_sfx(Sfx::Cancel);
                    }
                }
            } else if mctx.input.is_key_pressed(GameKey::Up)
                || mctx.input.is_key_pressed(GameKey::Down)
            {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    0 => {}
                    1 | 2 => selection += 2,
                    3 | 4 => selection -= 2,
                    _ => unreachable!(),
                }
                update_magic_cursor(&mut self.cursor, selection);
            } else if mctx.input.is_key_pressed(GameKey::Left) {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    0 => selection = 2,
                    3 => selection = 4,
                    1 | 2 | 4 => selection -= 1,
                    _ => unreachable!(),
                }
                update_magic_cursor(&mut self.cursor, selection);
            } else if mctx.input.is_key_pressed(GameKey::Right) {
                mctx.audio.play_sfx(Sfx::Cursor);
                match selection {
                    2 => selection = 0,
                    4 => selection = 3,
                    0 | 1 | 3 => selection += 1,
                    _ => unreachable!(),
                }
                update_magic_cursor(&mut self.cursor, selection);
            }
        }
    }

    fn show_change_text_at(
        &mut self,
        mctx: &mut ModeContext,
        middle_x: i32,
        bottom_y: i32,
        msg: &str,
    ) {
        self.change_text.set_text(mctx.gctx, mctx.res, msg);
        let text_width = self.change_text.width();
        let text_height = self.change_text.height();
        self.change_text
            .set_offset(middle_x - text_width / 2, bottom_y - (text_height + 6));
        self.change_window
            .resize(mctx.gctx, text_width + 12, text_height + 12);
        self.change_window.set_offset(
            middle_x - (text_width + 12) / 2,
            bottom_y - (text_height + 12),
        );
        self.change_visible = true;
    }

    fn show_status_change(&mut self, mctx: &mut ModeContext, msg: &str) {
        self.show_change_text_at(mctx, STATUS_X + 26, STATUS_Y + 6, msg);
    }

    pub async fn update(&mut self, mctx: &mut ModeContext<'_, '_>) -> BattleEvent {
        let mut follow_up: Option<(Magic, usize)> = None;

        self.update_status(mctx);

        mctx.audio
            .play_music(Some(if self.boss_fight {
                Music::Boss
            } else {
                Music::Battle
            }))
            .await;

        loop {
            if let Some((magic, turns)) = follow_up {
                follow_up = if turns > 0 {
                    Some((magic, turns - 1))
                } else {
                    None
                };
            }

            let player_damage = calc_magic_damage(
                mctx.rng,
                mctx.progress.attack,
                self.enemy.defense,
                follow_up,
                self.enemy.weakness,
            );
            let mut run_after_enemy_turn = false;

            match self.action_menu(mctx, follow_up.is_some()).await {
                PlayerChoice::Fight => {
                    mctx.audio.play_sfx(Sfx::Attack);
                    self.enemy_hit_animation(mctx, player_damage).await;
                    self.enemy.hp -= player_damage.min(self.enemy.hp);

                    self.message_text.set_text(
                        mctx.gctx,
                        mctx.res,
                        &format!(
                            "Coric attacks!\n{player_damage} HP damage to {}.",
                            self.enemy.name,
                        ),
                    );
                    self.message_text.reveal().await;
                    self.wait_for_confirmation(mctx).await;
                }

                PlayerChoice::Magic(choice) => {
                    mctx.progress.mp -= mctx.progress.magic[choice].magic.mp_cost();
                    self.update_status(mctx);

                    match mctx.progress.magic[choice].magic {
                        Magic::Heal => {
                            mctx.audio.play_sfx(Sfx::Heal);

                            let heal_amount = (mctx.progress.max_hp + 1) / 2;
                            self.show_status_change(mctx, &format!("{heal_amount:+}"));
                            mctx.progress.hp =
                                mctx.progress.max_hp.min(mctx.progress.hp + heal_amount);
                            self.update_status(mctx);

                            self.message_text.set_text(
                                mctx.gctx,
                                mctx.res,
                                &format!("Coric casts Heal!\n{heal_amount} HP recovered."),
                            );
                        }

                        magic @ (Magic::EarthEdge | Magic::WaterEdge | Magic::FireEdge) => {
                            follow_up = Some((magic, 1));
                            let damage = calc_magic_damage(
                                mctx.rng,
                                mctx.progress.attack,
                                self.enemy.defense,
                                follow_up,
                                self.enemy.weakness,
                            );
                            mctx.audio.play_sfx(Sfx::Magic);
                            self.enemy_hit_animation(mctx, damage).await;
                            self.enemy.hp -= damage.min(self.enemy.hp);

                            self.message_text.set_text(
                                mctx.gctx,
                                mctx.res,
                                &format!(
                                    "Coric casts {}!\n{}{damage} HP damage to {}.",
                                    magic.name(),
                                    match (follow_up, self.enemy.weakness) {
                                        (Some((magic, _)), Some(weakness)) if magic == weakness => {
                                            format!(
                                                "{} is weak to {}!\n",
                                                self.enemy.name,
                                                weakness.name(),
                                            )
                                        }
                                        _ => String::new(),
                                    },
                                    self.enemy.name,
                                ),
                            );
                        }
                    }

                    self.message_text.reveal().await;
                    self.wait_for_confirmation(mctx).await;
                }

                PlayerChoice::Item(choice) => {
                    mctx.audio.play_sfx(Sfx::Heal);

                    mctx.progress.items[choice].amount -= 1;

                    let item = mctx.progress.items[choice].item;
                    let (heal_hp, heal_mp) = match item {
                        Item::Salve => ((mctx.progress.max_hp * 3 + 9) / 10, 0),
                        Item::XSalve => (mctx.progress.max_hp, 0),
                        Item::Tonic => (0, (mctx.progress.max_mp * 3 + 9) / 10),
                        Item::XTonic => (0, mctx.progress.max_mp),
                    };

                    if heal_hp > 0 {
                        self.show_status_change(mctx, &format!("{heal_hp:+}"));
                        mctx.progress.hp = mctx.progress.max_hp.min(mctx.progress.hp + heal_hp);
                        self.update_status(mctx);

                        self.message_text.set_text(
                            mctx.gctx,
                            mctx.res,
                            &format!(
                                "Coric uses {}.\n{heal_hp} HP healed for Coric!",
                                item.name()
                            ),
                        );
                        self.message_text.reveal().await;
                        self.wait_for_confirmation(mctx).await;
                    }

                    if heal_mp > 0 {
                        self.show_status_change(mctx, &format!("{heal_mp:+}MP"));
                        mctx.progress.mp = mctx.progress.max_mp.min(mctx.progress.mp + heal_mp);
                        self.update_status(mctx);

                        self.message_text.set_text(
                            mctx.gctx,
                            mctx.res,
                            &format!(
                                "Coric uses {}.\n{heal_mp} MP healed for Coric!",
                                item.name()
                            ),
                        );
                        self.message_text.reveal().await;
                        self.wait_for_confirmation(mctx).await;
                    }
                }

                PlayerChoice::Run => {
                    mctx.audio.play_sfx(Sfx::Confirm);

                    self.message_text
                        .set_text(mctx.gctx, mctx.res, "Coric turns to flee…");
                    self.message_text.reveal().await;
                    self.wait_for_confirmation(mctx).await;

                    let hp_run_threshold = self.enemy_max_hp - self.enemy.hp + player_damage;
                    if mctx.rng.random(self.enemy_max_hp as u32) < hp_run_threshold as u32 {
                        mctx.audio.set_music_volume_scripted(40);
                        self.message_text
                            .set_text(mctx.gctx, mctx.res, "Coric ran away!");
                        self.message_text.reveal().await;
                        self.wait_for_confirmation(mctx).await;
                        return BattleEvent::RanAway;
                    } else {
                        run_after_enemy_turn = true;
                    }
                }
            }

            self.update_status(mctx);

            if self.enemy.hp <= 0 {
                self.enemy_visible = false;

                mctx.audio.set_music_volume_scripted(40);

                self.message_text.set_text(
                    mctx.gctx,
                    mctx.res,
                    &format!(
                        "{} is defeated!\nCoric gained {} XP!",
                        self.enemy.name, self.enemy.exp,
                    ),
                );
                self.message_text.reveal().await;
                self.wait_for_confirmation(mctx).await;

                mctx.progress.exp += self.enemy.exp;
                while mctx.progress.gain_level_from_exp() {
                    self.update_status(mctx);

                    let mut msg = format!("Coric is now level {}!", mctx.progress.level);
                    let new_rank = player_rank(mctx.progress.level);
                    if new_rank != player_rank(mctx.progress.level - 1) {
                        msg.push_str("\nCoric becomes a ");
                        msg.push_str(new_rank);
                        msg.push('!');
                    }

                    mctx.audio.play_sfx(Sfx::LevelUp);
                    self.message_text.set_text(mctx.gctx, mctx.res, &msg);
                    self.message_text.reveal().await;
                    self.wait_for_confirmation(mctx).await;
                }

                // Chance for an item to drop.
                if mctx.rng.random(10) == 0 {
                    let item_roll = mctx.rng.random(18);

                    if item_roll < 9 {
                        let salves = mctx
                            .progress
                            .items
                            .iter_mut()
                            .find(|i| matches!(i.item, Item::Salve))
                            .expect("salve item slot");
                        if salves.amount < 9 {
                            salves.amount += 1;
                            mctx.audio.play_sfx(Sfx::Chime);
                            self.message_text
                                .set_text(mctx.gctx, mctx.res, "Coric found a Salve!");
                            self.message_text.reveal().await;
                            self.wait_for_confirmation(mctx).await;
                        }
                    } else if item_roll < 14 {
                        let tonics = mctx
                            .progress
                            .items
                            .iter_mut()
                            .find(|i| matches!(i.item, Item::Tonic))
                            .expect("tonic item slot");
                        if tonics.amount < 9 {
                            tonics.amount += 1;
                            mctx.audio.play_sfx(Sfx::Chime);
                            self.message_text
                                .set_text(mctx.gctx, mctx.res, "Coric found a Tonic!");
                            self.message_text.reveal().await;
                            self.wait_for_confirmation(mctx).await;
                        }
                    } else if item_roll < 17 {
                        let xsalves = mctx
                            .progress
                            .items
                            .iter_mut()
                            .find(|i| matches!(i.item, Item::XSalve))
                            .expect("xsalve item slot");
                        if xsalves.amount < 9 {
                            xsalves.amount += 1;
                            mctx.audio.play_sfx(Sfx::Chime);
                            self.message_text.set_text(
                                mctx.gctx,
                                mctx.res,
                                "Coric found an XSalve!",
                            );
                            self.message_text.reveal().await;
                            self.wait_for_confirmation(mctx).await;
                        }
                    } else {
                        let xtonics = mctx
                            .progress
                            .items
                            .iter_mut()
                            .find(|i| matches!(i.item, Item::XTonic))
                            .expect("xtonic item slot");
                        if xtonics.amount < 9 {
                            xtonics.amount += 1;
                            mctx.audio.play_sfx(Sfx::Chime);
                            self.message_text.set_text(
                                mctx.gctx,
                                mctx.res,
                                "Coric found an XTonic!",
                            );
                            self.message_text.reveal().await;
                            self.wait_for_confirmation(mctx).await;
                        }
                    }
                }

                return BattleEvent::Victory;
            }

            let mut enemy_action = EnemyAction {
                chance: 100,
                msg: "attacks!",
                damage_factor: Some(1.0),
            };

            let mut enemy_roll = mctx.rng.random(100);
            for action in self.enemy.actions {
                if enemy_roll < action.chance {
                    enemy_action = action.clone();
                    break;
                } else {
                    enemy_roll -= action.chance;
                }
            }

            let mut enemy_msg = format!("{} {}", self.enemy.name, enemy_action.msg);

            if let Some(damage_factor) = enemy_action.damage_factor {
                let base_damage =
                    calc_base_damage(self.enemy.attack, mctx.progress.defense) * damage_factor;
                let damage = base_damage.trunc() as i32
                    + if (mctx.rng.random(100) as f32) < base_damage.fract() * 100.0 {
                        1
                    } else {
                        0
                    };

                mctx.audio.play_sfx(Sfx::Hurt);
                for _ in 0..5 {
                    self.status_visible = false;
                    wait_once().await;
                    wait_once().await;
                    self.status_visible = true;
                    wait_once().await;
                    wait_once().await;
                }

                self.show_status_change(mctx, &format!("{damage}"));
                mctx.progress.hp -= damage.min(mctx.progress.hp);
                self.update_status(mctx);

                enemy_msg.push_str(&format!("\n{damage} HP damage to Coric."));
            }

            self.message_text.set_text(mctx.gctx, mctx.res, &enemy_msg);
            self.message_text.reveal().await;
            self.wait_for_confirmation(mctx).await;

            if mctx.progress.hp <= 0 {
                mctx.audio.set_music_volume_scripted(40);
                return BattleEvent::Defeat;
            } else if run_after_enemy_turn {
                mctx.audio.set_music_volume_scripted(40);
                self.message_text
                    .set_text(mctx.gctx, mctx.res, "Coric ran away!");
                self.message_text.reveal().await;
                self.wait_for_confirmation(mctx).await;
                return BattleEvent::RanAway;
            }
        }
    }

    fn update_status(&mut self, mctx: &mut ModeContext) {
        self.hp_text
            .set_text(mctx.gctx, mctx.res, &format!("HP{:4}", mctx.progress.hp));
        self.mp_text
            .set_text(mctx.gctx, mctx.res, &format!("MP{:4}", mctx.progress.mp));
        self.hp_meter
            .set_value_and_max(mctx.gctx, mctx.progress.hp, mctx.progress.max_hp);
        self.mp_meter
            .set_value_and_max(mctx.gctx, mctx.progress.mp, mctx.progress.max_mp);
    }

    async fn wait_for_confirmation(&mut self, mctx: &mut ModeContext<'_, '_>) {
        while !mctx.input.is_key_pressed(GameKey::Confirm) {
            self.enemy_sprite.animate();
            wait_once().await;
        }
        self.change_visible = false;
        mctx.audio.play_sfx(Sfx::Cursor);
    }
}

fn calc_base_damage(attack: i32, defense: i32) -> f32 {
    let attack = attack as f32;
    let defense = defense as f32;
    if attack * attack < defense {
        1.0
    } else if attack < defense {
        attack * attack / defense
    } else {
        attack * 2.0 - defense
    }
}

fn calc_magic_damage(
    rng: &mut Rng,
    attack: i32,
    defense: i32,
    follow_up: Option<(Magic, usize)>,
    weakness: Option<Magic>,
) -> i32 {
    let base_damage = calc_base_damage(attack, defense);
    let bonus: f32 = match follow_up {
        Some((magic, turns)) => {
            let weak = weakness.map(|m| m == magic).unwrap_or(false);
            let which = if weak { turns + 1 } else { turns };
            [0.5, 1.0, 2.0][which.min(2)]
        }
        None => 0.0,
    };
    let damage = base_damage * (1.0 + bonus);
    damage.trunc() as i32
        + if (rng.random(100) as f32) < damage.fract() * 100.0 {
            1
        } else {
            0
        }
}
