use miniquad::KeyCode;

#[derive(Clone, Copy)]
pub enum GameKey {
    DebugMenu,
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Cancel,
}

pub struct Input {
    keys_down: Vec<bool>,
    keys_pressed: Vec<bool>,
}

impl GameKey {
    const NUM_KEYS: usize = Self::Cancel as usize + 1;
}

impl TryFrom<KeyCode> for GameKey {
    type Error = ();

    fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
        Ok(match value {
            KeyCode::Backslash => Self::DebugMenu,
            KeyCode::Up => Self::Up,
            KeyCode::Down => Self::Down,
            KeyCode::Left => Self::Left,
            KeyCode::Right => Self::Right,
            KeyCode::Space => Self::Confirm,
            KeyCode::LeftControl => Self::Cancel,
            _ => return Err(()),
        })
    }
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys_down: vec![false; GameKey::NUM_KEYS],
            keys_pressed: vec![false; GameKey::NUM_KEYS],
        }
    }

    pub fn handle_key_down_event(&mut self, keycode: KeyCode) {
        if let Ok(game_key) = GameKey::try_from(keycode) {
            if cfg!(debug_assertions) || !matches!(game_key, GameKey::DebugMenu) {
                self.keys_down[game_key as usize] = true;
                self.keys_pressed[game_key as usize] = true;
            }
        }
    }

    pub fn handle_key_up_event(&mut self, keycode: KeyCode) {
        if let Ok(game_key) = GameKey::try_from(keycode) {
            self.keys_down[game_key as usize] = false;
        }
    }

    pub fn is_key_down(&self, game_key: GameKey) -> bool {
        self.keys_down[game_key as usize]
    }

    pub fn is_key_pressed(&self, game_key: GameKey) -> bool {
        self.keys_pressed[game_key as usize]
    }

    pub fn reset_keys_down(&mut self) {
        self.keys_down.fill(false);
    }

    pub fn reset_keys_pressed(&mut self) {
        self.keys_pressed.fill(false);
    }
}
