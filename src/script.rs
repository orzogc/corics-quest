use crate::actor::*;
use crate::audio::*;
use crate::contexts::*;
use crate::enemy::*;
use crate::levels::TILE_SIZE;
use crate::modes::*;
use crate::progress::*;
use crate::wait_once;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

type ScriptCallback = for<'a> fn(&'a mut ScriptContext) -> Pin<Box<dyn Future<Output = ()> + 'a>>;

struct LevelScripts {
    level_name: &'static str,
    on_enter: Option<ScriptCallback>,
    on_talk: &'static [(ActorType, ScriptCallback)],
}

static LEVEL_SCRIPTS: &[LevelScripts] = &[
    LevelScripts {
        level_name: "Start",
        on_enter: None,
        on_talk: &[
            (ActorType::Bed, |sctx| {
                Box::pin(async {
                    if sctx.progress.hp < sctx.progress.max_hp
                        || sctx.progress.mp < sctx.progress.max_mp
                    {
                        sctx.audio.set_music_volume_scripted(40);
                        sctx.fade.out_to_black(60).await;
                        sctx.audio.play_sfx(Sfx::Heal);
                        for _ in 0..30 {
                            wait_once().await;
                        }
                        sctx.fade.in_from_black(60).await;
                        sctx.audio.set_music_volume_scripted(100);
                        sctx.progress.hp = sctx.progress.max_hp;
                        sctx.progress.mp = sctx.progress.max_mp;

                        sctx.push_text_box_mode("HP and MP recovered!");
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }

                    sctx.push_yes_no_prompt_mode("Save your progress?", "Save", "Don't save", true);
                    let mut do_save = matches!(
                        sctx.update_yes_no_prompt_mode().await,
                        YesNoPromptEvent::Yes
                    );
                    sctx.pop_mode();

                    if do_save && sctx.confirm_save_overwrite {
                        sctx.push_yes_no_prompt_mode(
                            "Save data exists; overwrite it?",
                            "Overwrite",
                            "Cancel",
                            false,
                        );
                        do_save = matches!(
                            sctx.update_yes_no_prompt_mode().await,
                            YesNoPromptEvent::Yes
                        );
                        sctx.pop_mode();
                    }

                    if do_save {
                        match sctx.progress.save() {
                            Ok(()) => {
                                sctx.push_text_box_mode("Progress has been saved.");
                                sctx.confirm_save_overwrite = false;
                            }
                            Err(e) => sctx.push_text_box_mode(&format!("Save error:\n{e}")),
                        }
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                })
            }),
            (ActorType::Ducille, |sctx| {
                Box::pin(async {
                    let (salves, xsalves, tonics) = if !sctx.progress.earth_defeated {
                        (1, 0, 1)
                    } else if !sctx.progress.water_defeated {
                        (2, 0, 2)
                    } else {
                        (1, 1, 2)
                    };
                    let salves_given = sctx.progress.maybe_give_items(Item::Salve, salves);
                    let xsalves_given = sctx.progress.maybe_give_items(Item::XSalve, xsalves);
                    let tonics_given = sctx.progress.maybe_give_items(Item::Tonic, tonics);
                    if salves_given + xsalves_given + tonics_given > 0 {
                        sctx.push_text_box_mode(
                            "Ducille:\n\
                             You need items, Coric?\n\
                             Let's see what I can find…",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    if salves_given > 0 {
                        sctx.audio.play_sfx(Sfx::Chime);
                        if salves_given == 1 {
                            sctx.push_text_box_mode("Coric got a Salve!");
                        } else {
                            sctx.push_text_box_mode(&format!("Coric got {salves_given} Salves!"));
                        }
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    if xsalves_given > 0 {
                        sctx.audio.play_sfx(Sfx::Chime);
                        if xsalves_given == 1 {
                            sctx.push_text_box_mode("Coric got an XSalve!");
                        } else {
                            sctx.push_text_box_mode(&format!("Coric got {xsalves_given} XSalves!"));
                        }
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    if tonics_given > 0 {
                        sctx.audio.play_sfx(Sfx::Chime);
                        if tonics_given == 1 {
                            sctx.push_text_box_mode("Coric got a Tonic!");
                        } else {
                            sctx.push_text_box_mode(&format!("Coric got {tonics_given} Tonics!"));
                        }
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }

                    if !sctx.progress.earth_defeated {
                        sctx.push_text_box_mode(
                            "Ducille:\n\
                             You can rest in your bed\n\
                             to recover and save.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.water_defeated {
                        sctx.push_text_box_mode(
                            "Ducille:\n\
                             If you fall in battle,\n\
                             we'll bring you back home.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.fire_defeated {
                        sctx.push_text_box_mode(
                            "Ducille:\n\
                             The spirits were possessed, you say?\n\
                             I wonder how that came to be…",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                })
            }),
            (ActorType::Jace, |sctx| {
                Box::pin(async {
                    if !sctx
                        .progress
                        .magic
                        .iter()
                        .find(|m| m.magic == Magic::Heal)
                        .expect("Heal magic slot")
                        .known
                    {
                        sctx.push_text_box_mode(
                            "Jace:\n\
                             If you're heading out, I can teach\n\
                             you some magic to keep you safe.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();

                        learn_magic(sctx, Magic::Heal).await;

                        sctx.push_text_box_mode(
                            "Jace:\n\
                             Don't be shy with it.  I've never\n\
                             seen the wildlife like this before.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }

                    if !sctx.progress.earth_defeated {
                        sctx.push_text_box_mode(
                            "Jace:\n\
                             The spirits reside in three castles.\n\
                             The Earth Castle lies to the east.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.water_defeated {
                        sctx.push_text_box_mode(
                            "Jace:\n\
                             Head to the Water Castle, across\n\
                             the lakes and forests to the west.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.fire_defeated {
                        sctx.push_text_box_mode(
                            "Jace:\n\
                             Across the chasms and cliffs\n\
                             to the north lies the Fire Castle.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                })
            }),
            (ActorType::Julis, |sctx| {
                Box::pin(async {
                    if !sctx.progress.earth_defeated {
                        sctx.push_text_box_mode(
                            "Julis:\n\
                             Press Left Ctrl to view your status,\n\
                             and use items and magic.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();

                        sctx.push_text_box_mode(
                            "Julis:\n\
                             Talk to us when you make progress;\n\
                             we'll have more to tell you.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.water_defeated {
                        sctx.push_text_box_mode(
                            "Julis:\n\
                             Ducille tends to the apocathery;\n\
                             Jace knows the lay of the land.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.fire_defeated {
                        sctx.push_text_box_mode(
                            "Julis:\n\
                             We have records of vampires\n\
                             bested by water.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                })
            }),
            (ActorType::Matero, |sctx| {
                Box::pin(async {
                    let attack_boost = sctx.progress.maybe_upgrade_weapon("Short Sword", 2);
                    let defense_boost = sctx.progress.maybe_upgrade_armor("Leather Armor", 2);
                    if attack_boost.is_some() || defense_boost.is_some() {
                        sctx.push_text_box_mode(
                            "Matero:\n\
                             Going on a quest?\n\
                             I have some gear you can use.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    if let Some(attack_boost) = attack_boost {
                        sctx.audio.play_sfx(Sfx::Chime);
                        sctx.push_text_box_mode(&format!(
                            "Coric got a Short Sword!\n\
                             Attack increased by {attack_boost}!",
                        ));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    if let Some(defense_boost) = defense_boost {
                        sctx.audio.play_sfx(Sfx::Chime);
                        sctx.push_text_box_mode(&format!(
                            "Coric got a Leather Armor!\n\
                             Defense increased by {defense_boost}!",
                        ));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }

                    if !sctx.progress.earth_defeated {
                        sctx.push_text_box_mode(
                            "Matero:\n\
                             If you use magic on a foe, you can\n\
                             follow up next turn for more damage.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.water_defeated {
                        sctx.push_text_box_mode(
                            "Matero:\n\
                             Rumor has it that each castle has\n\
                             a weapon and an armor to find.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    } else if !sctx.progress.fire_defeated {
                        sctx.push_text_box_mode(
                            "Matero:\n\
                             If spikes block your path, you can\n\
                             pull a lever to retract them.",
                        );
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                })
            }),
        ],
    },
    LevelScripts {
        level_name: "Level_10",
        on_enter: Some(|sctx| {
            Box::pin(async {
                if !sctx.progress.earth_defeated {
                    sctx.place_gates(9, 10);
                }
            })
        }),
        on_talk: &[],
    },
    LevelScripts {
        level_name: "Earth_4",
        on_enter: Some(|sctx| {
            Box::pin(async {
                if sctx.progress.earth_defeated {
                    let earth = sctx
                        .actors
                        .iter()
                        .position(|a| a.identifier == ActorType::Earth)
                        .expect("Earth actor");
                    sctx.actors.remove(earth);
                }
            })
        }),
        on_talk: &[(ActorType::Earth, |sctx| {
            Box::pin(async {
                sctx.push_text_box_mode(
                    "Earth:\nI will return you to the dust\nwhence you came, mortal!",
                );
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.push_battle_mode(Enemy::earth_spirit(), true);

                let earth = sctx
                    .actors
                    .iter()
                    .position(|a| a.identifier == ActorType::Earth)
                    .expect("Earth actor");
                sctx.actors[earth].visible = false;

                if !handle_battle(sctx).await {
                    return;
                }

                sctx.actors[earth].visible = true;

                sctx.push_text_box_mode("Earth:\nI was… possessed… please…\nsave… the others…");
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.actors.remove(earth);
                sctx.progress.earth_defeated = true;
            })
        })],
    },
    LevelScripts {
        level_name: "Level_17",
        on_enter: Some(|sctx| {
            Box::pin(async {
                if !sctx.progress.water_defeated {
                    sctx.place_gates(29, 12);
                }
            })
        }),
        on_talk: &[],
    },
    LevelScripts {
        level_name: "Water_4",
        on_enter: Some(|sctx| {
            Box::pin(async {
                if sctx.progress.water_defeated {
                    let water = sctx
                        .actors
                        .iter()
                        .position(|a| a.identifier == ActorType::Water)
                        .expect("Water actor");
                    sctx.actors.remove(water);
                }
            })
        }),
        on_talk: &[(ActorType::Water, |sctx| {
            Box::pin(async {
                sctx.push_text_box_mode("Water:\nThe Water of Life claims all souls!");
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.push_battle_mode(Enemy::water_spirit(), true);

                let water = sctx
                    .actors
                    .iter()
                    .position(|a| a.identifier == ActorType::Water)
                    .expect("Water actor");
                sctx.actors[water].visible = false;

                if !handle_battle(sctx).await {
                    return;
                }

                sctx.actors[water].visible = true;

                sctx.push_text_box_mode("Water:\nI was… possessed… please…\nsave… the others…");
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.actors.remove(water);
                sctx.progress.water_defeated = true;
            })
        })],
    },
    LevelScripts {
        level_name: "Fire_4",
        on_enter: Some(|sctx| {
            Box::pin(async {
                if sctx.progress.fire_defeated {
                    let fire = sctx
                        .actors
                        .iter()
                        .position(|a| a.identifier == ActorType::Fire)
                        .expect("Fire actor");
                    sctx.actors.remove(fire);
                }
            })
        }),
        on_talk: &[(ActorType::Fire, |sctx| {
            Box::pin(async {
                sctx.push_text_box_mode("Fire:\nYour flame of life will be\nextinguished here!");
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.push_battle_mode(Enemy::fire_spirit(), true);

                let fire = sctx
                    .actors
                    .iter()
                    .position(|a| a.identifier == ActorType::Fire)
                    .expect("Fire actor");
                sctx.actors[fire].visible = false;

                if !handle_battle(sctx).await {
                    return;
                }

                sctx.actors[fire].visible = true;

                sctx.push_text_box_mode("Fire:\nI was… possessed…\nThank you… Coric…");
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.actors.remove(fire);
                sctx.progress.fire_defeated = true;
            })
        })],
    },
];

pub async fn script_main(mut sctx: ScriptContext) {
    validate_level_scripts(&mut sctx);

    let mut play_intro_sequence = false;

    sctx.push_title_mode();
    loop {
        match sctx.update_title_mode().await {
            TitleEvent::NewGame(false) => {
                // Start a new game, but skip the intro.
                sctx.confirm_save_overwrite = save_data_exists();
                sctx.fade.out_to_black(30).await;
                sctx.pop_mode(); // Title
                break;
            }
            TitleEvent::NewGame(true) => {
                sctx.confirm_save_overwrite = save_data_exists();
                sctx.fade.out_to_black(30).await;
                sctx.pop_mode(); // Title

                sctx.push_intro_mode();
                let IntroEvent::Done = sctx.update_intro_mode().await;
                sctx.fade.in_from_black(120).await;

                sctx.push_text_box_mode(
                    "Air:\n\
                     Coric… listen well.\n\
                     I am the Spirit of Air.",
                );
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.push_text_box_mode(
                    "Air:\n\
                     A foul curse has befallen my kin.\n\
                     Even now, our power wanes…",
                );
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                sctx.pop_mode();

                sctx.push_text_box_mode(
                    "Air:\n\
                     Please… you must save us…\n\
                     Before all is lost…",
                );
                let TextBoxEvent::Done = sctx.update_text_box_mode().await;

                sctx.audio.set_music_volume_scripted(40);
                sctx.fade.to_color(120, [1.0, 1.0, 1.0, 1.0]).await;
                sctx.fade.to_color(120, [0.0, 0.0, 0.0, 1.0]).await;
                sctx.audio.play_music(None).await;
                sctx.audio.set_music_volume_scripted(100);

                sctx.pop_mode(); // TextBox
                sctx.pop_mode(); // Intro

                play_intro_sequence = true;

                break;
            }
            TitleEvent::Continue => {
                match Progress::load() {
                    Ok(progress) => {
                        sctx.progress = progress;
                        sctx.confirm_save_overwrite = false;
                        sctx.fade.out_to_black(30).await;
                        sctx.pop_mode(); // Title
                        break;
                    }
                    Err(e) => {
                        sctx.push_text_box_mode(&format!("Load error:\n{e}"));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode(); // TextBox
                    }
                }
            }
            TitleEvent::Options => {
                sctx.push_options_mode(82, 101, true);
                if !handle_options(&mut sctx).await {
                    return;
                }
            }
        }
    }

    sctx.push_walk_around_mode();

    if play_intro_sequence {
        sctx.fade.in_from_black(120).await;

        sctx.actors[0].start_animation("face_e");
        for _ in 0..30 {
            wait_once().await;
        }
        sctx.actors[0].start_animation("face_w");
        for _ in 0..30 {
            wait_once().await;
        }
        sctx.actors[0].start_animation("face_e");
        for _ in 0..30 {
            wait_once().await;
        }
        sctx.actors[0].start_animation("face_s");
        for _ in 0..30 {
            wait_once().await;
        }

        sctx.push_text_box_mode(
            "Coric:\n\
             That vision…\n\
             I'd better speak with the others.",
        );
        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
        sctx.pop_mode();
    } else {
        sctx.fade.in_from_black(30).await;
    }

    sctx.audio.play_music(sctx.level.music).await;

    loop {
        match sctx.update_walk_around_mode().await {
            WalkAroundEvent::DebugMenu => {
                sctx.audio.play_sfx(Sfx::Confirm);
                sctx.push_debug_menu_mode();
                let debug_menu_event = sctx.update_debug_menu_mode().await;
                sctx.pop_mode();
                match debug_menu_event {
                    DebugMenuEvent::Cancel => {}
                    DebugMenuEvent::GainLevel => {
                        sctx.progress.exp = 0;
                        sctx.progress.gain_level();
                        sctx.audio.play_sfx(Sfx::LevelUp);
                        sctx.push_text_box_mode(&format!(
                            "Coric is now level {}!",
                            sctx.progress.level,
                        ));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    DebugMenuEvent::Battle(battle) => {
                        let (enemy, boss_fight) = if battle < 6 {
                            let group = match battle {
                                0 => EncounterGroup::Wilderness1,
                                1 => EncounterGroup::Wilderness2,
                                2 => EncounterGroup::Wilderness3,
                                3 => EncounterGroup::EarthCastle,
                                4 => EncounterGroup::WaterCastle,
                                5 => EncounterGroup::FireCastle,
                                _ => unreachable!(),
                            };
                            (group.random_enemy(&mut sctx.rng), false)
                        } else {
                            (
                                match battle {
                                    6 => Enemy::earth_spirit(),
                                    7 => Enemy::water_spirit(),
                                    8 => Enemy::fire_spirit(),
                                    b => panic!("invalid battle number: {b}"),
                                },
                                true,
                            )
                        };
                        sctx.push_battle_mode(enemy, boss_fight);
                        handle_battle(&mut sctx).await;
                        sctx.audio.play_music(sctx.level.music).await;
                    }
                    DebugMenuEvent::SetWeapon(weapon) => {
                        sctx.progress.attack += weapon.as_ref().map(|w| w.attack).unwrap_or(0)
                            - sctx.progress.weapon.as_ref().map(|w| w.attack).unwrap_or(0);
                        sctx.progress.weapon = weapon;

                        sctx.push_text_box_mode(&format!(
                            "Coric's weapon is now {}.",
                            sctx.progress
                                .weapon
                                .as_ref()
                                .map(|w| w.name.as_str())
                                .unwrap_or("(none)"),
                        ));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    DebugMenuEvent::SetArmor(armor) => {
                        sctx.progress.defense += armor.as_ref().map(|a| a.defense).unwrap_or(0)
                            - sctx.progress.armor.as_ref().map(|a| a.defense).unwrap_or(0);
                        sctx.progress.armor = armor;

                        sctx.push_text_box_mode(&format!(
                            "Coric's armor is now {}.",
                            sctx.progress
                                .armor
                                .as_ref()
                                .map(|a| a.name.as_str())
                                .unwrap_or("(none)"),
                        ));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    DebugMenuEvent::GetItems => {
                        sctx.progress.maybe_give_items(Item::Salve, 2);
                        sctx.progress.maybe_give_items(Item::XSalve, 2);
                        sctx.progress.maybe_give_items(Item::Tonic, 2);
                        sctx.progress.maybe_give_items(Item::XTonic, 2);
                    }
                    DebugMenuEvent::LearnAllMagic => {
                        learn_magic(&mut sctx, Magic::Heal).await;
                        learn_magic(&mut sctx, Magic::FireEdge).await;
                        learn_magic(&mut sctx, Magic::EarthEdge).await;
                        learn_magic(&mut sctx, Magic::WaterEdge).await;
                    }
                    DebugMenuEvent::ResetStepCounts => {
                        sctx.progress.steps.fill(0);
                        sctx.push_text_box_mode("Step counts reset.");
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    DebugMenuEvent::ChangeFlag(flag) => {
                        let (name, new_value) = match flag {
                            0 => {
                                sctx.progress.earth_defeated ^= true;
                                ("earth_defeated", sctx.progress.earth_defeated)
                            }
                            1 => {
                                sctx.progress.water_defeated ^= true;
                                ("water_defeated", sctx.progress.water_defeated)
                            }
                            2 => {
                                sctx.progress.fire_defeated ^= true;
                                ("fire_defeated", sctx.progress.fire_defeated)
                            }
                            f => panic!("unknown flag number: {f}"),
                        };
                        sctx.push_text_box_mode(&format!("{name} is now {new_value}."));
                        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                        sctx.pop_mode();
                    }
                    DebugMenuEvent::Warp { level_id, x, y } => {
                        warp_to_level(&mut sctx, level_id, x, y).await;
                    }
                    DebugMenuEvent::Quit => return,
                }
            }
            WalkAroundEvent::Encounter => {
                if let Some(enemy) = sctx.level.encounters.map(|g| g.random_enemy(&mut sctx.rng)) {
                    sctx.push_battle_mode(enemy, false);
                    handle_battle(&mut sctx).await;
                    sctx.audio.play_music(sctx.level.music).await;
                }
                sctx.encounter_steps = 15 + sctx.rng.random(26) as i32;
            }
            WalkAroundEvent::MainMenu => {
                sctx.audio.play_sfx(Sfx::Confirm);
                sctx.push_main_menu_mode(false);
                loop {
                    match sctx.update_main_menu_mode().await {
                        MainMenuEvent::Done => {
                            sctx.pop_mode(); // MainMenu
                            break;
                        }
                        MainMenuEvent::Options => {
                            sctx.push_options_mode(52, 78, false);
                            if !handle_options(&mut sctx).await {
                                return;
                            }
                        }
                    }
                }
            }
            WalkAroundEvent::TalkActor(actor) => {
                if let Some((_, talk_script)) = LEVEL_SCRIPTS
                    .iter()
                    .find(|l| l.level_name == sctx.level.identifier)
                    .and_then(|l| {
                        l.on_talk
                            .iter()
                            .find(|(ty, _)| *ty == sctx.actors[actor].identifier)
                    })
                {
                    (talk_script)(&mut sctx).await;

                    // Trigger the ending when defeating the final boss.
                    if sctx.progress.fire_defeated {
                        sctx.fade.to_color(180, [1.0, 1.0, 1.0, 1.0]).await;
                        sctx.fade.to_color(180, [0.0, 0.0, 0.0, 1.0]).await;
                        sctx.fade.set([0.0; 4]);

                        sctx.pop_mode(); // WalkAround
                        sctx.push_ending_mode();
                        loop {
                            match sctx.update_ending_mode().await {
                                EndingEvent::Credits => {
                                    sctx.push_credits_mode();
                                    let CreditsEvent::Done = sctx.update_credits_mode().await;
                                    sctx.pop_mode(); // Credits
                                }
                                EndingEvent::Status => {
                                    sctx.push_main_menu_mode(true);
                                    match sctx.update_main_menu_mode().await {
                                        MainMenuEvent::Done => sctx.pop_mode(), // MainMenu
                                        _ => unreachable!(),
                                    }
                                }
                            }
                        }
                    } else {
                        sctx.audio.play_music(sctx.level.music).await;
                    }
                } else if sctx.actors[actor].identifier == ActorType::Chest {
                    if !sctx
                        .progress
                        .collected_chests
                        .iter()
                        .map(String::as_str)
                        .any(|s| s == sctx.level.identifier.as_str())
                    {
                        let chest_opened =
                            match sctx.actors[actor].chest_type.expect("ChestType for Chest") {
                                ChestType::FireEdge => {
                                    sctx.actors[actor].start_animation("open");
                                    learn_magic(&mut sctx, Magic::FireEdge).await;
                                    true
                                }

                                ChestType::EarthEdge => {
                                    sctx.actors[actor].start_animation("open");
                                    learn_magic(&mut sctx, Magic::EarthEdge).await;
                                    true
                                }

                                ChestType::WaterEdge => {
                                    sctx.actors[actor].start_animation("open");
                                    learn_magic(&mut sctx, Magic::WaterEdge).await;
                                    true
                                }

                                ChestType::LongSword => {
                                    chest_with_weapon(&mut sctx, actor, "Long Sword", 7).await
                                }

                                ChestType::ChainVest => {
                                    chest_with_armor(&mut sctx, actor, "Chain Vest", 7).await
                                }

                                ChestType::DuelistSword => {
                                    chest_with_weapon(&mut sctx, actor, "Duelist Sword", 13).await
                                }

                                ChestType::SteelArmor => {
                                    chest_with_armor(&mut sctx, actor, "Steel Armor", 13).await
                                }

                                ChestType::ValorBlade => {
                                    chest_with_weapon(&mut sctx, actor, "Valor Blade", 25).await
                                }

                                ChestType::MythicPlate => {
                                    chest_with_armor(&mut sctx, actor, "Mythic Plate", 25).await
                                }
                            };

                        if chest_opened {
                            sctx.progress
                                .collected_chests
                                .push(sctx.level.identifier.clone());
                        }
                    }
                } else if sctx.actors[actor].identifier == ActorType::Lever {
                    if sctx.lever_is_turned() {
                        sctx.push_text_box_mode("Coric turns the lever to the left.");
                    } else {
                        sctx.push_text_box_mode("Coric turns the lever to the right.");
                    }
                    let TextBoxEvent::Done = sctx.update_text_box_mode().await;
                    sctx.pop_mode();

                    sctx.toggle_lever();
                } else {
                    panic!(
                        "missing on_talk script for {:?} in level {}",
                        sctx.actors[actor].identifier, sctx.level.identifier,
                    );
                }
            }
            WalkAroundEvent::TouchLevelEdge(dir) => {
                if let Some((level, mut actors)) = sctx.level_by_neighbour(dir) {
                    // walk out of old level
                    walk_player(&mut sctx.actors[..], dir, |i, max| {
                        sctx.fade.set([0.0, 0.0, 0.0, i as f32 / max as f32])
                    })
                    .await;

                    // move player to the new level
                    sctx.actors.truncate(1);
                    let mut player = sctx.actors.pop().expect("player actor");
                    player.grid_x +=
                        (sctx.level.px_world_x - level.px_world_x) / TILE_SIZE - dir.dx();
                    player.grid_y +=
                        (sctx.level.px_world_y - level.px_world_y) / TILE_SIZE - dir.dy();
                    actors.insert(0, player);
                    *sctx.actors = actors;
                    *sctx.level = level;

                    run_level_on_enter(&mut sctx).await;

                    // walk into the new level
                    walk_player(&mut sctx.actors[..], dir, |i, max| {
                        let alpha = 1.0 - i as f32 / max as f32;
                        sctx.fade.set([0.0, 0.0, 0.0, alpha]);
                    })
                    .await;
                } else {
                    sctx.actors[0].stop_walk_animation();
                }
            }
        }
    }
}

fn validate_level_scripts(sctx: &mut ScriptContext) {
    let mut level_identifiers: HashSet<String> = HashSet::new();
    for l in LEVEL_SCRIPTS {
        if !sctx.res.levels.contains_identifier(l.level_name) {
            panic!("LEVEL_SCRIPTS: unknown level identifier: {}", l.level_name);
        }

        if !level_identifiers.contains(l.level_name) {
            level_identifiers.insert(l.level_name.to_string());
        } else {
            panic!(
                "LEVEL_SCRIPTS: duplicate level identifier: {}",
                l.level_name
            );
        }

        let mut actor_types: HashSet<ActorType> = HashSet::new();
        for t in l.on_talk {
            if !actor_types.contains(&t.0) {
                actor_types.insert(t.0);
            } else {
                panic!("on_talk: duplicate ActorType: {:?}", t.0);
            }
        }
    }
}

async fn run_level_on_enter(sctx: &mut ScriptContext) {
    if let Some(on_enter) = LEVEL_SCRIPTS
        .iter()
        .find(|l| l.level_name == sctx.level.identifier)
        .and_then(|l| l.on_enter)
    {
        (on_enter)(sctx).await;
    }
    sctx.audio.play_music(sctx.level.music).await;
}

async fn warp_to_level(sctx: &mut ScriptContext, level_id: &'static str, x: i32, y: i32) {
    let (level, mut actors) = sctx.level_by_identifier(level_id);
    sctx.actors.truncate(1);
    let mut player = sctx.actors.pop().expect("player actor");
    player.grid_x = x;
    player.grid_y = y;
    player.start_animation("face_s");
    actors.insert(0, player);
    *sctx.level = level;
    *sctx.actors = actors;

    run_level_on_enter(sctx).await;
}

async fn handle_battle(sctx: &mut ScriptContext) -> bool {
    sctx.actors[0].visible = false;
    let event = sctx.update_battle_mode().await;
    sctx.pop_mode();
    sctx.actors[0].visible = true;
    sctx.audio.play_music(None).await;
    sctx.audio.set_music_volume_scripted(100);
    match event {
        BattleEvent::Victory => true,
        BattleEvent::RanAway => false,
        BattleEvent::Defeat => {
            sctx.fade.out_to_black(90).await;

            // warp player back to town
            warp_to_level(sctx, "Start", 6, 3).await;

            sctx.progress.hp = sctx.progress.max_hp;
            sctx.progress.mp = sctx.progress.max_mp;
            sctx.fade.in_from_black(90).await;

            sctx.push_text_box_mode("Coric:\nOuch!");
            let TextBoxEvent::Done = sctx.update_text_box_mode().await;
            sctx.pop_mode();

            false
        }
    }
}

async fn handle_options(sctx: &mut ScriptContext) -> bool {
    loop {
        match sctx.update_options_mode().await {
            OptionsEvent::Credits => {
                sctx.audio.play_sfx(Sfx::Confirm);
                sctx.audio.set_music_volume_scripted(40);
                sctx.fade.out_to_black(60).await;

                sctx.push_credits_mode();
                let CreditsEvent::Done = sctx.update_credits_mode().await;
                sctx.pop_mode(); // Credits

                sctx.fade.in_from_black(60).await;
                sctx.audio.set_music_volume_scripted(100);
            }
            OptionsEvent::Done => {
                sctx.pop_mode(); // Options
                return true;
            }
            OptionsEvent::Quit => {
                sctx.audio.play_sfx(Sfx::Confirm);

                sctx.push_yes_no_prompt_mode("Really quit?", "Yes", "No", false);
                match sctx.update_yes_no_prompt_mode().await {
                    YesNoPromptEvent::Yes => return false,
                    YesNoPromptEvent::No => {
                        sctx.pop_mode(); // YesNoPrompt
                    }
                }
            }
        }
    }
}

async fn chest_with_armor(
    sctx: &mut ScriptContext,
    chest: usize,
    name: &'static str,
    defense: i32,
) -> bool {
    sctx.actors[chest].start_animation("open");
    if let Some(defense_boost) = sctx.progress.maybe_upgrade_armor(name, defense) {
        sctx.audio.play_sfx(Sfx::Chime);
        sctx.push_text_box_mode(&format!(
            "Coric found a {name}!\n\
             Defense increased by {defense_boost}!",
        ));
        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
        sctx.pop_mode();
        true
    } else {
        sctx.push_text_box_mode(&format!(
            "Coric found a {name},\n\
             but doesn't need it.",
        ));
        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
        sctx.pop_mode();
        sctx.actors[chest].start_animation("closed");
        false
    }
}

async fn chest_with_weapon(
    sctx: &mut ScriptContext,
    chest: usize,
    name: &'static str,
    attack: i32,
) -> bool {
    sctx.actors[chest].start_animation("open");
    if let Some(attack_boost) = sctx.progress.maybe_upgrade_weapon(name, attack) {
        sctx.audio.play_sfx(Sfx::Chime);
        sctx.push_text_box_mode(&format!(
            "Coric found a {name}!\n\
             Attack increased by {attack_boost}!",
        ));
        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
        sctx.pop_mode();
        true
    } else {
        sctx.push_text_box_mode(&format!(
            "Coric found a {name},\n\
             but doesn't need it.",
        ));
        let TextBoxEvent::Done = sctx.update_text_box_mode().await;
        sctx.pop_mode();
        sctx.actors[chest].start_animation("closed");
        false
    }
}

async fn learn_magic(sctx: &mut ScriptContext, magic: Magic) {
    let magic_slot = sctx
        .progress
        .magic
        .iter_mut()
        .find(|m| m.magic == magic)
        .expect("magic slot");
    magic_slot.known = true;

    sctx.audio.play_sfx(Sfx::Chime);
    sctx.push_text_box_mode(&format!("Coric learned {}!", magic.name()));
    let TextBoxEvent::Done = sctx.update_text_box_mode().await;
    sctx.pop_mode();
}
