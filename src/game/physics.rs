use std::f64::consts::PI;
use tracing::{info, warn, error};

use super::game::Game;
use super::types::{DX, DY, Explosion};

pub const MAX_LOCK_ON_RANGE: f64 = 100.0;
pub const BOAT_DETECTION_RANGE: f64 = 25.0;
pub const MISSILE_DODGE_CHANCE: f64 = 0.35;
pub const PLAYER_CANNON_RANGE: f64 = 35.0;
pub const BOAT_AA_RANGE: f64 = 55.0;
pub const MISSILE_MAX_RANGE: f64 = 100.0;

impl Game {
    /// Top-level physics coordinator called once per 40ms tick.
    /// Mirrors Go's updatePhysics() tick order exactly.
    pub fn update_physics(&mut self) {
        if self.game_over {
            return;
        }

        self.ticks += 1;

        // Carrier-destruction sequence (early return)
        if self.carrier_destroying {
            self.destruction_ticks -= 1;
            if self.destruction_ticks % 4 == 0 {
                let ex = self.carrier.x + (rand::random::<u8>() as i32 % self.carrier.width);
                let ey = self.carrier.y + (rand::random::<u8>() as i32 % self.carrier.height);
                self.explosions.push(Explosion { x: ex, y: ey, age: rand::random::<u8>() as i32 % 2 });
                if self.destruction_ticks % 12 == 0 {
                    self.play_sound("explosion");
                }
            }
            if self.destruction_ticks <= 0 {
                self.carrier_destroying = false;
                self.game_over = true;
            }
            self.update_helicopter();
            self.update_explosions();
            self.update_camera();
            return;
        }

        // Tick-order per RUST-PORT.md §Tick-order parity checklist
        self.apply_joystick_input();     // 3
        self.update_helicopter();        // 4
        self.update_camera();            // 5
        self.update_weapon_cooldowns();  // 6
        self.update_carrier_defense();   // 7
        self.update_projectiles();       // 8
        self.update_boats();             // 9
        self.update_stealth_boats();     // 10
        self.update_land_forces();       // 11
        self.update_explosions();        // 12
        self.check_collisions();         // 13
        self.check_wave_completion();    // 14
        self.locked = self.get_locked_target(); // 15
    }

    // -----------------------------------------------------------------------
    // Helicopter
    // -----------------------------------------------------------------------

    pub fn update_helicopter(&mut self) {
        if self.heli.takeoff_cooldown > 0 {
            self.heli.takeoff_cooldown -= 1;
        }

        // Autopilot during carrier destruction: fly back to watch
        if self.carrier_destroying && !self.heli.landed && self.heli.respawn_timer == 0 {
            let pad_x = (self.carrier.x + self.carrier.width / 3) as f64;
            let pad_y = (self.carrier.y + self.carrier.height / 2) as f64;
            let ddx = pad_x - self.heli.x;
            let ddy = pad_y - self.heli.y;
            let dist = (ddx * ddx + ddy * ddy).sqrt();
            if dist > 2.0 {
                let angle = ddy.atan2(ddx);
                let mut deg = angle * (180.0 / PI);
                if deg < 0.0 { deg += 360.0; }
                self.heli.dir = (deg / 45.0).round() as usize % 8;
                let speed = 1.2;
                self.heli.vx = angle.cos() * speed;
                self.heli.vy = angle.sin() * speed;
            } else {
                self.heli.vx = 0.0;
                self.heli.vy = 0.0;
            }
        }

        if self.heli.rotation_cooldown > 0 {
            self.heli.rotation_cooldown -= 1;
        }

        if self.heli.respawn_timer > 0 {
            self.heli.respawn_timer -= 1;
            if self.heli.respawn_timer % 4 == 0 {
                let hx = self.heli.x.round() as i32;
                let hy = self.heli.y.round() as i32;
                self.explosions.push(Explosion {
                    x: hx + (rand::random::<u8>() as i32 % 5) - 2,
                    y: hy + (rand::random::<u8>() as i32 % 3) - 1,
                    age: rand::random::<u8>() as i32 % 3,
                });
            }
            if self.heli.respawn_timer == 0 {
                let pad_x = self.carrier.x + self.carrier.width / 3;
                let pad_y = self.carrier.y + self.carrier.height / 2;
                self.heli.x = pad_x as f64;
                self.heli.y = pad_y as f64;
                self.heli.vx = 0.0;
                self.heli.vy = 0.0;
                self.heli.fuel = 100.0;
                self.heli.armor = 100.0;
                self.heli.missile_ammo = 4;
                self.heli.landed = true;
                self.heli.returning_to_carrier = false;
                self.heli.takeoff_cooldown = 25;
                self.center_camera_on_pad(pad_x, pad_y);
            }
            return;
        }

        if self.heli.landed {
            // Refuel / repair on pad
            if self.heli.fuel < 100.0 {
                self.heli.fuel += 0.4;
                if self.heli.fuel >= 100.0 {
                    self.heli.fuel = 100.0;
                    info!(fuel = self.heli.fuel, "Refueling completed");
                }
            }
            if self.heli.armor < 100.0 {
                self.heli.armor += 0.5;
                if self.heli.armor >= 100.0 {
                    self.heli.armor = 100.0;
                    info!(armor = self.heli.armor, "Repairs completed");
                }
            }
            if self.carrier.health < 100.0 {
                self.carrier.health += 0.2;
                if self.carrier.health >= 100.0 {
                    self.carrier.health = 100.0;
                    info!(health = self.carrier.health, "Carrier fully repaired");
                }
            }
            if self.heli.missile_ammo < 4 {
                self.heli.missile_ammo = 4;
                info!(ammo = self.heli.missile_ammo, "Missiles fully rearmed");
            }
            self.heli.vx = 0.0;
            self.heli.vy = 0.0;
            return;
        }

        // Airborne: burn fuel
        if self.heli.fuel > 0.0 {
            self.heli.fuel -= 0.05;
            if self.heli.fuel <= 0.0 {
                self.heli.fuel = 0.0;
                warn!("Engine failure: Out of fuel");
            }
        }

        let drag = if self.heli.fuel <= 0.0 { 0.85 } else { 0.99 };
        self.heli.x += self.heli.vx;
        self.heli.y += self.heli.vy;
        self.heli.vx *= drag;
        self.heli.vy *= drag;

        let speed = (self.heli.vx * self.heli.vx + self.heli.vy * self.heli.vy).sqrt();
        let max_speed = 1.2;
        if speed > max_speed {
            let ratio = max_speed / speed;
            self.heli.vx *= ratio;
            self.heli.vy *= ratio;
        }

        // Out-of-fuel crash
        if self.heli.fuel <= 0.0 {
            let hx = self.heli.x.round() as i32;
            let hy = self.heli.y.round() as i32;
            let on_carrier = hx >= self.carrier.x
                && hx < self.carrier.x + self.carrier.width
                && hy >= self.carrier.y
                && hy < self.carrier.y + self.carrier.height;
            if !on_carrier && speed < 0.02 {
                warn!(x = self.heli.x, y = self.heli.y, "Helicopter crashed into the ocean: Out of fuel");
                for ddx in -2..=2i32 {
                    for ddy in -1..=1i32 {
                        self.explosions.push(Explosion { x: hx + ddx, y: hy + ddy, age: 0 });
                    }
                }
                self.heli.armor = 0.0;
                let has_incoming = self.missiles.iter().any(|m| m.active && m.is_enemy);
                self.kill_heli(has_incoming);
            }
        }

        // Autopilot: return to carrier after wave clear
        if self.heli.returning_to_carrier {
            let pad_x = (self.carrier.x + self.carrier.width / 3) as f64;
            let pad_y = (self.carrier.y + self.carrier.height / 2) as f64;
            let to_dx = pad_x - self.heli.x;
            let to_dy = pad_y - self.heli.y;
            let dist = (to_dx * to_dx + to_dy * to_dy).sqrt();
            if dist > 0.5 {
                let mut best_dir = 0;
                let mut best_dot = f64::NEG_INFINITY;
                for d in 0..8 {
                    let dot = (to_dx * DX[d] + to_dy * DY[d] * 2.0) / (dist + 0.001);
                    if dot > best_dot {
                        best_dot = dot;
                        best_dir = d;
                    }
                }
                self.heli.dir = best_dir;
                let thrust = if dist < 5.0 { 0.04 } else { 0.10 };
                self.heli.vx += DX[self.heli.dir] * thrust;
                self.heli.vy += DY[self.heli.dir] * thrust;
            }
        }

        // World boundary bounce
        let map_h = self.world_height as f64;
        if self.heli.x < 1.0 { self.heli.x = 1.0; self.heli.vx = -self.heli.vx * 0.4; }
        if self.heli.x > (self.world_width - 2) as f64 { self.heli.x = (self.world_width - 2) as f64; self.heli.vx = -self.heli.vx * 0.4; }
        if self.heli.y < 1.0 { self.heli.y = 1.0; self.heli.vy = -self.heli.vy * 0.4; }
        if self.heli.y > map_h - 2.0 { self.heli.y = map_h - 2.0; self.heli.vy = -self.heli.vy * 0.4; }

        // Auto-land on pad
        let pad_x = self.carrier.x + self.carrier.width / 3;
        let pad_y = self.carrier.y + self.carrier.height / 2;
        let hx = self.heli.x.round() as i32;
        let hy = self.heli.y.round() as i32;
        let aligned = hx >= pad_x - 1 && hx <= pad_x + 1 && hy >= pad_y - 1 && hy <= pad_y + 1;
        let speed = (self.heli.vx * self.heli.vx + self.heli.vy * self.heli.vy).sqrt();
        if aligned && speed < 0.12 && self.heli.takeoff_cooldown == 0 {
            self.heli.landed = true;
            self.heli.returning_to_carrier = false;
            self.heli.x = pad_x as f64;
            self.heli.y = pad_y as f64;
            self.heli.vx = 0.0;
            self.heli.vy = 0.0;
            self.heli.takeoff_cooldown = 25;
            info!(x = self.heli.x, y = self.heli.y, "Auto-landed on carrier pad");
        } else {
            self.heli.rotor_state = (self.heli.rotor_state + 1) % 4;
        }
    }

