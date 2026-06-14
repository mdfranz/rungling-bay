# 🦀 Rungling Bay

**Rungling Bay** is a Rust-based, terminal-based tactical helicopter simulation game. It is a direct port of the original Go implementation, **[Gobungle](https://github.com/mdfranz/gobungle)**.

The game features momentum-based flight physics, custom procedural C64-style software audio synthesis, collision detection, and a retro Terminal User Interface (TUI) built using [Ratatui](https://crates.io/crates/ratatui) and [Crossterm](https://crates.io/crates/crossterm).

---

## 🎮 Gameplay & Design

For details on the background, gameplay mechanics, systems design, and lore of the game, please refer to the precursor Go repository:
👉 **[mdfranz/gobungle](https://github.com/mdfranz/gobungle)**

---

## 📚 Documentation

Detailed documentation on the Rust port's architecture and dependencies is available:
*   **[ARCHITECTURE.md](./ARCHITECTURE.md)** — Explains the thread design, gameplay loops, data safety, and structural refactorings.
*   **[PKG.md](./PKG.md)** — Details the direct and transitive third-party crates, audit classifications, and system dependencies.

---

## 🛠️ Build Instructions

To build and run **Rungling Bay**, you will need the Rust toolchain installed (compatible with the Rust 2024 edition).

### 1. System Dependencies

Since the game uses [Rodio](https://crates.io/crates/rodio) for real-time procedural audio synthesis, you will need the ALSA development libraries installed on Linux systems:

**Debian/Ubuntu:**
```bash
sudo apt-get install libasound2-dev
```

**Fedora/RHEL/CentOS:**
```bash
sudo dnf install alsa-lib-devel
```

### 2. Building the Project

Build the game in release mode for optimal performance, smooth frame rate rendering, and physical updates:

```bash
cargo build --release
```

---

## 🚀 Play Instructions

Start the game using Cargo:

```bash
cargo run --release
```

### ⌨️ Keyboard Controls

Control your helicopter using the following keys:

| Action | Key / Controls | Description |
| :--- | :--- | :--- |
| **Thrust** | `W` / `Up Arrow` | Applies forward momentum in the helicopter's current facing direction. |
| **Steer Left** | `A` / `Left Arrow` | Rotates the helicopter counter-clockwise (8-directional). |
| **Steer Right** | `D` / `Right Arrow` | Rotates the helicopter clockwise (8-directional). |
| **Brake / Hover** | `S` / `Down Arrow` | Dampens momentum drastically to slow down/hover. |
| **Fire Cannon** | `Spacebar` | Fires the main cannon. *Warning: Rapid firing heats the barrel; overheating causes it to jam and inflicts minor armor damage.* |
| **Fire Missile** | `F` / `M` | Launches a homing missile at a locked target (must have a target locked in a forward 45° aperture). |
| **Carrier Takeoff** | `Spacebar` / `W` / `Up Arrow` / `L` | Take off from the aircraft carrier deck when landed. |
| **Carrier Landing** | `L` | Manually land the helicopter on the carrier pad (must be aligned over the pad and traveling slower than `0.25` speed). |
| **Quit Game** | `Esc` / `Ctrl+C` | Prompts to exit the game. Confirm with `Y`/`N`. |

---

## 🔬 Running Tests

The game engine physics, AABB collisions, and targeting logic are covered by unit tests. You can run the test suite using Cargo:

```bash
cargo test
```

## 📋 Logging & Diagnostics

The game records detailed diagnostic logs to `./rungling-bay.log` during execution. If you experience issues (e.g., sound failure, unexpected physics, joystick events), check this file for details.
