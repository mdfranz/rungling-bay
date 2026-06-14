use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Widget,
};

use super::game::Game;
use super::types::{LockedTarget, ROTOR_FRAMES, SPRITES};

// tcell named-color equivalents
const NAVY: Color = Color::Rgb(0, 0, 128);
const DODGER_BLUE: Color = Color::Rgb(30, 144, 255);
const ORANGE: Color = Color::Rgb(255, 165, 0);
const KHAKI: Color = Color::Rgb(240, 230, 140);
const OLIVE: Color = Color::Rgb(128, 128, 0);
const DARK_GREEN: Color = Color::Rgb(0, 100, 0);
const LIME_GREEN: Color = Color::Rgb(50, 205, 50);
const DIM_GRAY: Color = Color::Rgb(105, 105, 105);
const LIGHT_GREY: Color = Color::Rgb(211, 211, 211);
const DARK_CYAN_C: Color = Color::Rgb(0, 139, 139);
const SILVER: Color = Color::Rgb(192, 192, 192);
const STEEL_BLUE: Color = Color::Rgb(70, 130, 180);
const PALE_TURQUOISE: Color = Color::Rgb(175, 238, 238);
const SLATE_GRAY: Color = Color::Rgb(112, 128, 144);
const DARK_RED: Color = Color::Rgb(139, 0, 0);
const STEALTH_GRAY: Color = Color::Rgb(160, 160, 160);
const STEALTH_WAKE: Color = Color::Rgb(100, 100, 100);

// ---------------------------------------------------------------------------
// Widget impl — entry point called by Terminal::draw
// ---------------------------------------------------------------------------

impl Widget for &Game {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.draw(area, buf);
    }
}

// ---------------------------------------------------------------------------
// Draw helpers
// ---------------------------------------------------------------------------

impl Game {
    /// Write a single char to an absolute screen position (clamped to area).
    fn set_at(&self, buf: &mut Buffer, area: Rect, sx: u16, sy: u16, ch: char, style: Style) {
        let x = area.left() + sx;
        let y = area.top() + sy;
        if x < area.right() && y < area.bottom() {
            buf[(x, y)].set_char(ch).set_style(style);
        }
    }

    /// Draw a string at screen position (sx, sy).
    fn draw_string(&self, buf: &mut Buffer, area: Rect, sx: i32, sy: i32, s: &str, style: Style) {
        for (i, ch) in s.chars().enumerate() {
            let x = sx + i as i32;
            if x >= 0 && sy >= 0 {
                self.set_at(buf, area, x as u16, sy as u16, ch, style);
            }
        }
    }

    /// Draw a char at world coords (wx, wy), camera-translated, with fg color over terrain bg.
    fn draw_cell(&self, buf: &mut Buffer, area: Rect, wx: i32, wy: i32, ch: char, fg: Color) {
        let sx = wx - self.cam_x;
        let sy = wy - self.cam_y;
        let play_h = area.height.saturating_sub(6) as i32;
        if sx < 0 || sx >= self.width || sy < 0 || sy >= play_h {
            return;
        }
        let style = self.get_map_style(wx, wy).fg(fg);
        self.set_at(buf, area, sx as u16, sy as u16, ch, style);
    }

    /// Returns background+foreground Style for a world cell (ocean / coast / road / carrier).
    pub fn get_map_style(&self, x: i32, y: i32) -> Style {
        let threshold = if self.island.active { Some(self.get_coastline_threshold(y)) } else { None };
        self.get_map_style_with_threshold(x, y, threshold)
    }

    /// Same as `get_map_style` but accepts a precomputed coastline threshold to avoid redundant trig.
    pub fn get_map_style_with_threshold(&self, x: i32, y: i32, threshold: Option<f64>) -> Style {
        // Carrier deck
        if x >= self.carrier.x
            && x < self.carrier.x + self.carrier.width
            && y >= self.carrier.y
            && y < self.carrier.y + self.carrier.height
        {
            let cy = y - self.carrier.y;
            let cx = x - self.carrier.x;
            let left_taper = (cy == 0 || cy == self.carrier.height - 1) && cx < 4;
            let right_taper =
                (cy == 0 || cy == self.carrier.height - 1) && cx >= self.carrier.width - 4;
            if !left_taper && !right_taper {
                return Style::new().bg(Color::DarkGray).fg(Color::White);
            }
        }

        // Coastline / terrain
        let xf = x as f64;
        let (is_land, is_sand) = match threshold {
            Some(t) => (xf >= t, xf >= t && xf < t + 3.0),
            None => (false, false),
        };
        if is_land {
            if !is_sand && self.is_road(x, y) {
                return Style::new().bg(DIM_GRAY);
            }
            if is_sand {
                return Style::new().bg(OLIVE).fg(KHAKI);
            }
            return Style::new().bg(DARK_GREEN).fg(LIME_GREEN);
        }

        // Ocean wave accent
        let is_wave = (x * 9 + y * 13) % 23 == 0;
        if is_wave {
            Style::new().bg(NAVY).fg(DODGER_BLUE)
        } else {
            Style::new().bg(NAVY).fg(NAVY)
        }
    }
}

// ---------------------------------------------------------------------------
// Main draw entry
// ---------------------------------------------------------------------------

