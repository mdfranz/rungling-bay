use std::collections::HashMap;
use std::f64::consts::PI;

use rand::Rng;

use super::types::*;

pub struct Game {
    pub width: i32,
    pub height: i32,
    pub world_width: i32,
    pub world_height: i32,
    pub cam_x: i32,
    pub cam_y: i32,
    pub quit_confirming: bool,
    pub game_over: bool,
    pub carrier_destroying: bool,
    pub destruction_ticks: i32,
    pub wave: i32,
    pub heli: Helicopter,
    pub carrier: Carrier,
    pub bullets: Vec<Bullet>,
    pub missiles: Vec<Missile>,
    pub boats: Vec<Boat>,
    pub initial_boats: Vec<Boat>,
    pub island: Island,
    pub factories: Vec<Factory>,
    pub drones: Vec<Drone>,
    pub tanks: Vec<Tank>,
    pub static_aas: Vec<StaticAA>,
    pub stealth_boats: Vec<StealthBoat>,
    pub stealth_spawn_at: i32,
    pub stealth_near: bool,
    pub explosions: Vec<Explosion>,
    pub lives: i32,
    pub ticks: i32,
    pub locked: LockedTarget,
    pub joystick_axes: HashMap<u8, i16>,
    pub joystick_buttons: HashMap<u8, bool>,
    pub joystick_last_btn: HashMap<u8, bool>,
    pub audio_tx: Option<std::sync::mpsc::Sender<super::sound::SoundType>>,
}

