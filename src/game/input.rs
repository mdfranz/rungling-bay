use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tracing::{debug, info, warn};

use super::game::Game;
use super::types::{LockedTarget, DX, DY, DIR_NAMES};

/// Messages sent from the keyboard/controller reader thread to the main loop.
pub enum InputMsg {
    Key(KeyEvent),
    Resize(u16, u16),
}

impl Game {
    /// Top-level key dispatcher. Returns `false` when the app should quit.
    pub fn handle_raw_key(&mut self, key: &KeyEvent) -> bool {
        if self.game_over {
            return false; // any key exits after game over
        }

        if self.quit_confirming {
            let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('c');
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => return false,
                _ if ctrl_c => return false,
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.quit_confirming = false;
                }
                _ => {}
            }
            return true;
        }

        let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('c');
        if key.code == KeyCode::Esc || ctrl_c {
            self.quit_confirming = true;
            return true;
        }

        self.handle_key_press(key);
        true
    }

    fn handle_key_press(&mut self, key: &KeyEvent) {
        if self.heli.armor <= 0.0
            || self.heli.respawn_timer > 0
            || self.heli.returning_to_carrier
            || self.carrier_destroying
        {
            return;
        }

        let pad_x = self.carrier.x + self.carrier.width / 3;
        let pad_y = self.carrier.y + self.carrier.height / 2;
        let hx = self.heli.x.round() as i32;
        let hy = self.heli.y.round() as i32;
        let aligned =
            hx >= pad_x - 1 && hx <= pad_x + 1 && hy >= pad_y - 1 && hy <= pad_y + 1;

        // On the carrier pad: only take off
        if self.heli.landed {
            match key.code {
                KeyCode::Char(' ')
                | KeyCode::Up
                | KeyCode::Char('w')
                | KeyCode::Char('W')
                | KeyCode::Char('l')
                | KeyCode::Char('L') if self.heli.takeoff_cooldown == 0 => {
                    self.heli.landed = false;
                    self.heli.vy = -0.1;
                    self.heli.takeoff_cooldown = 25;
                    info!(x = self.heli.x, y = self.heli.y, "Takeoff initiated");
                }
                _ => {}
            }
            return;
        }

        let thrust = if self.heli.fuel <= 0.0 { 0.0 } else { 0.18 };
        let dir = self.heli.dir;

        // Manual land
        if matches!(key.code, KeyCode::Char('l') | KeyCode::Char('L'))
            && self.heli.takeoff_cooldown == 0
        {
            let speed = (self.heli.vx * self.heli.vx + self.heli.vy * self.heli.vy).sqrt();
            if aligned && speed < 0.25 {
                self.heli.landed = true;
                self.heli.x = pad_x as f64;
                self.heli.y = pad_y as f64;
                self.heli.vx = 0.0;
                self.heli.vy = 0.0;
                self.heli.takeoff_cooldown = 25;
                info!(x = self.heli.x, y = self.heli.y, "Landed on carrier pad");
                return;
            }
        }

        // Cannon fire
        if key.code == KeyCode::Char(' ')
            && self.heli.fire_cooldown == 0
            && self.heli.cannon_jammed == 0
            && self.heli.fuel > 0.0
        {
            let bx = self.heli.x + DX[dir] * 1.5;
            let by = self.heli.y + DY[dir] * 1.5;
            let vx = DX[dir] * 2.0;
            let vy = DY[dir] * 2.0;
            self.spawn_player_bullet(bx, by, vx, vy);
            self.play_sound("laser");
            self.heli.fire_cooldown = 4;
            const MAX_HEAT: i32 = 20;
            self.heli.cannon_heat += 5;
            debug!(heli_x = self.heli.x, heli_y = self.heli.y,
                spawn_x = bx, spawn_y = by, vx, vy,
                dir = DIR_NAMES[dir], "cannon fired");
            if self.heli.cannon_heat >= MAX_HEAT {
                self.heli.cannon_heat = 0;
                self.heli.cannon_jammed = 60;
                self.heli.armor = (self.heli.armor - 5.0).max(0.0);
                warn!(jam_ticks = 60, armor_remaining = self.heli.armor, "Cannon overheated! Barrel jammed.");
            }
        }

        // Missile fire
        if matches!(
            key.code,
            KeyCode::Char('f') | KeyCode::Char('F') | KeyCode::Char('m') | KeyCode::Char('M')
        ) && self.heli.missile_cooldown == 0
            && self.heli.fuel > 0.0
            && self.heli.missile_ammo > 0
        {
            if self.locked == LockedTarget::None {
                warn!("Missile launch aborted: No target locked within +/- 45 degree forward aperture!");
                return;
            }
            let active_count = self
                .missiles
                .iter()
                .filter(|m| m.active && !m.is_enemy && !m.is_carrier)
                .count();
            if active_count < 2 {
                let mx = self.heli.x + DX[dir] * 1.5;
                let my = self.heli.y + DY[dir] * 1.5;
                let mvx = DX[dir] * 0.5;
                let mvy = DY[dir] * 0.5;
                self.spawn_player_missile(mx, my, mvx, mvy);
                self.heli.missile_ammo -= 1;
                self.play_sound("missile");
                self.heli.missile_cooldown = 12;
                debug!(heli_x = self.heli.x, heli_y = self.heli.y,
                    spawn_x = mx, spawn_y = my, vx = mvx, vy = mvy,
                    dir = DIR_NAMES[dir], ammo_remaining = self.heli.missile_ammo, "missile fired");
            }
        }

        // Direction and thrust (combined from Go's two switch blocks)
        match key.code {
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                self.heli.dir = (dir + 7) % 8;
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                self.heli.dir = (dir + 1) % 8;
            }
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                self.heli.vx += DX[dir] * thrust;
                self.heli.vy += DY[dir] * thrust;
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                self.heli.vx *= 0.3;
                self.heli.vy *= 0.3;
            }
            _ => {}
        }
    }

    pub fn apply_joystick_input(&mut self) {
        // Skip entirely if no joystick events have been received yet
        if self.joystick_axes.is_empty() && self.joystick_buttons.is_empty() {
            return;
        }
        if self.heli.armor <= 0.0 || self.heli.respawn_timer > 0 {
            return;
        }

        // Snapshot all input state up front to avoid borrow conflicts with spawn_* calls
        let ax = |n: u8| self.joystick_axes.get(&n).copied().unwrap_or(0) as f64 / 32767.0;
        let tx = (ax(0) + ax(6)).clamp(-1.0, 1.0);
        let ty = (ax(1) + ax(7)).clamp(-1.0, 1.0);
        let right_stick_y = ax(4);
        let guns_trigger = ax(5);
        let b0 = *self.joystick_buttons.get(&0).unwrap_or(&false);
        let b1 = *self.joystick_buttons.get(&1).unwrap_or(&false);
        let b2 = *self.joystick_buttons.get(&2).unwrap_or(&false);
        let b3 = *self.joystick_buttons.get(&3).unwrap_or(&false);
        let b6 = *self.joystick_buttons.get(&6).unwrap_or(&false);
        let b7 = *self.joystick_buttons.get(&7).unwrap_or(&false);

        let pad_x = self.carrier.x + self.carrier.width / 3;
        let pad_y = self.carrier.y + self.carrier.height / 2;
        let hx = self.heli.x.round() as i32;
        let hy = self.heli.y.round() as i32;
        let aligned =
            hx >= pad_x - 1 && hx <= pad_x + 1 && hy >= pad_y - 1 && hy <= pad_y + 1;

        if b0 {
            if self.heli.landed {
                if self.heli.takeoff_cooldown == 0 {
                    self.heli.landed = false;
                    self.heli.vy = -0.1;
                    self.heli.takeoff_cooldown = 25;
                    info!(x = self.heli.x, y = self.heli.y, "Takeoff initiated");
                }
            } else if self.heli.takeoff_cooldown == 0 {
                let speed = (self.heli.vx * self.heli.vx + self.heli.vy * self.heli.vy).sqrt();
                if aligned && speed < 0.25 {
                    self.heli.landed = true;
                    self.heli.x = pad_x as f64;
                    self.heli.y = pad_y as f64;
                    self.heli.vx = 0.0;
                    self.heli.vy = 0.0;
                    self.heli.takeoff_cooldown = 25;
                    info!(x = self.heli.x, y = self.heli.y, "Landed on carrier pad");
                }
            }
            return;
        }

        if self.heli.landed {
            return;
        }

        let thrust = if self.heli.fuel <= 0.0 { 0.0 } else { 0.18 };
        let dir = self.heli.dir;

        if tx * tx + ty * ty > 0.01 {
            self.heli.vx += tx * thrust * 0.2;
            self.heli.vy += ty * thrust * 0.2;
        }

        if self.heli.rotation_cooldown == 0 {
            if tx < -0.3 {
                self.heli.dir = (dir + 7) % 8;
                self.heli.rotation_cooldown = 4;
            } else if tx > 0.3 {
                self.heli.dir = (dir + 1) % 8;
                self.heli.rotation_cooldown = 4;
            }
        }

        if right_stick_y < -0.3 {
            self.heli.vx += DX[dir] * thrust;
            self.heli.vy += DY[dir] * thrust;
        } else if right_stick_y > 0.3 {
            self.heli.vx *= 0.3;
            self.heli.vy *= 0.3;
        }

        if (b2 || b6 || guns_trigger > 0.5) && self.heli.fire_cooldown == 0 && self.heli.fuel > 0.0 {
            let bx = self.heli.x + DX[dir] * 1.5;
            let by = self.heli.y + DY[dir] * 1.5;
            self.spawn_player_bullet(bx, by, DX[dir] * 2.0, DY[dir] * 2.0);
            self.play_sound("laser");
            self.heli.fire_cooldown = 4;
            info!(dir = self.heli.dir, degrees = self.heli.dir * 45, "Aerial cannon fired (joystick)");
        }

        let joystick_missile_pressed = b1 || b3 || b7;
        if joystick_missile_pressed
            && self.heli.missile_cooldown == 0
            && self.heli.fuel > 0.0
            && self.heli.missile_ammo > 0
        {
            if self.locked == LockedTarget::None {
                warn!("Missile launch aborted: No target locked within +/- 45 degree forward aperture!");
                return;
            }
            let active_count = self
                .missiles
                .iter()
                .filter(|m| m.active && !m.is_enemy && !m.is_carrier)
                .count();
            if active_count < 2 {
                let mx = self.heli.x + DX[dir] * 1.5;
                let my = self.heli.y + DY[dir] * 1.5;
                self.spawn_player_missile(mx, my, DX[dir] * 0.5, DY[dir] * 0.5);
                self.heli.missile_ammo -= 1;
                self.play_sound("missile");
                self.heli.missile_cooldown = 12;
                info!(dir = self.heli.dir, degrees = self.heli.dir * 45, ammo_remaining = self.heli.missile_ammo, "Guided missile fired (joystick)");
            }
        }
    }

    pub fn get_locked_target(&self) -> LockedTarget {
        use super::types::Targetable;
        use super::physics::MAX_LOCK_ON_RANGE;

        let dir = self.heli.dir;
        let mut fwd_x = DX[dir];
        let mut fwd_y = DY[dir] * 2.0;
        let len = (fwd_x * fwd_x + fwd_y * fwd_y).sqrt();
        if len > 0.0 {
            fwd_x /= len;
            fwd_y /= len;
        }

        let mut locked = LockedTarget::None;
        let mut min_dist = f64::MAX;

        fn check_slice<T: Targetable>(
            slice: &[T],
            heli_x: f64,
            heli_y: f64,
            fwd_x: f64,
            fwd_y: f64,
            min_dist: &mut f64,
            locked: &mut LockedTarget,
        ) {
            for (i, t) in slice.iter().enumerate() {
                if !t.is_active() || t.sinking_timer() > 0 {
                    continue;
                }
                let (tx, ty) = t.position();
                let ddx = tx - heli_x;
                let ddy = (ty - heli_y) * 2.0;
                let dist = (ddx * ddx + ddy * ddy).sqrt();
                if dist <= 0.0 || dist > MAX_LOCK_ON_RANGE {
                    continue;
                }
                let dot = fwd_x * (ddx / dist) + fwd_y * (ddy / dist);
                if dot >= 0.707 && dist < *min_dist {
                    *min_dist = dist;
                    *locked = T::to_locked_variant(i);
                }
            }
        }

        check_slice(&self.boats,      self.heli.x, self.heli.y, fwd_x, fwd_y, &mut min_dist, &mut locked);
        check_slice(&self.factories,  self.heli.x, self.heli.y, fwd_x, fwd_y, &mut min_dist, &mut locked);
        check_slice(&self.tanks,      self.heli.x, self.heli.y, fwd_x, fwd_y, &mut min_dist, &mut locked);
        check_slice(&self.static_aas, self.heli.x, self.heli.y, fwd_x, fwd_y, &mut min_dist, &mut locked);

        locked
    }
}