impl Game {
    pub fn draw(&self, area: Rect, buf: &mut Buffer) {
        let cam_x = self.cam_x;
        let cam_y = self.cam_y;
        let play_h = area.height.saturating_sub(6) as i32;
        let w = area.width as i32;

        // 1. Background: ocean / terrain / carrier
        // Precompute coastline threshold per world-row to avoid redundant sin/cos per cell.
        let coast_thresholds: Vec<f64> = (0..play_h)
            .map(|sy| self.get_coastline_threshold(sy + cam_y))
            .collect();

        for sy in 0..play_h {
            let vy = sy + cam_y;
            let threshold_opt = if self.island.active { Some(coast_thresholds[sy as usize]) } else { None };
            let threshold = coast_thresholds[sy as usize];
            for sx in 0..w {
                let vx = sx + cam_x;
                let mut style = self.get_map_style_with_threshold(vx, vy, threshold_opt);
                let mut r = ' ';

                if vx >= self.carrier.x
                    && vx < self.carrier.x + self.carrier.width
                    && vy >= self.carrier.y
                    && vy < self.carrier.y + self.carrier.height
                {
                    let cy = vy - self.carrier.y;
                    let cx = vx - self.carrier.x;
                    let left_taper = (cy == 0 || cy == self.carrier.height - 1) && cx < 4;
                    let right_taper = (cy == 0 || cy == self.carrier.height - 1)
                        && cx >= self.carrier.width - 4;

                    if !left_taper && !right_taper {
                        if self.carrier_destroying && rand::random::<u8>() % 100 < 40 {
                            if rand::random::<bool>() {
                                r = '█';
                                style = Style::new().bg(Color::Red).fg(ORANGE);
                            } else {
                                r = '░';
                                style = Style::new().bg(ORANGE).fg(Color::Yellow);
                            }
                        } else {
                            // Runway stripe
                            if cy == self.carrier.height / 2
                                && cx > 3
                                && cx < self.carrier.width - 3
                                && cx % 3 != 0
                            {
                                r = '-';
                                style = style.fg(Color::Yellow);
                            }
                            // Landing pad & H marker
                            let pad_x = self.carrier.width / 3;
                            let pad_y = self.carrier.height / 2;
                            if cx >= pad_x - 2 && cx <= pad_x + 2 && cy >= pad_y - 1 && cy <= pad_y + 1 {
                                style = style.fg(Color::Yellow);
                                if cx == pad_x && cy == pad_y {
                                    r = 'H';
                                } else if cx == pad_x - 2 || cx == pad_x + 2 {
                                    r = '|';
                                } else if cy == pad_y - 1 {
                                    r = '¯';
                                } else if cy == pad_y + 1 {
                                    r = '_';
                                }
                            }
                        }
                    }
                } else {
                    let vxf = vx as f64;
                    let is_land = vxf >= threshold;
                    if is_land {
                        let is_sand = vxf < threshold + 3.0;
                        if !is_sand && self.is_road(vx, vy) {
                            let w_val = self.world_width;
                            let h_val = self.world_height;
                            let is_vert = vx == w_val - 15;
                            let is_horiz = vy == h_val / 2;
                            if (is_vert && vy >= h_val / 6 && vy <= h_val * 5 / 6 && vy % 2 == 0)
                                || (is_horiz && vx >= w_val - 15 && vx <= w_val - 8 && vx % 2 == 0)
                            {
                                r = if is_vert { '|' } else { '-' };
                                style = Style::new().bg(DIM_GRAY).fg(Color::Yellow);
                            } else {
                                let hash = (vx * 13 + vy * 17) % 6;
                                if hash == 0 {
                                    r = '.';
                                    style = Style::new().bg(DIM_GRAY).fg(LIGHT_GREY);
                                } else {
                                    style = Style::new().bg(DIM_GRAY).fg(DIM_GRAY);
                                }
                            }
                        } else if is_sand {
                            let hash = (vx * 17 + vy * 13) % 4;
                            if hash == 0 { r = '.'; }
                        } else {
                            let hash = (vx * 7 + vy * 11) % 5;
                            r = if hash == 0 { ',' } else if hash == 1 { '`' } else { ' ' };
                        }
                    } else if (vx * 9 + vy * 13) % 23 == 0 {
                        r = '~';
                    }
                }

                self.set_at(buf, area, sx as u16, sy as u16, r, style);
            }
        }

        // 1.2 Carrier smoke columns (damage-based)
        self.draw_carrier_smoke(buf, area);

        // A. Boats
        self.draw_boats(buf, area);

        // A.1b Stealth boats
        self.draw_stealth_boats(buf, area);

        // A.2 Factories
        self.draw_factories(buf, area);

        // A.3 Drones
        self.draw_drones(buf, area);

        // A.4 Tanks
        self.draw_tanks(buf, area);

        // A.5 Static AA
        self.draw_static_aa(buf, area);

        // B. Bullets
        self.draw_bullets(buf, area);

        // B.5 Missiles
        self.draw_missiles(buf, area);

        // C. Explosions
        self.draw_explosions(buf, area);

        // 2. Helicopter sprite
        self.draw_heli(buf, area);

        // 3. HUD
        let hud_y = play_h as u16;
        self.draw_hud(buf, area, hud_y);

        if self.quit_confirming {
            self.draw_quit_confirmation(buf, area);
        }
        if self.game_over {
            self.draw_game_over(buf, area);
        }
    }
}

// ---------------------------------------------------------------------------
// Entity draw methods
// ---------------------------------------------------------------------------

impl Game {
    fn draw_carrier_smoke(&self, buf: &mut Buffer, area: Rect) {
        if self.carrier.health >= 100.0 {
            return;
        }
        let damage_pct = 100.0 - self.carrier.health;
        let sources: [(i32, i32); 12] = [
            (4, 2), (18, 1), (10, 3), (22, 4), (7, 1), (14, 4),
            (2, 3), (20, 3), (12, 1), (16, 2), (5, 4), (24, 2),
        ];
        let num_columns = ((damage_pct / 8.0) as usize).clamp(1, 12);

        let max_height = 5 + (damage_pct * 0.12) as i32;
        let speed_div = if damage_pct >= 75.0 { 1 } else if damage_pct >= 40.0 { 2 } else { 3 };
        let fire_height = if damage_pct >= 20.0 { (damage_pct / 25.0) as i32 } else { 0 };
        let play_h = area.height.saturating_sub(6) as i32;

        for (col, &(sdx, sdy)) in sources.iter().enumerate().take(num_columns) {
            let base_sx = self.carrier.x + sdx;
            let base_sy = self.carrier.y + sdy;
            let col_osc = ((self.ticks as f64 / 6.0 + col as f64).sin() * 1.5) as i32;
            let col_height = (max_height + col_osc).max(3);

            for h in 0..col_height {
                let wiggle = ((self.ticks as f64 / 5.0 + h as f64).sin() * 0.6) as i32;
                let sm_x = base_sx + h / 2 + wiggle;
                let sm_y = base_sy - h;
                let ss_x = sm_x - self.cam_x;
                let ss_y = sm_y - self.cam_y;
                if ss_x < 0 || ss_x >= self.width || ss_y < 0 || ss_y >= play_h {
                    continue;
                }
                let phase = ((self.ticks / speed_div - h) % 4 + 4) % 4;
                let draw_particle = if damage_pct < 30.0 {
                    phase == 0
                } else if damage_pct < 70.0 {
                    phase == 0 || phase == 1
                } else {
                    phase <= 2
                };
                if !draw_particle { continue; }

                let bg_style = self.get_map_style(sm_x, sm_y);
                let (r, fg) = if h < fire_height {
                    let flicker = (self.ticks + h + col as i32) % 3;
                    if flicker == 0 { ('▲', Color::Red) }
                    else if flicker == 1 { ('☼', ORANGE) }
                    else { ('▲', Color::Yellow) }
                } else if h < fire_height + 3 {
                    ('█', Color::DarkGray)
                } else if h < fire_height + 7 {
                    ('▒', Color::DarkGray)
                } else {
                    ('░', Color::Gray)
                };
                self.set_at(buf, area, ss_x as u16, ss_y as u16, r, bg_style.fg(fg));
            }
        }
    }