impl Game {
    pub fn new(width: i32, height: i32, audio_tx: Option<std::sync::mpsc::Sender<super::sound::SoundType>>) -> Self {
        let playable_height = (height - 6).max(10);
        let world_width = (width * 2).max(80);
        let world_height = playable_height * 2;

        let carrier = Carrier {
            x: world_width / 10,
            y: world_height / 4,
            width: 26,
            height: 6,
            health: 100.0,
            missile_cooldown: 0,
        };

        let pad_x = carrier.x + carrier.width / 3;
        let pad_y = carrier.y + carrier.height / 2;

        let heli = Helicopter {
            x: pad_x as f64,
            y: pad_y as f64,
            vx: 0.0,
            vy: 0.0,
            dir: 0,
            rotor_state: 0,
            landed: true,
            fuel: 100.0,
            armor: 100.0,
            fire_cooldown: 0,
            takeoff_cooldown: 0,
            missile_cooldown: 0,
            missile_ammo: 4,
            respawn_timer: 0,
            cannon_heat: 0,
            cannon_jammed: 0,
            returning_to_carrier: false,
            rotation_cooldown: 0,
        };

        let boats = vec![
            Boat {
                x: 15.0,
                y: (world_height - 10) as f64,
                vx: -0.05,
                health: 9,
                max_health: 9,
                active: true,
                fire_cooldown: 0,
                missile_cooldown: 1500,
                sinking_timer: 0,
                patrol_min_x: 0.0,
            },
            Boat {
                x: 20.0,
                y: 6.0,
                vx: -0.04,
                health: 9,
                max_health: 9,
                active: true,
                fire_cooldown: 0,
                missile_cooldown: 2000,
                sinking_timer: 0,
                patrol_min_x: 0.0,
            },
            Boat {
                x: 25.0,
                y: (world_height - 7) as f64,
                vx: -0.06,
                health: 9,
                max_health: 9,
                active: true,
                fire_cooldown: 0,
                missile_cooldown: 2500,
                sinking_timer: 0,
                patrol_min_x: 0.0,
            },
        ];

        let factories = vec![
            Factory {
                x: (world_width * 2 / 3) as f64,
                y: (world_height / 8) as f64,
                health: 25,
                max_health: 25,
                active: true,
                fire_cooldown: 100,
                sinking_timer: 0,
                drones_remaining: 8,
            },
            Factory {
                x: (world_width - 35) as f64,
                y: (world_height / 2) as f64,
                health: 25,
                max_health: 25,
                active: true,
                fire_cooldown: 150,
                sinking_timer: 0,
                drones_remaining: 8,
            },
            Factory {
                x: (world_width - 15) as f64,
                y: (world_height * 7 / 8) as f64,
                health: 25,
                max_health: 25,
                active: true,
                fire_cooldown: 200,
                sinking_timer: 0,
                drones_remaining: 8,
            },
        ];

        let mut drones: Vec<Drone> = Vec::with_capacity(factories.len() * 2 + 5);
        for (i, f) in factories.iter().enumerate() {
            drones.push(Drone {
                x: f.x + 8.0,
                y: f.y,
                vx: 0.0,
                vy: 0.0,
                active: true,
                angle: 0.0,
                factory_idx: i as i32,
            });
            drones.push(Drone {
                x: f.x - 8.0,
                y: f.y,
                vx: 0.0,
                vy: 0.0,
                active: true,
                angle: PI,
                factory_idx: i as i32,
            });
        }
        let cx = (carrier.x + carrier.width / 2) as f64;
        let cy = (carrier.y + carrier.height / 2) as f64;
        drones.push(Drone { x: cx + 12.0, y: cy, vx: 0.0, vy: 0.0, active: true, angle: 0.0, factory_idx: -1 });
        drones.push(Drone { x: cx, y: cy, vx: 0.0, vy: 0.0, active: true, angle: 2.0 * PI / 3.0, factory_idx: -1 });
        drones.push(Drone { x: cx, y: cy, vx: 0.0, vy: 0.0, active: true, angle: 4.0 * PI / 3.0, factory_idx: -1 });

        let tanks = vec![
            Tank {
                x: (world_width - 15) as f64,
                y: (world_height * 5 / 16) as f64,
                vx: 0.0,
                vy: 0.04,
                health: 6,
                max_health: 6,
                active: false,
                fire_cooldown: 0,
                sinking_timer: 0,
                patrol_dir: 0,
                min_coord: (world_height / 8) as f64,
                max_coord: (world_height / 2) as f64,
            },
            Tank {
                x: (world_width - 15) as f64,
                y: (world_height * 11 / 16) as f64,
                vx: 0.0,
                vy: -0.04,
                health: 6,
                max_health: 6,
                active: false,
                fire_cooldown: 0,
                sinking_timer: 0,
                patrol_dir: 0,
                min_coord: (world_height / 2) as f64,
                max_coord: (world_height * 7 / 8) as f64,
            },
            Tank {
                x: (world_width - 11) as f64,
                y: (world_height / 2) as f64,
                vx: 0.06,
                vy: 0.0,
                health: 6,
                max_health: 6,
                active: false,
                fire_cooldown: 0,
                sinking_timer: 0,
                patrol_dir: 1,
                min_coord: (world_width - 15) as f64,
                max_coord: (world_width - 7) as f64,
            },
        ];

        // Initial camera centered on heli/pad
        let heli_x = pad_x;
        let heli_y = pad_y;
        let mut cam_x = heli_x - width / 2;
        let mut cam_y = heli_y - playable_height / 2;
        cam_x = cam_x.clamp(0, (world_width - width).max(0));
        cam_y = cam_y.clamp(0, (world_height - playable_height).max(0));

        let initial_boats = boats.clone();

        let mut g = Game {
            width,
            height,
            world_width,
            world_height,
            cam_x,
            cam_y,
            quit_confirming: false,
            game_over: false,
            carrier_destroying: false,
            destruction_ticks: 0,
            wave: 1,
            heli,
            carrier,
            bullets: Vec::with_capacity(16),
            missiles: Vec::with_capacity(2),
            boats,
            initial_boats,
            island: Island { x: 0, y: 0, width: 0, height: 0, active: true },
            factories,
            drones,
            tanks,
            static_aas: Vec::new(),
            stealth_boats: Vec::with_capacity(2),
            stealth_spawn_at: 0,
            stealth_near: false,
            explosions: Vec::with_capacity(8),
            lives: 5,
            ticks: 0,
            locked: LockedTarget::None,
            joystick_axes: HashMap::new(),
            joystick_buttons: HashMap::new(),
            joystick_last_btn: HashMap::new(),
            audio_tx,
        };

        // Snap boats to coastline (matching Go's post-init loop)
        for i in 0..g.boats.len() {
            let by = g.boats[i].y.round() as i32;
            let thresh = g.get_coastline_threshold(by);
            g.boats[i].x = thresh - 8.0;
            g.boats[i].patrol_min_x = g.boats[i].x - 10.0;
        }
        g.initial_boats = g.boats.clone();

        g.init_static_aas();
        g
    }

    // --- Terrain helpers (also used by draw) ---

    pub fn get_coastline_threshold(&self, y: i32) -> f64 {
        let h = if self.world_height > 0 { self.world_height as f64 } else { 1.0 };
        let wiggle = (y as f64 * 0.7).sin() * 2.0 + (y as f64 * 0.3).cos() * 1.0;
        self.world_width as f64 / 3.0
            + (y as f64 / h * PI).sin() * (self.world_width as f64 / 2.2)
            + wiggle
    }

    pub fn get_coastline_style(&self, x: i32, y: i32) -> (bool, bool) {
        if !self.island.active {
            return (false, false);
        }
        let threshold = self.get_coastline_threshold(y);
        if (x as f64) >= threshold {
            let is_sand = (x as f64) < threshold + 3.0;
            (true, is_sand)
        } else {
            (false, false)
        }
    }

    pub fn is_road(&self, x: i32, y: i32) -> bool {
        let h = self.world_height;
        let w = self.world_width;
        if x >= w - 16 && x <= w - 14 && y >= h / 8 && y <= h * 7 / 8 {
            return true;
        }
        if y >= h / 2 - 1 && y <= h / 2 + 1 && x >= w - 15 && x <= w - 7 {
            return true;
        }
        false
    }