    // -----------------------------------------------------------------------
    // Camera
    // -----------------------------------------------------------------------

    pub fn update_camera(&mut self) {
        let hx = self.heli.x.round() as i32;
        let hy = self.heli.y.round() as i32;
        let play_h = self.height - 6;
        let margin_w = ((self.width as f64 * 0.30) as i32).max(5);
        let margin_h = ((play_h as f64 * 0.30) as i32).max(3);

        if hx - self.cam_x < margin_w {
            self.cam_x = hx - margin_w;
        } else if hx - self.cam_x > self.width - margin_w {
            self.cam_x = hx - (self.width - margin_w);
        }

        if hy - self.cam_y < margin_h {
            self.cam_y = hy - margin_h;
        } else if hy - self.cam_y > play_h - margin_h {
            self.cam_y = hy - (play_h - margin_h);
        }

        self.clamp_camera();
    }

    fn clamp_camera(&mut self) {
        let play_h = self.height - 6;
        self.cam_x = self.cam_x.max(0);
        if self.world_width > self.width {
            self.cam_x = self.cam_x.min(self.world_width - self.width);
        } else {
            self.cam_x = 0;
        }
        self.cam_y = self.cam_y.max(0);
        if self.world_height > play_h {
            self.cam_y = self.cam_y.min(self.world_height - play_h);
        } else {
            self.cam_y = 0;
        }
    }

    pub fn center_camera_on_pad(&mut self, pad_x: i32, pad_y: i32) {
        self.cam_x = pad_x - self.width / 2;
        self.cam_y = pad_y - (self.height - 6) / 2;
        self.clamp_camera();
    }

    // -----------------------------------------------------------------------
    // Weapon cooldowns
    // -----------------------------------------------------------------------

    fn update_weapon_cooldowns(&mut self) {
        if self.heli.fire_cooldown > 0 { self.heli.fire_cooldown -= 1; }
        if self.heli.missile_cooldown > 0 { self.heli.missile_cooldown -= 1; }
        if self.heli.cannon_heat > 0 { self.heli.cannon_heat -= 1; }
        if self.heli.cannon_jammed > 0 { self.heli.cannon_jammed -= 1; }
    }

    // -----------------------------------------------------------------------
    // Explosions
    // -----------------------------------------------------------------------

    fn update_explosions(&mut self) {
        self.explosions.retain_mut(|e| {
            e.age += 1;
            e.age < 15
        });
    }

    // -----------------------------------------------------------------------
    // Kill helicopter
    // -----------------------------------------------------------------------

    pub fn kill_heli(&mut self, _has_incoming: bool) {
        if self.lives > 0 {
            self.lives -= 1;
        }
        warn!(lives_remaining = self.lives, "Osprey lost");
        if self.lives == 0 {
            self.game_over = true;
            error!("No lives remaining — game over");
        } else {
            self.heli.respawn_timer = 150;
            self.heli.armor = 0.0;
            self.heli.landed = false;
        }
    }

    // -----------------------------------------------------------------------
    // Projectiles
    // -----------------------------------------------------------------------

    fn update_projectiles(&mut self) {
        self.update_bullets();
        self.update_missiles();
    }

    fn update_bullets(&mut self) {
        let ww = self.world_width as f64;
        let wh = self.world_height as f64;
        for b in &mut self.bullets {
            if !b.active { continue; }
            b.x += b.vx;
            b.y += b.vy;
            if b.x < 0.0 || b.x >= ww || b.y < 0.0 || b.y >= wh {
                b.active = false;
                continue;
            }
            let dx = b.x - b.start_x;
            let dy = b.y - b.start_y;
            let travel = (dx * dx + dy * dy).sqrt();
            let range = if b.is_enemy { BOAT_AA_RANGE } else { PLAYER_CANNON_RANGE };
            if travel > range { b.active = false; }
        }
    }