    fn draw_boats(&self, buf: &mut Buffer, area: Rect) {
        let boat_color = SILVER;
        let flag_color = Color::Red;
        for boat in &self.boats {
            if !boat.active { continue; }
            let bx = boat.x.round() as i32;
            let by = boat.y.round() as i32;
            if boat.vx < 0.0 {
                self.draw_cell(buf, area, bx - 1, by - 1, '_', boat_color);
                self.draw_cell(buf, area, bx,     by - 1, '╨', boat_color);
                self.draw_cell(buf, area, bx + 1, by - 1, '_', boat_color);
                self.draw_cell(buf, area, bx - 3, by, '/', boat_color);
                self.draw_cell(buf, area, bx - 2, by, '█', boat_color);
                self.draw_cell(buf, area, bx - 1, by, '█', flag_color);
                self.draw_cell(buf, area, bx,     by, '█', boat_color);
                self.draw_cell(buf, area, bx + 1, by, '█', boat_color);
                self.draw_cell(buf, area, bx + 2, by, '\\', boat_color);
                self.draw_cell(buf, area, bx - 5, by + 1, '◄', boat_color);
                for i in -4..=4i32 { self.draw_cell(buf, area, bx + i, by + 1, '█', boat_color); }
                self.draw_cell(buf, area, bx + 5, by + 1, '═', boat_color);
            } else {
                self.draw_cell(buf, area, bx - 1, by - 1, '_', boat_color);
                self.draw_cell(buf, area, bx,     by - 1, '╨', boat_color);
                self.draw_cell(buf, area, bx + 1, by - 1, '_', boat_color);
                self.draw_cell(buf, area, bx - 2, by, '/', boat_color);
                self.draw_cell(buf, area, bx - 1, by, '█', boat_color);
                self.draw_cell(buf, area, bx,     by, '█', flag_color);
                self.draw_cell(buf, area, bx + 1, by, '█', boat_color);
                self.draw_cell(buf, area, bx + 2, by, '█', boat_color);
                self.draw_cell(buf, area, bx + 3, by, '\\', boat_color);
                self.draw_cell(buf, area, bx - 5, by + 1, '═', boat_color);
                for i in -4..=4i32 { self.draw_cell(buf, area, bx + i, by + 1, '█', boat_color); }
                self.draw_cell(buf, area, bx + 5, by + 1, '▶', boat_color);
            }
        }
    }

    fn draw_stealth_boats(&self, buf: &mut Buffer, area: Rect) {
        for sb in &self.stealth_boats {
            if !sb.active { continue; }
            let sx = sb.x.round() as i32;
            let sy = sb.y.round() as i32;
            self.draw_cell(buf, area, sx,     sy, '◄', STEALTH_GRAY);
            self.draw_cell(buf, area, sx + 1, sy, '▬', STEALTH_GRAY);
            self.draw_cell(buf, area, sx + 2, sy, '▬', STEALTH_GRAY);
            self.draw_cell(buf, area, sx + 3, sy, '▐', STEALTH_GRAY);
            self.draw_cell(buf, area, sx + 4, sy, '·', STEALTH_WAKE);
            self.draw_cell(buf, area, sx + 5, sy, '·', STEALTH_WAKE);
        }
    }

    fn draw_factories(&self, buf: &mut Buffer, area: Rect) {
        const FACTORY_SPRITE: [[char; 17]; 5] = [
            [' ','░','█','░',' ',' ',' ',' ','☼',' ',' ',' ',' ','░','█','░',' '],
            [' ','║','█','║',' ',' ','┌','─','┴','─','┐',' ',' ','║','█','║',' '],
            ['╓','─','╨','─','┴','─','┘',' ',' ',' ','└','─','┴','─','╨','─','╖'],
            ['║',' ','█',' ',' ','█',' ',' ','█',' ',' ','█',' ',' ','█',' ','║'],
            ['╙','─','─','─','─','─','[','▓','▓','▓',']','─','─','─','─','─','╜'],
        ];
        for (f_idx, fact) in self.factories.iter().enumerate() {
            if !fact.active { continue; }
            let fx = fact.x.round() as i32;
            let fy = fact.y.round() as i32;
            let is_destroying = fact.sinking_timer > 0;

            for (r, row) in FACTORY_SPRITE.iter().enumerate() {
                for (c, &sprite_ch) in row.iter().enumerate() {
                    let ch = sprite_ch;
                    if ch == ' ' { continue; }
                    let mx = fx + c as i32 - 8;
                    let my = fy + r as i32 - 2;
                    let fg = if is_destroying {
                        let flicker = (self.ticks + r as i32 + c as i32) % 3;
                        let (new_ch, fc) = if flicker == 0 { ('▲', Color::Red) }
                            else if flicker == 1 { ('☼', ORANGE) }
                            else { ('█', Color::DarkGray) };
                        self.draw_cell(buf, area, mx, my, new_ch, fc);
                        continue;
                    } else {
                        match ch {
                            '║' | '┌' | '─' | '┐' | '└' | '┘' | '┴' => SILVER,
                            '☼' => {
                                let phase_offset = f_idx as i32 * 4;
                                if ((self.ticks + phase_offset) / 8) % 2 == 0 { Color::Red } else { Color::Yellow }
                            }
                            '▓' => DARK_CYAN_C,
                            '╓' | '╖' | '╙' | '╜' => STEEL_BLUE,
                            '░' => Color::DarkGray,
                            _ => Color::Gray,
                        }
                    };
                    self.draw_cell(buf, area, mx, my, ch, fg);
                }
            }
            if !is_destroying {
                self.draw_factory_smoke(buf, area, fx - 6, fy - 2);
                self.draw_factory_smoke(buf, area, fx + 6, fy - 2);
            }
        }
    }

