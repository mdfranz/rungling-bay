// Directions: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
pub const DIR_NAMES: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
pub const DIR_DEGREES: [i32; 8] = [0, 45, 90, 135, 180, 225, 270, 315];

// Y is pre-scaled by 0.5 for terminal cell aspect ratio
pub const DX: [f64; 8] = [0.0, 0.707, 1.0, 0.707, 0.0, -0.707, -1.0, -0.707];
pub const DY: [f64; 8] = [-0.5, -0.354, 0.0, 0.354, 0.5, 0.354, 0.0, -0.354];

// 7x5 sprites for 8 directions; '*' is replaced by spinning rotor frame
pub const SPRITES: [[[char; 7]; 5]; 8] = [
    // 0: North
    [
        [' ', ' ', ' ', '▲', ' ', ' ', ' '],
        [' ', '*', '═', '#', '═', '*', ' '],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        [' ', ' ', ' ', '+', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' '],
    ],
    // 1: NE
    [
        [' ', ' ', ' ', ' ', '▲', ' ', ' '],
        [' ', ' ', '*', '═', '#', '═', '*'],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        [' ', ' ', '+', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' '],
    ],
    // 2: East
    [
        [' ', ' ', ' ', ' ', '*', ' ', ' '],
        [' ', ' ', '+', ' ', '║', ' ', ' '],
        [' ', ' ', '=', '#', '#', '►', ' '],
        [' ', ' ', '+', ' ', '║', ' ', ' '],
        [' ', ' ', ' ', ' ', '*', ' ', ' '],
    ],
    // 3: SE
    [
        [' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', '\\', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        [' ', ' ', '*', '═', '#', '═', '*'],
        [' ', ' ', ' ', ' ', ' ', '▼', ' '],
    ],
    // 4: South
    [
        [' ', ' ', ' ', ' ', ' ', ' ', ' '],
        [' ', ' ', ' ', '+', ' ', ' ', ' '],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        [' ', '*', '═', '#', '═', '*', ' '],
        [' ', ' ', ' ', '▼', ' ', ' ', ' '],
    ],
    // 5: SW
    [
        [' ', ' ', ' ', ' ', ' ', '/', ' '],
        [' ', ' ', ' ', ' ', '/', ' ', ' '],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        ['*', '═', '#', '═', '*', ' ', ' '],
        [' ', '▼', ' ', ' ', ' ', ' ', ' '],
    ],
    // 6: West
    [
        [' ', ' ', '*', ' ', ' ', ' ', ' '],
        [' ', ' ', '║', ' ', '+', ' ', ' '],
        [' ', '◄', '#', '#', '=', ' ', ' '],
        [' ', ' ', '║', ' ', '+', ' ', ' '],
        [' ', ' ', '*', ' ', ' ', ' ', ' '],
    ],
    // 7: NW
    [
        [' ', '▲', ' ', ' ', ' ', ' ', ' '],
        ['*', '═', '#', '═', '*', ' ', ' '],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', '\\', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', '\\', ' '],
    ],
];

pub const ROTOR_FRAMES: [char; 4] = ['|', '/', '-', '\\'];

#[derive(Debug, Clone)]
pub struct Carrier {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub health: f64,
    pub missile_cooldown: i32,
}

#[derive(Debug, Clone)]
pub struct Bullet {
    pub x: f64,
    pub y: f64,
    pub start_x: f64,
    pub start_y: f64,
    pub vx: f64,
    pub vy: f64,
    pub active: bool,
    pub is_enemy: bool,
    pub is_countermeasure: bool,
}

#[derive(Debug, Clone)]
pub struct Missile {
    pub x: f64,
    pub y: f64,
    pub start_x: f64,
    pub start_y: f64,
    pub vx: f64,
    pub vy: f64,
    pub active: bool,
    pub interception_rolled: bool,
    pub is_enemy: bool,
    pub is_carrier: bool,
}

#[derive(Debug, Clone)]
pub struct Boat {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub health: i32,
    pub max_health: i32,
    pub active: bool,
    pub fire_cooldown: i32,
    pub missile_cooldown: i32,
    pub sinking_timer: i32,
    pub patrol_min_x: f64,
}

#[derive(Debug, Clone)]
pub struct Explosion {
    pub x: i32,
    pub y: i32,
    pub age: i32,
}

#[derive(Debug, Clone)]
pub struct Helicopter {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub dir: usize,
    pub rotor_state: usize,
    pub landed: bool,
    pub fuel: f64,
    pub armor: f64,
    pub fire_cooldown: i32,
    pub takeoff_cooldown: i32,
    pub missile_cooldown: i32,
    pub missile_ammo: i32,
    pub respawn_timer: i32,
    pub cannon_heat: i32,
    pub cannon_jammed: i32,
    pub returning_to_carrier: bool,
    pub rotation_cooldown: i32,
}

#[derive(Debug, Clone)]
pub struct Island {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct Factory {
    pub x: f64,
    pub y: f64,
    pub health: i32,
    pub max_health: i32,
    pub active: bool,
    pub fire_cooldown: i32,
    pub sinking_timer: i32,
    pub drones_remaining: i32,
}

#[derive(Debug, Clone)]
pub struct Drone {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub active: bool,
    pub angle: f64,
    pub factory_idx: i32,
}

#[derive(Debug, Clone)]
pub struct Tank {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub health: i32,
    pub max_health: i32,
    pub active: bool,
    pub fire_cooldown: i32,
    pub sinking_timer: i32,
    pub patrol_dir: i32,
    pub min_coord: f64,
    pub max_coord: f64,
}

#[derive(Debug, Clone)]
pub struct StealthBoat {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct StaticAA {
    pub x: f64,
    pub y: f64,
    pub health: i32,
    pub max_health: i32,
    pub active: bool,
    pub fire_cooldown: i32,
    pub sinking_timer: i32,
}

// Lock-on target: mutually exclusive, recomputed each tick at step 15
#[derive(Debug, Clone, PartialEq)]
pub enum LockedTarget {
    None,
    Boat(usize),
    Factory(usize),
    Tank(usize),
    StaticAA(usize),
}