    fn update_missiles(&mut self) {
        let ww = self.world_width as f64;
        let wh = self.world_height as f64;
        let carrier_cx = (self.carrier.x + self.carrier.width / 2) as f64;
        let carrier_cy = (self.carrier.y + self.carrier.height / 2) as f64;

        let mut ciws_spawns: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..self.missiles.len() {
            if !self.missiles[i].active { continue; }

            // Snapshot missile state to avoid borrow conflicts during entity scanning
            let mx = self.missiles[i].x;
            let my = self.missiles[i].y;
            let mvx = self.missiles[i].vx;
            let mvy = self.missiles[i].vy;
            let is_enemy_missile = self.missiles[i].is_enemy;
            let interception_rolled = self.missiles[i].interception_rolled;

            let (new_vx, new_vy, new_ir) = if is_enemy_missile {
                // Enemy missiles home toward the carrier
                let dx = carrier_cx - mx;
                let dy = carrier_cy - my;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= 0.0 {
                    (mvx, mvy, interception_rolled)
                } else {
                    let nx = dx / dist;
                    let ny = dy / dist;
                    let spd = ((mvx * mvx + mvy * mvy).sqrt() + 0.03).min(1.1);
                    (mvx * 0.92 + nx * spd * 0.08, mvy * 0.92 + ny * spd * 0.08, interception_rolled)
                }
            } else {
                // Player/carrier missiles home toward the nearest active enemy
                let mut min_dist = f64::MAX;
                let mut tx = 0.0_f64;
                let mut ty = 0.0_f64;
                let mut has = false;
                let mut ciws_pos: Option<(f64, f64)> = None;

                for b in &self.boats {
                    if !b.active || b.sinking_timer > 0 { continue; }
                    let dx = b.x - mx; let dy = b.y - my;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < min_dist { min_dist = d; tx = b.x; ty = b.y; has = true; ciws_pos = Some((b.x, b.y)); }
                }
                for f in &self.factories {
                    if !f.active || f.sinking_timer > 0 { continue; }
                    let dx = f.x - mx; let dy = f.y - my;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < min_dist { min_dist = d; tx = f.x; ty = f.y; has = true; ciws_pos = None; }
                }
                for t in &self.tanks {
                    if !t.active || t.sinking_timer > 0 { continue; }
                    let dx = t.x - mx; let dy = t.y - my;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < min_dist { min_dist = d; tx = t.x; ty = t.y; has = true; ciws_pos = None; }
                }
                for s in &self.static_aas {
                    if !s.active || s.sinking_timer > 0 { continue; }
                    let dx = s.x - mx; let dy = s.y - my;
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < min_dist { min_dist = d; tx = s.x; ty = s.y; has = true; ciws_pos = None; }
                }

                if !has {
                    (mvx, mvy, interception_rolled)
                } else {
                    let dx = tx - mx;
                    let dy = ty - my;
                    let nx = dx / min_dist;
                    let ny = dy / min_dist;
                    let spd = ((mvx * mvx + mvy * mvy).sqrt() + 0.20).min(5.0);
                    let nv_x = mvx * 0.82 + nx * spd * 0.18;
                    let nv_y = mvy * 0.82 + ny * spd * 0.18;

                    let mut new_ir = interception_rolled;
                    if let Some((bx, by)) = ciws_pos {
                        if !interception_rolled && min_dist < BOAT_DETECTION_RANGE {
                            new_ir = true;
                            if rand::random::<f64>() < 0.10 {
                                let bullet_spd = 3.5;
                                ciws_spawns.push((bx, by, -(dx / min_dist) * bullet_spd, -(dy / min_dist) * bullet_spd));
                            }
                        }
                    }
                    (nv_x, nv_y, new_ir)
                }
            };

            self.missiles[i].vx = new_vx;
            self.missiles[i].vy = new_vy;
            self.missiles[i].interception_rolled = new_ir;
            self.missiles[i].x += new_vx;
            self.missiles[i].y += new_vy;

            if self.missiles[i].x < 0.0 || self.missiles[i].x >= ww
                || self.missiles[i].y < 0.0 || self.missiles[i].y >= wh
            {
                self.missiles[i].active = false;
                continue;
            }
            let dx2 = self.missiles[i].x - self.missiles[i].start_x;
            let dy2 = self.missiles[i].y - self.missiles[i].start_y;
            if (dx2 * dx2 + dy2 * dy2).sqrt() > MISSILE_MAX_RANGE {
                self.missiles[i].active = false;
            }
        }

        // Spawn CIWS countermeasure bullets after missile iteration completes
        use super::types::Bullet;
        for (x, y, vx, vy) in ciws_spawns {
            let b = Bullet { x, y, start_x: x, start_y: y, vx, vy, active: true, is_enemy: true, is_countermeasure: true };
            if let Some(slot) = self.bullets.iter().position(|b| !b.active) {
                self.bullets[slot] = b;
            } else if self.bullets.len() < 24 {
                self.bullets.push(b);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Collisions
    // -----------------------------------------------------------------------

    fn check_collisions(&mut self) {
        self.check_drone_missile_interceptions();
        self.check_bullet_collisions();
        self.check_missile_collisions();
        self.check_player_bullets_vs_enemy_missiles();
        self.check_enemy_bullets_vs_player_missiles();
        self.check_stealth_boat_vs_carrier();
    }

    fn check_drone_missile_interceptions(&mut self) {
        for mi in 0..self.missiles.len() {
            if !self.missiles[mi].active { continue; }
            for di in 0..self.drones.len() {
                if !self.drones[di].active { continue; }
                let is_factory = !self.missiles[mi].is_enemy && self.drones[di].factory_idx >= 0;
                let is_carrier_def = self.missiles[mi].is_enemy && self.drones[di].factory_idx == -1;
                if !is_factory && !is_carrier_def { continue; }
                let m_x = self.missiles[mi].x;
                let m_y = self.missiles[mi].y;
                let d_x = self.drones[di].x;
                let d_y = self.drones[di].y;
                let ddx = d_x - m_x;
                let ddy = d_y - m_y;
                if (ddx * ddx + ddy * ddy).sqrt() < 4.0 {
                    self.missiles[mi].active = false;
                    self.drones[di].active = false;
                    let mid_x = ((m_x + d_x) / 2.0).round() as i32;
                    let mid_y = ((m_y + d_y) / 2.0).round() as i32;
                    if is_factory {
                        info!(missile_idx = mi, drone_idx = di, "drone shield: air defense drone neutralized player guided missile");
                    } else {
                        info!(missile_idx = mi, drone_idx = di, "carrier drone shield: defense drone neutralized enemy guided missile");
                    }
                    for ox in -1i32..=1 {
                        for oy in -1i32..=1 {
                            self.explosions.push(Explosion { x: mid_x + ox, y: mid_y + oy, age: rand::random::<u8>() as i32 % 4 });
                        }
                    }
                    break;
                }
            }
        }
    }

    fn check_bullet_collisions(&mut self) {
        for i in 0..self.bullets.len() {
            if !self.bullets[i].active { continue; }
            if self.bullets[i].is_enemy {
                self.check_enemy_bullet_vs_player(i);
            } else {
                self.check_player_bullet_vs_targets(i);
            }
        }
    }

    fn check_enemy_bullet_vs_player(&mut self, bi: usize) {
        if self.heli.landed || self.heli.armor <= 0.0 { return; }
        let bx = self.bullets[bi].x;
        let by = self.bullets[bi].y;
        if !aabb(bx, by, self.heli.x, self.heli.y, 3.5, 2.5) { return; }

        self.bullets[bi].active = false;
        self.heli.armor -= 15.0;
        info!(damage = 15.0, armor_remaining = self.heli.armor, "enemy bullet hit player");
        self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
        self.play_sound("explosion");

        if self.heli.armor <= 0.0 {
            self.heli.armor = 0.0;
            warn!(x = self.heli.x, y = self.heli.y, "helicopter destroyed by enemy bullet");
            let hx = self.heli.x.round() as i32;
            let hy = self.heli.y.round() as i32;
            for ddx in -3i32..=3 {
                for ddy in -2i32..=2 {
                    self.explosions.push(Explosion { x: hx + ddx, y: hy + ddy, age: 0 });
                }
            }
            let has_incoming = self.missiles.iter().any(|m| m.active && m.is_enemy);
            self.kill_heli(has_incoming);
        }
    }

    fn check_player_bullet_vs_targets(&mut self, bi: usize) {
        let bx = self.bullets[bi].x;
        let by = self.bullets[bi].y;

        // vs Boats
        for j in 0..self.boats.len() {
            if !self.boats[j].active { continue; }
            if aabb(bx, by, self.boats[j].x, self.boats[j].y, 5.5, 1.5) {
                self.bullets[bi].active = false;
                if self.boats[j].sinking_timer == 0 {
                    self.boats[j].health -= 1;
                    info!(boat_idx = j, health = self.boats[j].health, max_health = self.boats[j].max_health,
                        bullet_x = bx, bullet_y = by, "player bullet hit boat");
                    self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
                    self.play_sound("explosion");
                    if self.boats[j].health <= 0 {
                        self.boats[j].active = false;
                        info!(boat_idx = j, "boat sunk by cannon round");
                        let bx2 = self.boats[j].x.round() as i32;
                        let by2 = self.boats[j].y.round() as i32;
                        for ddx in -5i32..=5 {
                            for ddy in -1i32..=1 {
                                self.explosions.push(Explosion { x: bx2 + ddx, y: by2 + ddy, age: 0 });
                            }
                        }
                    }
                } else {
                    info!(boat_idx = j, "player bullet hit already-sinking boat");
                    self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
                    self.play_sound("explosion");
                }
                return;
            }
        }

        // vs Stealth boats
        for i in 0..self.stealth_boats.len() {
            if !self.stealth_boats[i].active { continue; }
            if aabb(bx, by, self.stealth_boats[i].x, self.stealth_boats[i].y, 4.0, 1.0) {
                self.bullets[bi].active = false;
                self.stealth_boats[i].active = false;
                info!(idx = i, bullet_x = bx, bullet_y = by, "stealth drone speedboat destroyed by cannon");
                self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
                self.play_sound("explosion");
                return;
            }
        }

        // vs Drones
        for d in 0..self.drones.len() {
            if !self.drones[d].active { continue; }
            let drone_x = self.drones[d].x;
            let drone_y = self.drones[d].y;
            if aabb(bx, by, drone_x, drone_y, 1.5, 1.2) {
                self.bullets[bi].active = false;
                self.drones[d].active = false;
                info!(drone_idx = d, bullet_x = bx, bullet_y = by, "player shot down air defense drone");
                self.explosions.push(Explosion { x: drone_x.round() as i32, y: drone_y.round() as i32, age: 0 });
                self.play_sound("explosion");
                return;
            }
        }

        // vs Factories
        for f in 0..self.factories.len() {
            if !self.factories[f].active || self.factories[f].sinking_timer != 0 { continue; }
            if aabb(bx, by, self.factories[f].x, self.factories[f].y, 8.5, 2.5) {
                self.bullets[bi].active = false;
                self.factories[f].health -= 1;
                info!(factory_idx = f, health = self.factories[f].health, max_health = self.factories[f].max_health,
                    bullet_x = bx, bullet_y = by, "player bullet hit factory");
                self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
                self.play_sound("explosion");
                if self.factories[f].health <= 0 {
                    self.factories[f].sinking_timer = 45;
                    info!(factory_idx = f, "factory destroyed by cannon");
                }
                return;
            }
        }

        // vs Tanks
        for t in 0..self.tanks.len() {
            if !self.tanks[t].active || self.tanks[t].sinking_timer != 0 { continue; }
            if aabb(bx, by, self.tanks[t].x, self.tanks[t].y, 2.5, 1.5) {
                self.bullets[bi].active = false;
                self.tanks[t].health -= 1;
                info!(tank_idx = t, health = self.tanks[t].health, max_health = self.tanks[t].max_health,
                    bullet_x = bx, bullet_y = by, "player bullet hit tank");
                self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
                self.play_sound("explosion");
                if self.tanks[t].health <= 0 {
                    self.tanks[t].sinking_timer = 45;
                    info!(tank_idx = t, "tank destroyed by player cannon");
                }
                return;
            }
        }

        // vs Static AA
        for s in 0..self.static_aas.len() {
            if !self.static_aas[s].active || self.static_aas[s].sinking_timer != 0 { continue; }
            if aabb(bx, by, self.static_aas[s].x, self.static_aas[s].y, 1.5, 1.5) {
                self.bullets[bi].active = false;
                self.static_aas[s].health -= 1;
                info!(aa_idx = s, health = self.static_aas[s].health, max_health = self.static_aas[s].max_health,
                    bullet_x = bx, bullet_y = by, "player bullet hit static AA");
                self.explosions.push(Explosion { x: bx.round() as i32, y: by.round() as i32, age: 0 });
                self.play_sound("explosion");
                if self.static_aas[s].health <= 0 {
                    self.static_aas[s].sinking_timer = 45;
                    info!(aa_idx = s, "static AA destroyed by player cannon");
                }
                return;
            }
        }
    }

    fn check_missile_collisions(&mut self) {
        for i in 0..self.missiles.len() {
            if !self.missiles[i].active { continue; }
            if self.missiles[i].is_enemy {
                self.check_enemy_missile_vs_carrier(i);
            } else {
                self.check_player_missile_vs_targets(i);
            }
        }
    }

    fn check_enemy_missile_vs_carrier(&mut self, mi: usize) {
        let mx = self.missiles[mi].x.round() as i32;
        let my = self.missiles[mi].y.round() as i32;
        if mx < self.carrier.x || mx >= self.carrier.x + self.carrier.width
            || my < self.carrier.y || my >= self.carrier.y + self.carrier.height
        {
            return;
        }
        self.missiles[mi].active = false;
        self.carrier.health -= 25.0;
        warn!(damage = 25.0, carrier_health = self.carrier.health, "enemy guided missile hit the carrier");
        self.play_sound("explosion");
        for ddx in -2i32..=2 {
            for ddy in -1i32..=1 {
                self.explosions.push(Explosion { x: mx + ddx, y: my + ddy, age: rand::random::<u8>() as i32 % 4 });
            }
        }
        if self.carrier.health <= 0.0 {
            self.carrier.health = 0.0;
            error!("CRITICAL: aircraft carrier destroyed");
            let cx = self.carrier.x + self.carrier.width / 2;
            let cy = self.carrier.y + self.carrier.height / 2;
            for ddx in -4i32..=4 {
                for ddy in -2i32..=2 {
                    self.explosions.push(Explosion { x: cx + ddx, y: cy + ddy, age: rand::random::<u8>() as i32 % 5 });
                }
            }
            self.game_over = true;
        }
    }

    fn check_player_missile_vs_targets(&mut self, mi: usize) {
        let mx = self.missiles[mi].x;
        let my = self.missiles[mi].y;

        // vs Boats
        for j in 0..self.boats.len() {
            if !self.boats[j].active { continue; }
            if aabb(mx, my, self.boats[j].x, self.boats[j].y, 5.5, 1.5) {
                self.missiles[mi].active = false;
                if self.boats[j].sinking_timer == 0 {
                    self.boats[j].sinking_timer = 45;
                    self.boats[j].health = 0;
                    info!(boat_idx = j, missile_x = mx, missile_y = my, "player guided missile hit boat — sinking initiated");
                } else {
                    info!(boat_idx = j, "player guided missile hit already-sinking boat");
                }
                self.explosions.push(Explosion { x: mx.round() as i32, y: my.round() as i32, age: 0 });
                self.play_sound("explosion");
                return;
            }
        }

        // vs Factories
        for f in 0..self.factories.len() {
            if !self.factories[f].active || self.factories[f].sinking_timer != 0 { continue; }
            if aabb(mx, my, self.factories[f].x, self.factories[f].y, 8.5, 2.5) {
                self.missiles[mi].active = false;
                self.factories[f].health -= 10;
                info!(factory_idx = f, health = self.factories[f].health, missile_x = mx, missile_y = my, "player guided missile hit factory");
                self.explosions.push(Explosion { x: mx.round() as i32, y: my.round() as i32, age: 0 });
                self.play_sound("explosion");
                if self.factories[f].health <= 0 {
                    self.factories[f].health = 0;
                    self.factories[f].sinking_timer = 45;
                    info!(factory_idx = f, "factory destroyed by guided missile");
                }
                return;
            }
        }

        // vs Tanks
        for t in 0..self.tanks.len() {
            if !self.tanks[t].active || self.tanks[t].sinking_timer != 0 { continue; }
            if aabb(mx, my, self.tanks[t].x, self.tanks[t].y, 2.5, 1.5) {
                self.missiles[mi].active = false;
                self.tanks[t].sinking_timer = 45;
                self.tanks[t].health = 0;
                info!(tank_idx = t, missile_x = mx, missile_y = my, "player guided missile hit tank (critical hit)");
                self.explosions.push(Explosion { x: mx.round() as i32, y: my.round() as i32, age: 0 });
                self.play_sound("explosion");
                return;
            }
        }

        // vs Static AA
        for s in 0..self.static_aas.len() {
            if !self.static_aas[s].active || self.static_aas[s].sinking_timer != 0 { continue; }
            if aabb(mx, my, self.static_aas[s].x, self.static_aas[s].y, 1.5, 1.5) {
                self.missiles[mi].active = false;
                self.static_aas[s].sinking_timer = 45;
                self.static_aas[s].health = 0;
                info!(aa_idx = s, missile_x = mx, missile_y = my, "player guided missile hit static AA (critical hit)");
                self.explosions.push(Explosion { x: mx.round() as i32, y: my.round() as i32, age: 0 });
                self.play_sound("explosion");
                return;
            }
        }
    }

    fn check_player_bullets_vs_enemy_missiles(&mut self) {
        'outer: for bi in 0..self.bullets.len() {
            if !self.bullets[bi].active || self.bullets[bi].is_enemy { continue; }
            for mi in 0..self.missiles.len() {
                if !self.missiles[mi].active || !self.missiles[mi].is_enemy { continue; }
                if aabb(self.bullets[bi].x, self.bullets[bi].y, self.missiles[mi].x, self.missiles[mi].y, 1.5, 1.5) {
                    let ex = self.missiles[mi].x.round() as i32;
                    let ey = self.missiles[mi].y.round() as i32;
                    self.bullets[bi].active = false;
                    self.missiles[mi].active = false;
                    self.explosions.push(Explosion { x: ex, y: ey, age: 0 });
                    self.play_sound("explosion");
                    info!(missile_idx = mi, bullet_idx = bi, "player manually intercepted enemy missile");
                    continue 'outer;
                }
            }
        }
    }

    fn check_enemy_bullets_vs_player_missiles(&mut self) {
        'outer: for bi in 0..self.bullets.len() {
            if !self.bullets[bi].active || !self.bullets[bi].is_enemy { continue; }
            for mi in 0..self.missiles.len() {
                if !self.missiles[mi].active || self.missiles[mi].is_enemy { continue; }
                if aabb(self.bullets[bi].x, self.bullets[bi].y, self.missiles[mi].x, self.missiles[mi].y, 1.5, 1.5) {
                    if rand::random::<f64>() < MISSILE_DODGE_CHANCE {
                        info!(missile_idx = mi, bullet_idx = bi, "missile dodged enemy anti-aircraft fire");
                        continue 'outer;
                    }
                    let ex = self.missiles[mi].x.round() as i32;
                    let ey = self.missiles[mi].y.round() as i32;
                    self.bullets[bi].active = false;
                    self.missiles[mi].active = false;
                    self.explosions.push(Explosion { x: ex, y: ey, age: 0 });
                    self.play_sound("explosion");
                    info!(missile_idx = mi, bullet_idx = bi, "CIWS interception: guided missile shot down by boat anti-air fire");
                    continue 'outer;
                }
            }
        }
    }

    fn check_stealth_boat_vs_carrier(&mut self) {
        let carrier_cx = (self.carrier.x + self.carrier.width / 2) as f64;
        let carrier_cy = (self.carrier.y + self.carrier.height / 2) as f64;
        let half_w = (self.carrier.width / 2) as f64 + 1.0;
        let half_h = (self.carrier.height / 2) as f64 + 1.0;
        for i in 0..self.stealth_boats.len() {
            if !self.stealth_boats[i].active { continue; }
            if aabb(self.stealth_boats[i].x, self.stealth_boats[i].y, carrier_cx, carrier_cy, half_w, half_h) {
                self.stealth_boats[i].active = false;
                error!("stealth drone speedboat rammed the carrier — carrier destroyed!");
                self.carrier_destroying = true;
                self.destruction_ticks = 80;
                self.carrier.health = 0.0;
                let cw = self.carrier.width;
                let ch = self.carrier.height;
                let cx = self.carrier.x;
                let cy = self.carrier.y;
                for _ in 0..15 {
                    let ex = cx + (rand::random::<u8>() as i32 % cw);
                    let ey = cy + (rand::random::<u8>() as i32 % ch);
                    self.explosions.push(Explosion { x: ex, y: ey, age: rand::random::<u8>() as i32 % 3 });
                }
                self.play_sound("explosion");
            }
        }
    }

    // -----------------------------------------------------------------------
    // Remaining stubs (enemy AI, waves)
    // -----------------------------------------------------------------------

    fn tick_sinking(
        &mut self,
        timer: &mut i32,
        x: f64,
        y: f64,
        scatter_x: i32,
        scatter_y: i32,
        grid_w: i32,
        grid_h: i32,
        max_final_age: i32,
    ) -> bool {
        *timer -= 1;
        if *timer % 3 == 0 {
            let mut rng = rand::rng();
            use rand::Rng;
            let off_x = rng.random_range(-scatter_x..=scatter_x) as f64;
            let off_y = rng.random_range(-scatter_y..=scatter_y) as f64;
            self.explosions.push(Explosion {
                x: (x + off_x).round() as i32,
                y: (y + off_y).round() as i32,
                age: 0,
            });
        }
        if *timer > 0 {
            return false;
        }
        let mut rng = rand::rng();
        use rand::Rng;
        let age = if max_final_age > 0 {
            rng.random_range(0..max_final_age)
        } else {
            0
        };
        for ddx in -grid_w..=grid_w {
            for ddy in -grid_h..=grid_h {
                self.explosions.push(Explosion {
                    x: x.round() as i32 + ddx,
                    y: y.round() as i32 + ddy,
                    age,
                });
            }
        }
        true
    }

    fn apply_blast_damage(&mut self, x: f64, y: f64, blast_radius: f64, damage: f64) {
        if self.heli.landed || self.heli.armor <= 0.0 || self.heli.respawn_timer > 0 {
            return;
        }
        let dx = self.heli.x - x;
        let dy = self.heli.y - y;
        if (dx * dx + dy * dy).sqrt() > blast_radius {
            return;
        }
        self.heli.armor -= damage;
        if self.heli.armor < 0.0 {
            self.heli.armor = 0.0;
        }
        warn!(
            blast_origin_x = x,
            blast_origin_y = y,
            damage = damage,
            armor_remaining = self.heli.armor,
            "Helicopter caught in secondary explosion blast!"
        );

        if self.heli.armor <= 0.0 {
            warn!(x = self.heli.x, y = self.heli.y, "Helicopter destroyed by secondary explosion!");
            let hx = self.heli.x.round() as i32;
            let hy = self.heli.y.round() as i32;
            for ddx in -2..=2 {
                for ddy in -1..=1 {
                    self.explosions.push(Explosion {
                        x: hx + ddx,
                        y: hy + ddy,
                        age: 0,
                    });
                }
            }
            let has_incoming = self.missiles.iter().any(|m| m.active && m.is_enemy);
            self.kill_heli(has_incoming);
        }
    }

    fn tick_aa_fire(
        &mut self,
        x: f64,
        y: f64,
        cooldown: &mut i32,
        aa_range: f64,
        speed: f64,
        cooldown_min: i32,
        cooldown_rand: i32,
    ) -> bool {
        if *cooldown > 0 {
            *cooldown -= 1;
            return false;
        }
        if self.heli.landed || self.heli.fuel <= 0.0 || self.heli.armor <= 0.0 {
            return false;
        }
        let dx = self.heli.x - x;
        let dy = self.heli.y - y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist <= 0.0 || dist >= aa_range {
            return false;
        }
        self.spawn_enemy_bullet(x, y, (dx / dist) * speed, (dy / dist) * speed);
        let mut rng = rand::rng();
        use rand::Rng;
        *cooldown = cooldown_min + rng.random_range(0..cooldown_rand);
        true
    }

    fn replenish_carrier_drones(&mut self) {
        if !self.heli.landed || self.ticks % 100 != 0 {
            return;
        }

        let mut carrier_drones_count = 0;
        for d in 0..self.drones.len() {
            if self.drones[d].active && self.drones[d].factory_idx == -1 {
                carrier_drones_count += 1;
            }
        }
        if carrier_drones_count >= 3 {
            return;
        }

        let mut angle = 0.0;
        if carrier_drones_count == 1 {
            for d in 0..self.drones.len() {
                if self.drones[d].active && self.drones[d].factory_idx == -1 {
                    angle = self.drones[d].angle + 2.0 * PI / 3.0;
                    break;
                }
            }
        } else if carrier_drones_count == 2 {
            let mut angles = Vec::new();
            for d in 0..self.drones.len() {
                if self.drones[d].active && self.drones[d].factory_idx == -1 {
                    angles.push(self.drones[d].angle);
                }
            }
            if angles.len() == 2 {
                let mid = (angles[0] + angles[1]) / 2.0;
                if (angles[0] - angles[1]).abs() > PI {
                    angle = mid;
                } else {
                    angle = mid + PI;
                }
            }
        }
        let cx = (self.carrier.x + self.carrier.width / 2) as f64;
        let cy = (self.carrier.y + self.carrier.height / 2) as f64;

        use super::types::Drone;
        self.append_drone(Drone {
            x: cx,
            y: cy,
            vx: 0.0,
            vy: 0.0,
            active: true,
            angle,
            factory_idx: -1,
        });
        info!("Carrier repaired/spawned defensive carrier drone!");
    }

    fn update_carrier_defense(&mut self) {
        self.replenish_carrier_drones();

        if self.carrier.health <= 0.0 {
            return;
        }
        if self.carrier.missile_cooldown > 0 {
            self.carrier.missile_cooldown -= 1;
            return;
        }

        let cx = (self.carrier.x + self.carrier.width / 2) as f64;
        let cy = (self.carrier.y + self.carrier.height / 2) as f64;
        let mut target_idx: Option<usize> = None;
        let mut min_dist = 45.0;

        for i in 0..self.boats.len() {
            let boat = &self.boats[i];
            if !boat.active || boat.sinking_timer > 0 {
                continue;
            }
            let dx_vec = boat.x - cx;
            let dy_vec = boat.y - cy;
            let dist = (dx_vec * dx_vec + dy_vec * dy_vec).sqrt();
            if dist < min_dist {
                min_dist = dist;
                target_idx = Some(i);
            }
        }

        let target_idx = match target_idx {
            Some(idx) => idx,
            None => return,
        };

        let target_x = self.boats[target_idx].x;
        let target_y = self.boats[target_idx].y;
        let dx_vec = target_x - cx;
        let dy_vec = target_y - cy;
        let dist = (dx_vec * dx_vec + dy_vec * dy_vec).sqrt();
        let initial_speed = 0.5;
        let mvx = (dx_vec / dist) * initial_speed;
        let mvy = (dy_vec / dist) * initial_speed;

        self.spawn_carrier_missile(cx, cy, mvx, mvy);
        info!(
            boat_x = target_x,
            dist = min_dist,
            "Carrier launched defensive SSM at enemy boat!"
        );
        let mut rng = rand::rng();
        use rand::Rng;
        self.carrier.missile_cooldown = 300 + rng.random_range(0..150);
    }

    fn update_boats(&mut self) {
        const BOAT_MISSILE_RANGE: f64 = 80.0;

        for i in 0..self.boats.len() {
            if !self.boats[i].active {
                continue;
            }

            if self.boats[i].sinking_timer > 0 {
                let bx = self.boats[i].x;
                let by = self.boats[i].y;
                let mut st = self.boats[i].sinking_timer;
                if self.tick_sinking(&mut st, bx, by, 5, 1, 5, 1, 0) {
                    self.apply_blast_damage(bx, by, 7.0, 20.0);
                    self.boats[i].active = false;
                    info!(boat_idx = i, "Doomed boat has fully sunk");
                    self.boats[i].sinking_timer = st;
                    continue;
                }
                self.boats[i].sinking_timer = st;
            }

            let mut speed_mult = 1.0;
            if self.boats[i].sinking_timer > 0 {
                speed_mult = 0.25;
            }
            self.boats[i].x += self.boats[i].vx * speed_mult;

            let threshold = self.get_coastline_threshold(self.boats[i].y.round() as i32);
            let mut max_water_x = threshold - 7.0;
            if max_water_x > (self.world_width - 7) as f64 {
                max_water_x = (self.world_width - 7) as f64;
            }
            if self.boats[i].x < self.boats[i].patrol_min_x || self.boats[i].x > max_water_x {
                self.boats[i].vx = -self.boats[i].vx;
                self.boats[i].x += self.boats[i].vx * speed_mult;
                if self.boats[i].x < self.boats[i].patrol_min_x {
                    self.boats[i].x = self.boats[i].patrol_min_x;
                } else if self.boats[i].x > max_water_x {
                    self.boats[i].x = max_water_x;
                }
            }

            if self.boats[i].sinking_timer > 0 {
                continue;
            }

            // Advance patrol front toward carrier, stopping at missile range.
            let missile_stop_x = (self.carrier.x + self.carrier.width / 2) as f64 + BOAT_MISSILE_RANGE;
            if self.boats[i].patrol_min_x > missile_stop_x {
                self.boats[i].patrol_min_x -= 0.02;
                if self.boats[i].patrol_min_x < missile_stop_x {
                    self.boats[i].patrol_min_x = missile_stop_x;
                }
            }

            // AA fire against the helicopter
            let bx = self.boats[i].x;
            let by = self.boats[i].y;
            let mut fc = self.boats[i].fire_cooldown;
            if self.tick_aa_fire(bx, by, &mut fc, BOAT_AA_RANGE, 2.0, 60, 80) {
                info!(boat_idx = i, x = bx, y = by, "Boat fired anti-aircraft projectile");
            }
            self.boats[i].fire_cooldown = fc;

            // Guided missile at carrier
            if self.boats[i].missile_cooldown > 0 {
                self.boats[i].missile_cooldown -= 1;
            } else {
                let target_x = (self.carrier.x + self.carrier.width / 2) as f64;
                let target_y = (self.carrier.y + self.carrier.height / 2) as f64;
                let dx_vec = target_x - bx;
                let dy_vec = target_y - by;
                let dist = (dx_vec * dx_vec + dy_vec * dy_vec).sqrt();
                if dist > 0.0 {
                    let speed = 0.3;
                    self.spawn_enemy_missile(bx, by, (dx_vec / dist) * speed, (dy_vec / dist) * speed);
                    info!(
                        boat_idx = i,
                        boat_x = bx,
                        boat_y = by,
                        "Boat launched guided missile at Carrier!"
                    );
                    self.play_sound("missile");
                }
                let mut rng = rand::rng();
                use rand::Rng;
                self.boats[i].missile_cooldown = 600 + rng.random_range(0..400);
            }
        }
    }

    fn update_stealth_boats(&mut self) {
        const MINUTE_TICKS: i32 = 1500;
        const TICKS_PER_SEC: i32 = MINUTE_TICKS / 60;
        const WARN_DIST_SQ: f64 = 71.0 * 71.0;

        if self.ticks > 0 && self.ticks % MINUTE_TICKS == 0 {
            let none_active = self.stealth_boats.iter().all(|sb| !sb.active);
            if none_active && self.stealth_spawn_at == 0 {
                let mut chance = self.wave * 10;
                if chance > 80 {
                    chance = 80;
                }
                let mut rng = rand::rng();
                use rand::Rng;
                if rng.random_range(0..100) < chance {
                    let delay_sec = 1 + rng.random_range(0..10);
                    self.stealth_spawn_at = self.ticks + delay_sec * TICKS_PER_SEC;
                }
            }
        }

        if self.stealth_spawn_at > 0 && self.ticks >= self.stealth_spawn_at {
            self.stealth_spawn_at = 0;
            self.spawn_stealth_boat();
        }

        for i in 0..self.stealth_boats.len() {
            if !self.stealth_boats[i].active {
                continue;
            }
            let vx = self.stealth_boats[i].vx;
            self.stealth_boats[i].x += vx;
            if self.stealth_boats[i].x < 0.0 {
                info!(idx = i, "Stealth drone speedboat exited map without hitting carrier");
                self.stealth_boats[i].active = false;
            }
        }

        let carrier_cx = (self.carrier.x + self.carrier.width / 2) as f64;
        let carrier_cy = (self.carrier.y + self.carrier.height / 2) as f64;
        self.stealth_near = false;
        for sb in &self.stealth_boats {
            if !sb.active {
                continue;
            }
            let dx = sb.x - carrier_cx;
            let dy = sb.y - carrier_cy;
            if dx * dx + dy * dy < WARN_DIST_SQ {
                self.stealth_near = true;
                break;
            }
        }
        if self.stealth_near {
            if self.ticks % 20 == 0 {
                self.play_sound("warning");
            }
            if self.ticks % 15 == 0 {
                self.play_sound("speedboat");
            }
        }
    }

    fn spawn_stealth_boat(&mut self) {
        let carrier_cy = (self.carrier.y + self.carrier.height / 2) as f64;
        let mut rng = rand::rng();
        use rand::Rng;
        let spawn_y = carrier_cy + rng.random_range(-3..=3) as f64;
        let spawn_x = self.get_coastline_threshold(spawn_y.round() as i32) - 3.0;

        use super::types::StealthBoat;
        let sb = StealthBoat {
            x: spawn_x,
            y: spawn_y,
            vx: -0.42,
            active: true,
        };

        for i in 0..self.stealth_boats.len() {
            if !self.stealth_boats[i].active {
                self.stealth_boats[i] = sb.clone();
                warn!(x = spawn_x, y = spawn_y, wave = self.wave, "Stealth drone speedboat launched!");
                return;
            }
        }
        self.stealth_boats.push(sb);
        warn!(x = spawn_x, y = spawn_y, wave = self.wave, "Stealth drone speedboat launched!");
    }

    fn update_land_forces(&mut self) {
        self.update_factories();
        self.update_drone_orbits();
        self.update_tanks();
        self.update_static_aas();
    }

    fn update_factories(&mut self) {
        for f_idx in 0..self.factories.len() {
            if !self.factories[f_idx].active {
                continue;
            }

            if self.factories[f_idx].sinking_timer > 0 {
                let fx = self.factories[f_idx].x;
                let fy = self.factories[f_idx].y;
                let mut st = self.factories[f_idx].sinking_timer;
                if self.tick_sinking(&mut st, fx, fy, 3, 1, 6, 2, 4) {
                    self.apply_blast_damage(fx, fy, 9.0, 25.0);
                    self.factories[f_idx].active = false;
                    info!(idx = f_idx, "Enemy military Factory has been completely destroyed!");
                    for d in 0..self.drones.len() {
                        if self.drones[d].active && self.drones[d].factory_idx == f_idx as i32 {
                            self.drones[d].active = false;
                            self.explosions.push(Explosion {
                                x: self.drones[d].x.round() as i32,
                                y: self.drones[d].y.round() as i32,
                                age: 0,
                            });
                        }
                    }
                }
                self.factories[f_idx].sinking_timer = st;
            }

            // Factory AA fire
            if self.factories[f_idx].active && self.factories[f_idx].sinking_timer == 0 {
                let fx = self.factories[f_idx].x;
                let fy = self.factories[f_idx].y;
                let mut fc = self.factories[f_idx].fire_cooldown;
                if self.tick_aa_fire(fx, fy, &mut fc, BOAT_AA_RANGE, 2.0, 40, 40) {
                    info!(x = fx, y = fy, idx = f_idx, "Factory fired fortress anti-aircraft projectile!");
                }
                self.factories[f_idx].fire_cooldown = fc;
            }

            // Factory ground-launched missile at Carrier (Wave 4+)
            if self.wave >= 4 && self.factories[f_idx].active && self.factories[f_idx].sinking_timer == 0 {
                if (self.ticks + f_idx as i32 * 200) % 800 == 0 {
                    let target_x = (self.carrier.x + self.carrier.width / 2) as f64;
                    let target_y = (self.carrier.y + self.carrier.height / 2) as f64;
                    let fx = self.factories[f_idx].x;
                    let fy = self.factories[f_idx].y;
                    let dx_vec = target_x - fx;
                    let dy_vec = target_y - fy;
                    let dist = (dx_vec * dx_vec + dy_vec * dy_vec).sqrt();
                    if dist > 0.0 {
                        let speed = 0.25;
                        self.spawn_enemy_missile(fx, fy, (dx_vec / dist) * speed, (dy_vec / dist) * speed);
                        info!(
                            factory_idx = f_idx,
                            fact_x = fx,
                            fact_y = fy,
                            "Factory launched fortress ground missile at Carrier!"
                        );
                        self.play_sound("missile");
                    }
                }
            }

            // Factory drone replenishment
            if self.factories[f_idx].active && self.factories[f_idx].sinking_timer == 0 && self.ticks % 100 == 0 {
                let mut active_count = 0;
                for d in 0..self.drones.len() {
                    if self.drones[d].active && self.drones[d].factory_idx == f_idx as i32 {
                        active_count += 1;
                    }
                }

                if active_count < 2 && self.factories[f_idx].drones_remaining > 0 {
                    let mut angle = 0.0;
                    if active_count > 0 {
                        for d in 0..self.drones.len() {
                            if self.drones[d].active && self.drones[d].factory_idx == f_idx as i32 {
                                angle = self.drones[d].angle + PI;
                                break;
                            }
                        }
                    }

                    use super::types::Drone;
                    let fx = self.factories[f_idx].x;
                    let fy = self.factories[f_idx].y;
                    self.append_drone(Drone {
                        x: fx,
                        y: fy,
                        vx: 0.0,
                        vy: 0.0,
                        active: true,
                        angle,
                        factory_idx: f_idx as i32,
                    });
                    self.factories[f_idx].drones_remaining -= 1;
                    info!(
                        factory_idx = f_idx,
                        reserves_remaining = self.factories[f_idx].drones_remaining,
                        "Factory spawned replacement defense drone!"
                    );
                }
            }
        }
    }

    fn update_drone_orbits(&mut self) {
        for d in 0..self.drones.len() {
            if !self.drones[d].active {
                continue;
            }
            let f_idx = self.drones[d].factory_idx;
            if f_idx >= 0 && (f_idx as usize) < self.factories.len() {
                let fact = &self.factories[f_idx as usize];
                if fact.active && fact.sinking_timer == 0 {
                    self.drones[d].angle += 0.045;
                    let radius = 8.0;
                    self.drones[d].x = fact.x + self.drones[d].angle.cos() * radius;
                    self.drones[d].y = fact.y + self.drones[d].angle.sin() * radius * 0.5;
                }
            } else if f_idx == -1 {
                self.drones[d].angle += 0.035;
                let cx = (self.carrier.x + self.carrier.width / 2) as f64;
                let cy = (self.carrier.y + self.carrier.height / 2) as f64;
                let radius = 12.0;
                self.drones[d].x = cx + self.drones[d].angle.cos() * radius;
                self.drones[d].y = cy + self.drones[d].angle.sin() * radius * 0.5;
            }
        }
    }

    fn update_tanks(&mut self) {
        for t_idx in 0..self.tanks.len() {
            if !self.tanks[t_idx].active {
                continue;
            }

            if self.tanks[t_idx].sinking_timer > 0 {
                let tx = self.tanks[t_idx].x;
                let ty = self.tanks[t_idx].y;
                let mut st = self.tanks[t_idx].sinking_timer;
                if self.tick_sinking(&mut st, tx, ty, 1, 1, 2, 1, 3) {
                    self.apply_blast_damage(tx, ty, 4.0, 15.0);
                    self.tanks[t_idx].active = false;
                    info!(tank_idx = t_idx, "Patrolling Tank has fully blown up!");
                    self.tanks[t_idx].sinking_timer = st;
                    continue;
                }
                self.tanks[t_idx].sinking_timer = st;
            }

            if self.tanks[t_idx].sinking_timer == 0 {
                if self.tanks[t_idx].patrol_dir == 0 {
                    self.tanks[t_idx].y += self.tanks[t_idx].vy;
                    if self.tanks[t_idx].y < self.tanks[t_idx].min_coord {
                        self.tanks[t_idx].y = self.tanks[t_idx].min_coord;
                        self.tanks[t_idx].vy = -self.tanks[t_idx].vy;
                    } else if self.tanks[t_idx].y > self.tanks[t_idx].max_coord {
                        self.tanks[t_idx].y = self.tanks[t_idx].max_coord;
                        self.tanks[t_idx].vy = -self.tanks[t_idx].vy;
                    }
                } else {
                    self.tanks[t_idx].x += self.tanks[t_idx].vx;
                    if self.tanks[t_idx].x < self.tanks[t_idx].min_coord {
                        self.tanks[t_idx].x = self.tanks[t_idx].min_coord;
                        self.tanks[t_idx].vx = -self.tanks[t_idx].vx;
                    } else if self.tanks[t_idx].x > self.tanks[t_idx].max_coord {
                        self.tanks[t_idx].x = self.tanks[t_idx].max_coord;
                        self.tanks[t_idx].vx = -self.tanks[t_idx].vx;
                    }
                }
            }

            if self.tanks[t_idx].active && self.tanks[t_idx].sinking_timer == 0 {
                let tx = self.tanks[t_idx].x;
                let ty = self.tanks[t_idx].y;
                let mut fc = self.tanks[t_idx].fire_cooldown;
                if self.tick_aa_fire(tx, ty, &mut fc, 40.0, 2.2, 50, 40) {
                    info!(tank_idx = t_idx, x = tx, y = ty, "Tank fired flak projectile!");
                }
                self.tanks[t_idx].fire_cooldown = fc;
            }
        }
    }

    fn update_static_aas(&mut self) {
        for sa_idx in 0..self.static_aas.len() {
            if !self.static_aas[sa_idx].active {
                continue;
            }

            if self.static_aas[sa_idx].sinking_timer > 0 {
                let sax = self.static_aas[sa_idx].x;
                let say = self.static_aas[sa_idx].y;
                let mut st = self.static_aas[sa_idx].sinking_timer;
                if self.tick_sinking(&mut st, sax, say, 1, 1, 2, 1, 3) {
                    self.apply_blast_damage(sax, say, 4.0, 15.0);
                    self.static_aas[sa_idx].active = false;
                    info!(idx = sa_idx, "Static AA has fully blown up!");
                    self.static_aas[sa_idx].sinking_timer = st;
                    continue;
                }
                self.static_aas[sa_idx].sinking_timer = st;
            }

            if self.static_aas[sa_idx].active && self.static_aas[sa_idx].sinking_timer == 0 {
                let sax = self.static_aas[sa_idx].x;
                let say = self.static_aas[sa_idx].y;
                let mut fc = self.static_aas[sa_idx].fire_cooldown;
                if self.tick_aa_fire(sax, say, &mut fc, 45.0, 2.2, 45, 35) {
                    info!(idx = sa_idx, x = sax, y = say, "Static AA fired flak projectile!");
                }
                self.static_aas[sa_idx].fire_cooldown = fc;
            }
        }
    }

    fn check_wave_completion(&mut self) {
        let mut all_sunk = true;
        for i in 0..self.boats.len() {
            if self.boats[i].active {
                all_sunk = false;
                break;
            }
        }
        if all_sunk {
            for f_idx in 0..self.factories.len() {
                if self.factories[f_idx].active {
                    all_sunk = false;
                    break;
                }
            }
        }
        if all_sunk {
            for t_idx in 0..self.tanks.len() {
                if self.tanks[t_idx].active {
                    all_sunk = false;
                    break;
                }
            }
        }
        if all_sunk {
            for sa_idx in 0..self.static_aas.len() {
                if self.static_aas[sa_idx].active {
                    all_sunk = false;
                    break;
                }
            }
        }

        if !all_sunk {
            return;
        }

        self.wave += 1;
        self.lives = 5;
        self.play_sound("explosion");
        info!(
            wave = self.wave,
            speed_multiplier = 1.25,
            "All enemy assets destroyed! Advancing to next wave"
        );

        if !self.heli.landed && self.heli.armor > 0.0 && self.heli.respawn_timer == 0 {
            self.heli.returning_to_carrier = true;
            info!("Wave cleared - Osprey returning to carrier");
        }

        let mut rng = rand::rng();
        use rand::Rng;

        for i in 0..self.boats.len() {
            self.boats[i].active = true;
            self.boats[i].health = self.boats[i].max_health;
            self.boats[i].sinking_timer = 0;
            self.boats[i].x = self.initial_boats[i].x;
            self.boats[i].y = self.initial_boats[i].y;
            let by = self.boats[i].y.round() as i32;
            let thresh = self.get_coastline_threshold(by);
            self.boats[i].patrol_min_x = thresh - 18.0;
            let new_speed = self.boats[i].vx * 1.25;
            if new_speed.abs() > 2.0 {
                if new_speed < 0.0 {
                    self.boats[i].vx = -2.0;
                } else {
                    self.boats[i].vx = 2.0;
                }
            } else {
                self.boats[i].vx = new_speed;
            }
            self.boats[i].missile_cooldown = 600 + rng.random_range(0..400);
        }

        self.reset_factories();
        self.reset_drones();

        for t_idx in 0..self.tanks.len() {
            self.tanks[t_idx].active = self.wave >= 2;
            self.tanks[t_idx].health = self.tanks[t_idx].max_health;
            self.tanks[t_idx].sinking_timer = 0;
            self.tanks[t_idx].fire_cooldown = 60 + rng.random_range(0..100);
            if self.tanks[t_idx].patrol_dir == 0 {
                let new_speed = self.tanks[t_idx].vy * 1.25;
                if new_speed.abs() > 2.0 {
                    if new_speed < 0.0 {
                        self.tanks[t_idx].vy = -2.0;
                    } else {
                        self.tanks[t_idx].vy = 2.0;
                    }
                } else {
                    self.tanks[t_idx].vy = new_speed;
                }
            } else {
                let new_speed = self.tanks[t_idx].vx * 1.25;
                if new_speed.abs() > 2.0 {
                    if new_speed < 0.0 {
                        self.tanks[t_idx].vx = -2.0;
                    } else {
                        self.tanks[t_idx].vx = 2.0;
                    }
                } else {
                    self.tanks[t_idx].vx = new_speed;
                }
            }
        }

        self.reset_static_aas(self.wave >= 3);
    }
}

#[inline]
fn aabb(ax: f64, ay: f64, bx: f64, by: f64, hw: f64, hh: f64) -> bool {
    (ax - bx).abs() < hw && (ay - by).abs() < hh
}