    fn draw_factory_smoke(&self, buf: &mut Buffer, area: Rect, sx: i32, sy: i32) {
        for h in 1..=5i32 {
            let wiggle = ((self.ticks as f64 / 5.0 + h as f64).sin() * 0.8) as i32;
            let sm_x = sx + h / 2 + wiggle;
            let sm_y = sy - h;
            if sm_x < 0 || sm_x >= self.world_width || sm_y < 0 || sm_y >= self.world_height {
                continue;
            }
            let phase = ((self.ticks / 3 - h) % 3 + 3) % 3;
            if phase > 1 { continue; }
            let (r, fg) = if h < 3 { ('█', Color::DarkGray) }
                else if h < 5 { ('▒', Color::DarkGray) }
                else { ('░', Color::Gray) };
            self.draw_cell(buf, area, sm_x, sm_y, r, fg);
        }
    }

    fn draw_drones(&self, buf: &mut Buffer, area: Rect) {
        for drone in &self.drones {
            if !drone.active { continue; }
            let dx = drone.x.round() as i32;
            let dy = drone.y.round() as i32;
            self.draw_cell(buf, area, dx, dy, '⌖', Color::LightCyan);
        }
    }

    fn draw_tanks(&self, buf: &mut Buffer, area: Rect) {
        for tank in &self.tanks {
            if !tank.active { continue; }
            let tx = tank.x.round() as i32;
            let ty = tank.y.round() as i32;
            let is_burning = tank.sinking_timer > 0;
            let color = if is_burning { DARK_RED } else { Color::Black };
            let tread_color = Color::DarkGray;
            let gun_color = if is_burning { Color::DarkGray } else { SILVER };
            let fire_color = ORANGE;

            if tank.patrol_dir == 0 {
                if tank.vy < 0.0 {
                    self.draw_cell(buf, area, tx - 1, ty - 1, '║', gun_color);
                    self.draw_cell(buf, area, tx + 1, ty - 1, '║', gun_color);
                    self.draw_cell(buf, area, tx - 2, ty, '▒', tread_color);
                    self.draw_cell(buf, area, tx - 1, ty, '(', color);
                    self.draw_cell(buf, area, tx,     ty, '▓', color);
                    self.draw_cell(buf, area, tx + 1, ty, ')', color);
                    self.draw_cell(buf, area, tx + 2, ty, '▒', tread_color);
                    self.draw_cell(buf, area, tx - 2, ty + 1, '▒', tread_color);
                    self.draw_cell(buf, area, tx,     ty + 1, '▄', color);
                    self.draw_cell(buf, area, tx + 2, ty + 1, '▒', tread_color);
                } else {
                    self.draw_cell(buf, area, tx - 2, ty - 1, '▒', tread_color);
                    self.draw_cell(buf, area, tx,     ty - 1, '▀', color);
                    self.draw_cell(buf, area, tx + 2, ty - 1, '▒', tread_color);
                    self.draw_cell(buf, area, tx - 2, ty, '▒', tread_color);
                    self.draw_cell(buf, area, tx - 1, ty, '(', color);
                    self.draw_cell(buf, area, tx,     ty, '▓', color);
                    self.draw_cell(buf, area, tx + 1, ty, ')', color);
                    self.draw_cell(buf, area, tx + 2, ty, '▒', tread_color);
                    self.draw_cell(buf, area, tx - 1, ty + 1, '║', gun_color);
                    self.draw_cell(buf, area, tx + 1, ty + 1, '║', gun_color);
                }
                if is_burning {
                    let flicker = (self.ticks / 3) % 2;
                    let (r, c) = if flicker == 0 { ('▲', Color::Red) } else { ('☼', fire_color) };
                    self.draw_cell(buf, area, tx, ty, r, c);
                }
            } else {
                for i in -2..=2i32 {
                    self.draw_cell(buf, area, tx + i, ty - 1, if i == -2 || i == 2 { '▄' } else { '▒' }, tread_color);
                    self.draw_cell(buf, area, tx + i, ty + 1, if i == -2 || i == 2 { '▀' } else { '▒' }, tread_color);
                }
                if tank.vx < 0.0 {
                    self.draw_cell(buf, area, tx - 2, ty, '═', gun_color);
                    self.draw_cell(buf, area, tx - 1, ty, '═', gun_color);
                    self.draw_cell(buf, area, tx,     ty, '▓', color);
                    self.draw_cell(buf, area, tx + 1, ty, '▒', color);
                    self.draw_cell(buf, area, tx + 2, ty, ']', color);
                } else {
                    self.draw_cell(buf, area, tx - 2, ty, '[', color);
                    self.draw_cell(buf, area, tx - 1, ty, '▒', color);
                    self.draw_cell(buf, area, tx,     ty, '▓', color);
                    self.draw_cell(buf, area, tx + 1, ty, '═', gun_color);
                    self.draw_cell(buf, area, tx + 2, ty, '═', gun_color);
                }
                if is_burning {
                    let flicker = (self.ticks / 3) % 2;
                    let (r, c) = if flicker == 0 { ('▲', Color::Red) } else { ('☼', fire_color) };
                    self.draw_cell(buf, area, tx, ty, r, c);
                }
            }
        }
    }