    fn init_static_aas(&mut self) {
        let h = if self.world_height > 0 { self.world_height } else { 1 };
        let mut rng = rand::rng();

        self.static_aas = (0..6)
            .map(|i| {
                let y = ((h as f64 * (i as f64 + 0.5) / 6.0) as i32).clamp(2, h - 3);
                let threshold = self.get_coastline_threshold(y);
                let mut x = threshold + 6.0;
                while self.is_road(x.round() as i32, y) {
                    x += 3.0;
                }
                StaticAA {
                    x,
                    y: y as f64,
                    health: 4,
                    max_health: 4,
                    active: self.wave >= 3,
                    fire_cooldown: 30 + rng.random_range(0..100i32),
                    sinking_timer: 0,
                }
            })
            .collect();
    }

    // --- Spawn helpers (used by input + physics) ---

    pub fn spawn_player_bullet(&mut self, x: f64, y: f64, vx: f64, vy: f64) {
        use super::types::Bullet;
        let b = Bullet {
            x, y, start_x: x, start_y: y, vx, vy,
            active: true, is_enemy: false, is_countermeasure: false,
        };
        if let Some(slot) = self.bullets.iter().position(|bullet| !bullet.active) {
            self.bullets[slot] = b;
        } else if self.bullets.len() < 16 {
            self.bullets.push(b);
        }
    }

    pub fn spawn_player_missile(&mut self, x: f64, y: f64, vx: f64, vy: f64) {
        use super::types::Missile;
        let m = Missile {
            x, y, start_x: x, start_y: y, vx, vy,
            active: true, interception_rolled: false, is_enemy: false, is_carrier: false,
        };
        if let Some(slot) = self.missiles.iter().position(|missile| !missile.active) {
            self.missiles[slot] = m;
        } else if self.missiles.len() < 16 {
            self.missiles.push(m);
        }
    }

    pub fn spawn_carrier_missile(&mut self, x: f64, y: f64, vx: f64, vy: f64) {
        use super::types::Missile;
        let m = Missile {
            x, y, start_x: x, start_y: y, vx, vy,
            active: true, interception_rolled: false, is_enemy: false, is_carrier: true,
        };
        if let Some(slot) = self.missiles.iter().position(|missile| !missile.active) {
            self.missiles[slot] = m;
        } else if self.missiles.len() < 16 {
            self.missiles.push(m);
        }
    }

    pub fn spawn_enemy_bullet(&mut self, x: f64, y: f64, vx: f64, vy: f64) {
        use super::types::Bullet;
        let b = Bullet {
            x, y, start_x: x, start_y: y, vx, vy,
            active: true, is_enemy: true, is_countermeasure: false,
        };
        if let Some(slot) = self.bullets.iter().position(|bullet| !bullet.active) {
            self.bullets[slot] = b;
        } else if self.bullets.len() < 24 {
            self.bullets.push(b);
        }
    }

    pub fn spawn_enemy_missile(&mut self, x: f64, y: f64, vx: f64, vy: f64) {
        use super::types::Missile;
        let m = Missile {
            x, y, start_x: x, start_y: y, vx, vy,
            active: true, interception_rolled: false, is_enemy: true, is_carrier: false,
        };
        if let Some(slot) = self.missiles.iter().position(|missile| !missile.active) {
            self.missiles[slot] = m;
        } else if self.missiles.len() < 16 {
            self.missiles.push(m);
        }
    }

    pub fn append_drone(&mut self, drone: super::types::Drone) {
        if let Some(slot) = self.drones.iter().position(|d| !d.active) {
            self.drones[slot] = drone;
        } else {
            self.drones.push(drone);
        }
    }

    pub fn play_sound(&self, name: &str) {
        if let Some(tx) = &self.audio_tx {
            let sound_type = match name {
                "warning" => super::sound::SoundType::Warning,
                "laser" => super::sound::SoundType::Laser,
                "missile" => super::sound::SoundType::Missile,
                "explosion" => super::sound::SoundType::Explosion,
                "speedboat" => super::sound::SoundType::Speedboat,
                _ => return,
            };
            let _ = tx.send(sound_type);
        }
    }

    pub fn reset_factories(&mut self) {
        for f in &mut self.factories {
            f.active = true;
            f.health = f.max_health;
            f.sinking_timer = 0;
            f.drones_remaining = 5;
        }
    }

    pub fn reset_drones(&mut self) {
        self.drones.clear();
    }

    pub fn reset_static_aas(&mut self, active: bool) {
        for sa in &mut self.static_aas {
            sa.active = active;
            sa.health = sa.max_health;
            sa.sinking_timer = 0;
        }
    }
}