    fn draw_static_aa(&self, buf: &mut Buffer, area: Rect) {
        for aa in &self.static_aas {
            if !aa.active { continue; }
            let ax = aa.x.round() as i32;
            let ay = aa.y.round() as i32;
            let is_burning = aa.sinking_timer > 0;
            let gun_color = if is_burning { Color::DarkGray } else { SILVER };
            let base_color = if is_burning { DARK_RED } else { DARK_CYAN_C };
            let shield_color = Color::DarkGray;
            let center_color = if is_burning { ORANGE } else { Color::Red };
            let fire_color = ORANGE;

            self.draw_cell(buf, area, ax - 1, ay - 1, '║', gun_color);
            self.draw_cell(buf, area, ax + 1, ay - 1, '║', gun_color);
            self.draw_cell(buf, area, ax - 1, ay, '▕', shield_color);
            self.draw_cell(buf, area, ax,     ay, '╬', base_color);
            self.draw_cell(buf, area, ax + 1, ay, '▏', shield_color);
            if !is_burning && (self.ticks / 10) % 2 == 0 {
                self.draw_cell(buf, area, ax, ay, '☼', center_color);
            }
            self.draw_cell(buf, area, ax - 1, ay + 1, '▀', shield_color);
            self.draw_cell(buf, area, ax,     ay + 1, '█', shield_color);
            self.draw_cell(buf, area, ax + 1, ay + 1, '▀', shield_color);
            if is_burning {
                let flicker = (self.ticks / 3) % 2;
                let (r, c) = if flicker == 0 { ('▲', Color::Red) } else { ('☼', fire_color) };
                self.draw_cell(buf, area, ax, ay, r, c);
            }
        }
    }

    fn draw_bullets(&self, buf: &mut Buffer, area: Rect) {
        for bullet in &self.bullets {
            if !bullet.active { continue; }
            let bx = bullet.x.round() as i32;
            let by = bullet.y.round() as i32;
            let sx = bx - self.cam_x;
            let sy = by - self.cam_y;
            let play_h = area.height.saturating_sub(6) as i32;
            if sx >= 0 && sx < self.width && sy >= 0 && sy < play_h {
                let fg = if bullet.is_enemy { Color::Red } else { Color::Yellow };
                let style = self.get_map_style(bx, by).fg(fg);
                self.set_at(buf, area, sx as u16, sy as u16, '•', style);
            }
        }
    }

    fn draw_missiles(&self, buf: &mut Buffer, area: Rect) {
        for m in &self.missiles {
            if !m.active { continue; }
            let mx = m.x.round() as i32;
            let my = m.y.round() as i32;
            let sx = mx - self.cam_x;
            let sy = my - self.cam_y;
            let play_h = area.height.saturating_sub(6) as i32;
            if sx < 0 || sx >= self.width || sy < 0 || sy >= play_h { continue; }
            let ch = if m.vx.abs() > m.vy.abs() {
                if m.vx > 0.0 { '►' } else { '◄' }
            } else {
                if m.vy > 0.0 { '▼' } else { '▲' }
            };
            let fg = if m.is_enemy { Color::Red } else { ORANGE };
            let style = self.get_map_style(mx, my).fg(fg).add_modifier(Modifier::BOLD);
            self.set_at(buf, area, sx as u16, sy as u16, ch, style);
        }
    }

    fn draw_explosions(&self, buf: &mut Buffer, area: Rect) {
        for exp in &self.explosions {
            let ex = exp.x;
            let ey = exp.y;
            let sx = ex - self.cam_x;
            let sy = ey - self.cam_y;
            let play_h = area.height.saturating_sub(6) as i32;
            if sx < 0 || sx >= self.width || sy < 0 || sy >= play_h { continue; }
            let (r, fg) = if exp.age < 4 { ('*', Color::Yellow) }
                else if exp.age < 9 { ('¤', ORANGE) }
                else { ('·', Color::DarkGray) };
            let style = self.get_map_style(ex, ey).fg(fg);
            self.set_at(buf, area, sx as u16, sy as u16, r, style);
        }
    }

    fn draw_heli(&self, buf: &mut Buffer, area: Rect) {
        let h = &self.heli;
        let hx = h.x.round() as i32;
        let hy = h.y.round() as i32;
        let rotor_char = ROTOR_FRAMES[h.rotor_state % 4];
        let play_h = area.height.saturating_sub(6) as i32;

        for (r, row) in SPRITES[h.dir % 8].iter().enumerate() {
            for (c, &sprite_ch) in row.iter().enumerate() {
                let mut ch = sprite_ch;
                if ch == ' ' { continue; }
                let mx = hx + c as i32 - 3;
                let my = hy + r as i32 - 2;
                let sx = mx - self.cam_x;
                let sy = my - self.cam_y;
                if sx < 0 || sx >= self.width || sy < 0 || sy >= play_h { continue; }

                let is_rotor = ch == '*';
                if is_rotor { ch = rotor_char; }

                let fg = if is_rotor {
                    Color::White
                } else {
                    match ch {
                        '▲' | '▼' | '►' | '◄' => Color::White,
                        '|' | '/' | '\\' | '│' | '╪' => PALE_TURQUOISE,
                        '-' | '_' | '¯' | '[' | ']' | '=' | '═' | '─' | '║' => SILVER,
                        '█' | '▓' | '▒' | '╟' | '╢' => SLATE_GRAY,
                        _ => Color::White,
                    }
                };
                let style = self.get_map_style(mx, my).fg(fg);
                self.set_at(buf, area, sx as u16, sy as u16, ch, style);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// HUD
// ---------------------------------------------------------------------------

impl Game {
    fn draw_hud(&self, buf: &mut Buffer, area: Rect, hud_y: u16) {
        let hud_style = Style::new().bg(Color::Black).fg(Color::White);
        let border_style = Style::new().bg(Color::Black).fg(DARK_CYAN_C);

        // Top separator line
        for x in 0..area.width {
            self.set_at(buf, area, x, hud_y, '═', border_style);
        }
        let hud_title = format!(" 🚁 COCKPIT HUD PANEL (WAVE {}) 🚁 ", self.wave);
        self.draw_string(buf, area, 2, hud_y as i32, &hud_title, border_style.fg(Color::Yellow));

        // Incoming missile warning
        let has_incoming = self.missiles.iter().any(|m| m.active && m.is_enemy);
        if has_incoming && (self.heli.rotor_state / 2).is_multiple_of(2) {
            self.draw_string(buf, area, self.width - 33, hud_y as i32,
                "⚠️ WARNING: INCOMING MISSILE ⚠️",
                border_style.fg(Color::Red).add_modifier(Modifier::BOLD));
            // sound stub: play_sound("warning")
        }
        // Stealth threat
        if self.stealth_near && (self.ticks / 4) % 2 == 0 {
            self.draw_string(buf, area, self.width - 34, hud_y as i32 + 1,
                "⚠ STEALTH THREAT: CANNON ONLY ⚠",
                border_style.fg(STEALTH_GRAY).add_modifier(Modifier::BOLD));
        }

        // Clear lines hud_y+1..hud_y+5
        for dy in 1..=5u16 {
            for x in 0..area.width {
                self.set_at(buf, area, x, hud_y + dy, ' ', hud_style);
            }
        }

        // Speed / altitude
        let speed_knots = if !self.heli.landed {
            let v_mag = (self.heli.vx * self.heli.vx + (self.heli.vy * 2.0) * (self.heli.vy * 2.0)).sqrt();
            (v_mag * 450.0) as i32
        } else { 0 };
        let altitude_ft = if self.heli.landed { 0 } else { 150 };

        let pad_x = self.carrier.x + self.carrier.width / 3;
        let pad_y = self.carrier.y + self.carrier.height / 2;
        let hx = self.heli.x.round() as i32;
        let hy = self.heli.y.round() as i32;
        let aligned = hx >= pad_x - 1 && hx <= pad_x + 1 && hy >= pad_y - 1 && hy <= pad_y + 1;

        let (align_str, align_style) = if aligned {
            ("READY", hud_style.fg(Color::Green))
        } else {
            ("NO", hud_style.fg(Color::Red))
        };

        let (status_str, status_style) = if self.heli.landed {
            if self.heli.fuel < 100.0 {
                ("LANDED (REFUELING...)", Style::new().bg(Color::Yellow).fg(Color::Black))
            } else {
                ("LANDED (READY)", Style::new().bg(Color::Gray).fg(Color::White))
            }
        } else if self.heli.fuel <= 0.0 {
            ("OUT OF FUEL", Style::new().bg(Color::Red).fg(Color::White))
        } else {
            ("AIRBORNE", Style::new().bg(Color::Green).fg(Color::Black))
        };

        let fuel_color = if self.heli.fuel < 25.0 { Color::Red }
            else if self.heli.fuel < 50.0 { ORANGE }
            else { Color::Green };

        use super::types::{DIR_DEGREES, DIR_NAMES};
        let instrument_text = format!(
            "GPS:({},{}) | SPD:{:3}kt | HDG:{:3}°{:<2} | ALT:{:3}ft | FUEL:",
            hx, hy, speed_knots,
            DIR_DEGREES[self.heli.dir % 8], DIR_NAMES[self.heli.dir % 8], altitude_ft,
        );
        self.draw_string(buf, area, 2, hud_y as i32 + 1, &instrument_text, hud_style);

        let fuel_text = format!("{:3.1}%", self.heli.fuel);
        let fuel_off = 2 + instrument_text.len() as i32;
        self.draw_string(buf, area, fuel_off, hud_y as i32 + 1, &fuel_text, hud_style.fg(fuel_color));

        let ammo_label = " | MSL:";
        let ammo_off = fuel_off + fuel_text.len() as i32;
        self.draw_string(buf, area, ammo_off, hud_y as i32 + 1, ammo_label, hud_style);

        let ammo_color = if self.heli.missile_ammo == 0 { Color::Red }
            else if self.heli.missile_ammo <= 2 { ORANGE }
            else { Color::Green };
        let ammo_str: String = (0..4).map(|i| if i < self.heli.missile_ammo { "▲ " } else { "· " }).collect();
        let ammo_str_off = ammo_off + ammo_label.len() as i32;
        self.draw_string(buf, area, ammo_str_off, hud_y as i32 + 1, &ammo_str,
            hud_style.fg(ammo_color).add_modifier(Modifier::BOLD));

        let lives_label = " | LVS:";
        let lives_off = ammo_str_off + ammo_str.len() as i32;
        self.draw_string(buf, area, lives_off, hud_y as i32 + 1, lives_label, hud_style);
        let lives_str: String = (0..5).map(|i| if i < self.lives { "♥ " } else { "· " }).collect();
        let lives_color = if self.lives <= 2 { Color::Red } else if self.lives <= 3 { ORANGE } else { Color::Green };
        self.draw_string(buf, area, lives_off + lives_label.len() as i32, hud_y as i32 + 1,
            &lives_str, hud_style.fg(lives_color).add_modifier(Modifier::BOLD));

        // Row hud+2: status metrics
        let status_label = "FLIGHT STATUS: ";
        self.draw_string(buf, area, 2, hud_y as i32 + 2, status_label, hud_style);
        let status_padded = format!(" {} ", status_str);
        self.draw_string(buf, area, 2 + status_label.len() as i32, hud_y as i32 + 2, &status_padded, status_style);

        let mut off = 2 + status_label.len() as i32 + status_padded.len() as i32;
        off = self.draw_hud_stat(buf, area, off, hud_y as i32 + 2, " | ALIGN:", align_str, hud_style, align_style);

        let boats_rem = self.boats.iter().filter(|b| b.active).count();
        off = self.draw_hud_stat(buf, area, off, hud_y as i32 + 2, " | BOATS:",
            &boats_rem.to_string(), hud_style, hud_style.fg(Color::LightCyan));

        let factories_rem = self.factories.iter().filter(|f| f.active).count();
        off = self.draw_hud_stat(buf, area, off, hud_y as i32 + 2, " | FACTORIES:",
            &factories_rem.to_string(), hud_style, hud_style.fg(ORANGE));

        let lock_label = " | LOCK:";
        let (lock_str, lock_color) = match &self.locked {
            LockedTarget::None => ("NONE".to_string(), Color::Red),
            LockedTarget::Boat(_) => ("BOAT".to_string(), Color::Green),
            LockedTarget::Factory(idx) => {
                if let Some(fact) = self.factories.get(*idx) {
                    let active_drones = self.drones.iter()
                        .filter(|d| d.active && d.factory_idx == *idx as i32).count() as i32;
                    let total = active_drones + fact.drones_remaining;
                    let s = if total > 0 {
                        format!("FACTORY (DRONES: {}/10)", total)
                    } else {
                        "FACTORY (SHIELDS DOWN!)".to_string()
                    };
                    (s, Color::Green)
                } else { ("FACTORY".to_string(), Color::Green) }
            }
            LockedTarget::Tank(_) => ("TANK".to_string(), Color::Green),
            LockedTarget::StaticAA(_) => ("STATIC AA".to_string(), Color::Green),
        };
        self.draw_hud_stat(buf, area, off, hud_y as i32 + 2, lock_label, &lock_str,
            hud_style, hud_style.fg(lock_color).add_modifier(Modifier::BOLD));

        // Row hud+3: controls
        self.draw_string(buf, area, 2, hud_y as i32 + 3,
            "WASD=Fly | S=Brake | SPC=Cannon | F=Missile | L=Land",
            hud_style.fg(SILVER));

        // Row hud+4: bars
        let armor_color = if self.heli.armor < 25.0 { Color::Red }
            else if self.heli.armor < 50.0 { ORANGE } else { Color::Green };
        let armor_filled = (self.heli.armor.round() as i32) / 10;
        let armor_bar = format!("ARMOR:[{}{}]",
            "█".repeat(armor_filled.max(0) as usize),
            "░".repeat((10 - armor_filled).max(0) as usize));
        self.draw_string(buf, area, 2, hud_y as i32 + 4, &armor_bar,
            hud_style.fg(armor_color).add_modifier(Modifier::BOLD));

        let carrier_color = if self.carrier.health < 25.0 { Color::Red }
            else if self.carrier.health < 50.0 { ORANGE } else { Color::Green };
        let c_filled = (self.carrier.health.round() as i32) / 10;
        let carrier_bar = format!(" | CARRIER:[{}{}]",
            "█".repeat(c_filled.max(0) as usize),
            "░".repeat((10 - c_filled).max(0) as usize));
        self.draw_string(buf, area, 2 + armor_bar.len() as i32, hud_y as i32 + 4, &carrier_bar,
            hud_style.fg(carrier_color).add_modifier(Modifier::BOLD));

        const MAX_HEAT: i32 = 20;
        let heat_color = if self.heli.cannon_jammed > 0 || self.heli.cannon_heat >= 16 { Color::Red }
            else if self.heli.cannon_heat >= 10 { ORANGE } else { Color::Green };
        let heat_bar = if self.heli.cannon_jammed > 0 {
            " | CANNON:[JAMMED    ]".to_string()
        } else {
            let filled = self.heli.cannon_heat * 10 / MAX_HEAT;
            format!(" | CANNON:[{}{}]",
                "█".repeat(filled.max(0) as usize),
                "░".repeat((10 - filled).max(0) as usize))
        };
        self.draw_string(buf, area, 2 + armor_bar.len() as i32 + carrier_bar.len() as i32,
            hud_y as i32 + 4, &heat_bar, hud_style.fg(heat_color).add_modifier(Modifier::BOLD));

        self.draw_radar(buf, area, hud_y);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_hud_stat(&self, buf: &mut Buffer, area: Rect, x: i32, y: i32,
        label: &str, value: &str, label_style: Style, value_style: Style) -> i32
    {
        self.draw_string(buf, area, x, y, label, label_style);
        self.draw_string(buf, area, x + label.len() as i32, y, value, value_style);
        x + label.len() as i32 + value.len() as i32
    }

    fn draw_radar(&self, buf: &mut Buffer, area: Rect, hud_y: u16) {
        const RADAR_W: i32 = 23;
        const RADAR_H: i32 = 6;
        const RADAR_RANGE: f64 = 100.0;

        if self.width < 30 { return; }
        let rx = self.width - RADAR_W - 1;
        let ry = hud_y as i32;

        let border_style = Style::new().bg(Color::Black).fg(DARK_CYAN_C);

        // Top border
        self.set_at(buf, area, rx as u16, ry as u16, '╦', border_style);
        for dx in 1..RADAR_W - 1 {
            self.set_at(buf, area, (rx + dx) as u16, ry as u16, '═', border_style);
        }
        self.set_at(buf, area, (rx + RADAR_W - 1) as u16, ry as u16, '╗', border_style);
        let label = " RADAR ";
        self.draw_string(buf, area, rx + (RADAR_W - label.len() as i32) / 2, ry, label,
            border_style.fg(Color::Yellow));

        // Side borders
        for dy in 1..RADAR_H - 1 {
            self.set_at(buf, area, rx as u16, (ry + dy) as u16, '║', border_style);
            self.set_at(buf, area, (rx + RADAR_W - 1) as u16, (ry + dy) as u16, '║', border_style);
        }

        // Bottom border
        self.set_at(buf, area, rx as u16, (ry + RADAR_H - 1) as u16, '╚', border_style);
        for dx in 1..RADAR_W - 1 {
            self.set_at(buf, area, (rx + dx) as u16, (ry + RADAR_H - 1) as u16, '═', border_style);
        }
        self.set_at(buf, area, (rx + RADAR_W - 1) as u16, (ry + RADAR_H - 1) as u16, '╝', border_style);

        let int_w = RADAR_W - 2;
        let int_h = RADAR_H - 2;
        let cx = rx + 1 + int_w / 2;
        let cy = ry + 1 + int_h / 2;

        let mut plot = |wx: f64, wy: f64, sym: char, style: Style| {
            let ddx = wx - self.heli.x;
            let ddy = wy - self.heli.y;
            if ddx * ddx + ddy * ddy > RADAR_RANGE * RADAR_RANGE { return; }
            let px = cx + (ddx / RADAR_RANGE * (int_w / 2) as f64).round() as i32;
            let py = cy + (ddy / RADAR_RANGE * (int_h / 2) as f64).round() as i32;
            if px <= rx || px >= rx + RADAR_W - 1 || py <= ry || py >= ry + RADAR_H - 1 { return; }
            self.set_at(buf, area, px as u16, py as u16, sym, style);
        };

        let boat_style = Style::new().bg(Color::Black).fg(Color::LightCyan);
        let fact_style = Style::new().bg(Color::Black).fg(ORANGE);
        let aa_style = Style::new().bg(Color::Black).fg(Color::Red);
        let missile_style = Style::new().bg(Color::Black).fg(Color::Red).add_modifier(Modifier::BOLD);
        let player_style = Style::new().bg(Color::Black).fg(Color::Yellow).add_modifier(Modifier::BOLD);

        for b in &self.boats {
            if b.active { plot(b.x, b.y, '~', boat_style); }
        }
        for f in &self.factories {
            if f.active { plot(f.x, f.y, '■', fact_style); }
        }
        for aa in &self.static_aas {
            if aa.active { plot(aa.x, aa.y, '^', aa_style); }
        }
        for m in &self.missiles {
            if m.active && m.is_enemy && rand::random::<bool>() {
                plot(m.x, m.y, '!', missile_style);
            }
        }
        self.set_at(buf, area, cx as u16, cy as u16, '+', player_style);
    }
}

// ---------------------------------------------------------------------------
// Modals
// ---------------------------------------------------------------------------

impl Game {
    #[allow(clippy::too_many_arguments)]
    fn draw_modal_box(&self, buf: &mut Buffer, area: Rect, start_x: i32, start_y: i32,
        box_w: i32, box_h: i32, bg: Color)
    {
        let bg_style = Style::new().bg(bg).fg(Color::White);
        for r in 0..box_h {
            for c in 0..box_w {
                self.set_at(buf, area, (start_x + c) as u16, (start_y + r) as u16, ' ', bg_style);
            }
        }
        for c in 0..box_w {
            self.set_at(buf, area, (start_x + c) as u16, start_y as u16, '═', bg_style);
            self.set_at(buf, area, (start_x + c) as u16, (start_y + box_h - 1) as u16, '═', bg_style);
        }
        for r in 0..box_h {
            self.set_at(buf, area, start_x as u16, (start_y + r) as u16, '║', bg_style);
            self.set_at(buf, area, (start_x + box_w - 1) as u16, (start_y + r) as u16, '║', bg_style);
        }
        self.set_at(buf, area, start_x as u16, start_y as u16, '╔', bg_style);
        self.set_at(buf, area, (start_x + box_w - 1) as u16, start_y as u16, '╗', bg_style);
        self.set_at(buf, area, start_x as u16, (start_y + box_h - 1) as u16, '╚', bg_style);
        self.set_at(buf, area, (start_x + box_w - 1) as u16, (start_y + box_h - 1) as u16, '╝', bg_style);
    }

    fn draw_game_over(&self, buf: &mut Buffer, area: Rect) {
        const BOX_W: i32 = 46;
        const BOX_H: i32 = 9;
        let play_h = area.height.saturating_sub(6) as i32;
        let start_x = (self.width - BOX_W) / 2;
        let start_y = ((play_h - BOX_H) / 2).max(0);
        self.draw_modal_box(buf, area, start_x, start_y, BOX_W, BOX_H, DARK_RED);

        let border_style = Style::new().bg(DARK_RED).fg(Color::White);
        let title_style = Style::new().bg(DARK_RED).fg(Color::Yellow).add_modifier(Modifier::BOLD);

        let title = " ☠️  MISSION FAILURE  ☠️ ";
        self.draw_string(buf, area, start_x + (BOX_W - title.len() as i32) / 2, start_y + 1, title, title_style);

        let msg = "THE AIRCRAFT CARRIER HAS BEEN DESTROYED!";
        self.draw_string(buf, area, start_x + (BOX_W - msg.len() as i32) / 2, start_y + 3, msg, border_style);

        let stats = format!("You survived until Wave {}", self.wave);
        self.draw_string(buf, area, start_x + (BOX_W - stats.len() as i32) / 2, start_y + 5, &stats, title_style);

        let exit = "Press ANY KEY to exit the game";
        self.draw_string(buf, area, start_x + (BOX_W - exit.len() as i32) / 2, start_y + 7, exit, border_style);
    }

    fn draw_quit_confirmation(&self, buf: &mut Buffer, area: Rect) {
        const BOX_W: i32 = 42;
        const BOX_H: i32 = 7;
        let play_h = area.height.saturating_sub(6) as i32;
        let start_x = (self.width - BOX_W) / 2;
        let start_y = ((play_h - BOX_H) / 2).max(0);
        self.draw_modal_box(buf, area, start_x, start_y, BOX_W, BOX_H, DARK_RED);

        let border_style = Style::new().bg(DARK_RED).fg(Color::White);
        let title_style = Style::new().bg(DARK_RED).fg(Color::Yellow).add_modifier(Modifier::BOLD);

        let title = "  CONFIRM QUIT  ";
        self.draw_string(buf, area, start_x + (BOX_W - title.len() as i32) / 2, start_y + 1, title, title_style);

        let msg = "Are you sure you want to exit?";
        self.draw_string(buf, area, start_x + (BOX_W - msg.len() as i32) / 2, start_y + 3, msg, border_style);

        let opts = "[Y]es, Quit  |  [N]o, Resume";
        self.draw_string(buf, area, start_x + (BOX_W - opts.len() as i32) / 2, start_y + 5, opts, title_style);
    }
}
