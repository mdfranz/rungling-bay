# 🦀 Rungling Bay: Rust Implementation Reference Guide

This document is a hands-on learning resource and architectural analysis mapping the **Rungling Bay** codebase to the concepts found in the official [Rust by Example (RBE)](https://doc.rust-lang.org/rust-by-example/) curriculum. 

Whether you are learning Rust or reviewing this specific codebase, this guide highlights how core Rust features, APIs, and data structures are applied in a terminal-based action game.

---

## 🗺️ Codebase & Rust by Example Map

| Rust by Example Chapter | Key Feature | Application in Rungling Bay |
| :--- | :--- | :--- |
| **[2. Primitives](#2-primitives)** | Arrays, Slices, Tuples | Sprite constants, return patterns, non-blocking logs |
| **[3. Custom Types](#3-custom-types)** | Structs, Enums | Entity definitions, Algebraic Data Types, thread messaging |
| **[5. Types](#5-types)** | Casting, Type Inference | Coordinate conversion, implicit collection typing |
| **[7. Expressions](#7-expressions)** | Expression-oriented syntax | `if/else` and `match` expressions returning values |
| **[8. Flow Control](#8-flow-control)** | Match guards, `if let`, `while let`, `matches!` | Input dispatch, channel event polling, input filtering |
| **[9. Functions](#9-functions)** | Multiple `impl` blocks, Closures | Code modularity, UI painting, iterator combinators |
| **[10 & 12. Modules & Cargo](#10--12-modules-and-cargo)** | Module system, dependency management | Submodules, external crates (`ratatui`, `rodio`, `tracing`) |
| **[15. Scoping Rules](#15-scoping-rules)** | Ownership moves, Borrowing, Lifetimes | Thread spawning, solving borrow conflicts, elision |
| **[16. Traits](#16-traits)** | Manual traits, Derived traits | Custom Iterator, Rodio `Source`, Ratatui `Widget`, derivations |
| **[17. `macro_rules!`](#17-macro_rules)** | Declarative macros | Local target scanning macro to eliminate boilerplate |
| **[18. Error Handling](#18-error-handling)** | `?` operator, Option/Result combinators | Terminal initialization, safe channel reads, Option filter |
| **[19. Std Library Types](#19-std-library-types)** | `Vec`, `HashMap` | Entity management, joystick inputs, audio rate limiting |
| **[20. Std Misc](#20-std-misc)** | Threads, Channels, Time | Multithreaded key reader, synth mixer, delta-time ticks |
| **[21. Testing](#21-testing)** | `#[cfg(test)]`, `#[test]`, asserts | Unit tests for collision, lock-on targeting, and dynamics |

---

## 2. Primitives

*Rust by Example References: [Arrays and Slices](https://doc.rust-lang.org/rust-by-example/primitives/array.html), [Tuples](https://doc.rust-lang.org/rust-by-example/primitives/tuples.html)*

### Arrays and Slices
Rust arrays have a fixed size known at compile time, written as `[T; N]`. Slices are dynamically sized views into a contiguous sequence of elements, written as `[T]`.

In [src/game/types.rs](src/game/types.rs), fixed-size arrays and multi-dimensional arrays represent assets and physics constants:

```rust
// 1D array of string slices for directions
pub const DIR_NAMES: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

// 3D array of characters defining the 7x5 text sprites for 8 helicopter directions
pub const SPRITES: [[[char; 7]; 5]; 8] = [
    // 0: North
    [
        [' ', ' ', ' ', '▲', ' ', ' ', ' '],
        [' ', '*', '═', '#', '═', '*', ' '],
        [' ', ' ', ' ', '#', ' ', ' ', ' '],
        [' ', ' ', ' ', '+', ' ', ' ', ' '],
        [' ', ' ', ' ', ' ', ' ', ' ', ' '],
    ],
    // ...
];
```

### Tuples
Tuples are ordered collections of values of different types, written as `(T1, T2, ...)`. In [src/main.rs](src/main.rs), tuples are used to bind multiple variables returned by standard library operations:

```rust
// Destructuring a tuple returned by the tracing library
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

// Destructuring a tuple returned by the mpsc channel initialization
let (audio_tx, audio_rx) = mpsc::channel::<game::sound::SoundType>();
```

---

## 3. Custom Types

*Rust by Example References: [Structures](https://doc.rust-lang.org/rust-by-example/custom_types/structs.html), [Enums](https://doc.rust-lang.org/rust-by-example/custom_types/enum.html)*

### Structures
Rust supports C-like structs, tuple structs, and unit structs. In [src/game/types.rs](src/game/types.rs), structures group the properties of game entities:

```rust
#[derive(Debug, Clone)]
pub struct Helicopter {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub dir: usize,
    pub landed: bool,
    pub fuel: f64,
    pub armor: f64,
    pub missile_ammo: i32,
    // ...
}
```

### Enums (Algebraic Data Types)
Unlike C-style enums, Rust enums can contain payloads. These are known as *sum types* or *Algebraic Data Types (ADTs)*.

In [src/game/types.rs](src/game/types.rs), the lock-on targeting system is implemented with an enum that stores the index of the locked entity:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum LockedTarget {
    None,
    Boat(usize),
    Factory(usize),
    Tank(usize),
    StaticAA(usize),
}
```

In [src/game/input.rs](src/game/input.rs), the `InputMsg` enum wraps different types of keyboard or terminal events sent over a channel:

```rust
pub enum InputMsg {
    Key(KeyEvent),
    Resize(u16, u16),
}
```

---

## 5. Types

*Rust by Example References: [Casting](https://doc.rust-lang.org/rust-by-example/types/cast.html), [Inference](https://doc.rust-lang.org/rust-by-example/types/inference.html)*

### Casting with `as`
Rust requires explicit casting between scalar types; there is no implicit widening (e.g., `i32` → `i64`) or narrowing (e.g., `f64` → `i32`) conversion. This forces you to think about precision loss and overflow.

In [src/game/draw.rs](src/game/draw.rs), coordinates bridge two systems:
- **Physics engine:** uses `f64` for sub-pixel precision and smooth motion
- **Terminal rendering (ratatui):** uses `u16` (0–65535) for screen cell addresses

```rust
let sx = wx - self.cam_x;                    // f64: world-space X minus camera position
let play_h = area.height.saturating_sub(6) as i32;  // Clamp to 0, convert to i32

// Cast f64 to u16 for screen coordinates (truncates fractional part)
self.set_at(buf, area, sx as u16, sy as u16, ch, style);
```

**Key details:**
- `as u16` truncates, not rounds: `3.9 as u16` → `3`
- `saturating_sub(6)` prevents underflow by clamping to 0 instead of wrapping
- The cast is one-way: once you cast to screen coordinates, you lose sub-pixel precision; that's why physics uses `f64`

### Type Inference
Rust's compiler infers the type of variables by analyzing how they are used downstream. Rather than requiring explicit type annotations everywhere, the compiler works *backwards* from usage sites to determine what types must flow through.

In [src/main.rs](src/main.rs), a `HashMap` is instantiated without type parameters:

```rust
let mut last_play_time = std::collections::HashMap::new();
// At this point, Rust doesn't know the key or value types yet

last_play_time.insert(sound_type, now); 
// Compiler sees: sound_type is SoundType, now is Instant
// Therefore infers: HashMap<SoundType, Instant>
```

**How inference works:** The compiler looks at every operation on the variable and collects constraints:
- `.insert(sound_type, ...)` → key type is `SoundType`
- `.insert(..., now)` → value type is `Instant`

If you never use the variable, or use it ambiguously, type inference fails:

```rust
let mut data = HashMap::new();
// ERROR: can't infer types; nothing tells Rust what to store
```

Explicit type annotations resolve ambiguity:

```rust
let mut data: HashMap<String, i32> = HashMap::new();
```

---

## 7. Expressions

*Rust by Example Reference: [Expressions](https://doc.rust-lang.org/rust-by-example/expression.html)*

Rust is an expression-oriented language: almost every statement (including blocks, `if` statements, and `match` statements) can return a value. 

In [src/game/input.rs](src/game/input.rs), a conditional expression evaluates directly into a variable binding:

```rust
let thrust = if self.heli.fuel <= 0.0 { 0.0 } else { 0.18 };
```

In [src/main.rs](src/main.rs), a `match` expression resolves the sound characteristics:

```rust
let (dur, volume) = match sound_type {
    game::sound::SoundType::Warning => (Duration::from_millis(500), 0.20),
    game::sound::SoundType::Laser => (Duration::from_millis(55), 0.28),
    game::sound::SoundType::Missile => (Duration::from_millis(400), 0.20),
    game::sound::SoundType::Explosion => (Duration::from_millis(800), 0.38),
    game::sound::SoundType::Speedboat => (Duration::from_millis(300), 0.18),
};
```

---

## 8. Flow Control

*Rust by Example References: [if/else](https://doc.rust-lang.org/rust-by-example/flow_control/if_else.html), [loop](https://doc.rust-lang.org/rust-by-example/flow_control/loop.html), [match](https://doc.rust-lang.org/rust-by-example/flow_control/match.html), [if let](https://doc.rust-lang.org/rust-by-example/flow_control/if_let.html), [while let](https://doc.rust-lang.org/rust-by-example/flow_control/while_let.html)*

### Pattern Matching & Match Guards
`match` expressions check patterns exhaustively. A **match guard** (`_ if condition => ...`) adds an extra conditional check to a match arm, allowing one pattern to branch on multiple conditions.

In [src/game/input.rs](src/game/input.rs), the quit confirmation dialog uses guards to handle user input:

```rust
// Player pressed Ctrl+C; we're asking "Are you sure? (y/n)"
if self.quit_confirming {
    let ctrl_c = key.modifiers.contains(KeyModifiers::CONTROL)
        && key.code == KeyCode::Char('c');
    match key.code {
        // Confirm quit with Y
        KeyCode::Char('y') | KeyCode::Char('Y') => return false,
        
        // OR confirm quit with Ctrl+C again (match guard)
        // Matches any key code, but only proceeds if ctrl_c is true
        _ if ctrl_c => return false,
        
        // Cancel quit with N or Escape
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            self.quit_confirming = false;
        }
        
        // Ignore other keys
        _ => {}
    }
    return true;
}
```

**Why use a guard?** Without it, you'd need to check `ctrl_c` inside the match arm. With a guard, the control flow is cleaner: match on the *structure* (any key code), then filter by *logic* (is it Ctrl+C?).

### `while let` Loop
The `while let` construct is a cleaner alternative to matching repeatedly in a loop. In [src/main.rs](src/main.rs), it polls and drains all messages currently available on the input channel:

```rust
// Drain any additional buffered key events before executing the physics tick
while let Ok(msg) = rx.try_recv() {
    match msg {
        InputMsg::Key(k) => {
            if !game.handle_raw_key(&k) {
                running = false;
            }
        }
        InputMsg::Resize(w, h) => {
            game.width = w as i32;
            game.height = h as i32;
        }
    }
}
```

### `matches!` Macro
The `matches!` macro returns `true` if an expression matches a pattern. In [src/game/input.rs](src/game/input.rs), it is used to check key inputs concisely:

```rust
if matches!(key.code, KeyCode::Char('l') | KeyCode::Char('L'))
    && self.heli.takeoff_cooldown == 0
{
    // ...
}
```

---

## 9. Functions (Methods & Closures)

*Rust by Example References: [Methods](https://doc.rust-lang.org/rust-by-example/fn/methods.html), [Closures](https://doc.rust-lang.org/rust-by-example/fn/closures.html)*

### Multi-File Implementation Blocks
In Rust, you can define multiple `impl` blocks for the same struct across different files (as long as they're in the same module/crate). This is a powerful organizational tool: you can grow a type's functionality without cramming everything into one bloated file.

The `Game` struct has its state defined in [src/game/game.rs](src/game/game.rs), but its methods are distributed by responsibility:
- `impl Game` (Input Processing): [src/game/input.rs](src/game/input.rs)
- `impl Game` (Physics Engine): [src/game/physics.rs](src/game/physics.rs)
- `impl Game` (Drawing Engine): [src/game/draw.rs](src/game/draw.rs)

**Why?** A monolithic `impl Game` block would be thousands of lines—hard to navigate, hard to review, hard to maintain. By splitting across files, each file has a clear purpose:
- [input.rs](src/game/input.rs) handles keyboard and joystick events
- [physics.rs](src/game/physics.rs) handles entity updates, collisions, targeting
- [draw.rs](src/game/draw.rs) handles rendering to the terminal

When you modify rendering logic, you touch [draw.rs](src/game/draw.rs). When you fix a physics bug, you touch [physics.rs](src/game/physics.rs). The code organization matches the architectural separation.

The compiler treats all `impl` blocks as equivalent—the split is purely for human readability and maintainability.

### Closures
Closures are anonymous functions that can capture variables from their enclosing environment. 

In [src/main.rs](src/main.rs), `terminal.draw` accepts a closure that receives a terminal `Frame` and renders the game UI:

```rust
terminal.draw(|frame| {
    frame.render_widget(&game, frame.area());
})?;
```

Closures are also used in iterator combinators throughout the game logic to transform and filter collections. In [src/game/input.rs](src/game/input.rs), counting active player-controlled missiles uses a pipeline:

```rust
let active_count = self
    .missiles
    .iter()                                  // Iterate over all missiles
    .filter(|m| m.active && !m.is_enemy && !m.is_carrier)  // Keep only player missiles
    .count();                                // Count survivors
```

**How it works:**
1. `iter()` borrows each missile as `m`
2. The closure `|m| m.active && !m.is_enemy && !m.is_carrier` returns `true` to keep a missile, `false` to skip it
3. `.count()` consumes the filtered iterator and returns how many missiles passed the test

This is equivalent to (but more concise than):

```rust
let mut active_count = 0;
for m in &self.missiles {
    if m.active && !m.is_enemy && !m.is_carrier {
        active_count += 1;
    }
}
```

---

## 10 & 12. Modules and Cargo

*Rust by Example References: [Modules](https://doc.rust-lang.org/rust-by-example/mod.html), [Cargo](https://doc.rust-lang.org/rust-by-example/cargo.html)*

### Module Structure
Rungling Bay divides modules into files. [src/main.rs](src/main.rs) registers the root module `mod game;`, and [src/game/mod.rs](src/game/mod.rs) exposes its submodules to the program:

```rust
pub mod draw;
pub mod game;
pub mod input;
pub mod physics;
pub mod sound;
pub mod types;

pub use game::Game;
```

Within submodules, items are referenced using `super::` (referring to parent scope) or `crate::` (referring to module root):

```rust
use super::game::Game;
use super::types::{LockedTarget, DX, DY};
```

### Cargo Manifest
Third-party libraries are declared in [Cargo.toml](Cargo.toml):

```toml
[dependencies]
ratatui = "0.30"        # Terminal graphics UI framework
crossterm = "0.29"      # Cross-platform raw input and terminal management
rodio = "0.22"          # Thread-safe audio playback and synthesis
tracing = "0.1"         # High-performance structured logging framework
rand = "0.9"            # Random number generator
```

---

## 15. Scoping Rules (Ownership and Borrowing)

*Rust by Example References: [RAII](https://doc.rust-lang.org/rust-by-example/scope/raii.html), [Ownership & Moves](https://doc.rust-lang.org/rust-by-example/scope/move.html), [Borrowing & Mutability](https://doc.rust-lang.org/rust-by-example/scope/borrow.html), [Lifetime Elision](https://doc.rust-lang.org/rust-by-example/scope/lifetime/elision.html)*

### Ownership Transfer (Moves)
When an object is assigned to another variable or passed by value to a function, its ownership is transferred. The compiler ensures the original variable can no longer be read.

In [src/main.rs](src/main.rs), we spawn an audio thread using a `move` closure:

```rust
// Spawn audio playback thread and move the channel receiver `audio_rx` into it
thread::spawn(move || {
    // ...
    while let Ok(sound_type) = audio_rx.recv() {
        // ...
    }
});
```

By placing the `move` keyword before the closure arguments, ownership of `audio_rx` moves from the main thread into the spawned thread, guaranteeing memory safety across threads without a mutex.

### Borrow Checker Patterns (Preventing Alias Conflicts)
Rust guarantees memory safety by enforcing a rule: **either many immutable references (`&T`) OR exactly one mutable reference (`&mut T`)** to the same data at any time. This prevents data races and use-after-free bugs.

In [src/game/physics.rs](src/game/physics.rs), missile updates must search other entities (boats, tanks, factories) for targeting. The naive approach fails:

```rust
// ❌ Compiler error:
for m in &mut self.missiles {          // Borrow self.missiles mutably
    for b in &self.boats {             // ERROR: can't borrow self immutably
        // Trying to read boats while missiles borrow is active
    }
}
```

**The problem:** `self.missiles` and `self.boats` are both fields of `self`. When we mutably borrow `missiles`, the compiler blocks us from even *reading* `boats`, because `boats` is part of the same parent `self`. Rust is being conservative: "You mutated missiles; I can't let you read boats—you might have broken an invariant."

**The solution is the snapshot index pattern:** copy out the data you need, release the borrow, then read other collections:

```rust
// Loop over index bounds instead of borrowing the collection
for i in 0..self.missiles.len() {
    if !self.missiles[i].active { continue; }

    // Snapshot the specific missile state fields into local copy variables
    let mx = self.missiles[i].x;
    let my = self.missiles[i].y;
    let mvx = self.missiles[i].vx;
    let mvy = self.missiles[i].vy;
    let is_enemy_missile = self.missiles[i].is_enemy;
    let interception_rolled = self.missiles[i].interception_rolled;

    // Now, we can freely borrow other entity fields of self (&self.boats, &self.factories)
    // because we are not holding any borrow on `self.missiles`!
    let (new_vx, new_vy, new_ir) = if is_enemy_missile {
        // ...
    } else {
        // Safe to read other collections
        for b in &self.boats { ... }
        // ...
    };

    // Re-acquire reference at the very end to write calculations back
    self.missiles[i].vx = new_vx;
    self.missiles[i].vy = new_vy;
    self.missiles[i].interception_rolled = new_ir;
}
```

### Lifetime Elision
Every reference in Rust has an invisible "lifetime" — a label that specifies how long the reference is valid. Without lifetimes, the compiler cannot guarantee you won't use a reference after the data it points to has been deallocated. Fortunately, Rust infers lifetimes automatically in most cases using simple rules.

In [src/game/input.rs](src/game/input.rs), the signature below elides lifetimes:

```rust
pub fn handle_raw_key(&mut self, key: &KeyEvent) -> bool
```

The **explicit** form (with lifetimes named) would be:

```rust
pub fn handle_raw_key<'a>(&'a mut self, key: &'a KeyEvent) -> bool
```

Rust automatically applies the rule: **"If there's exactly one input reference lifetime, it must be the lifetime of all output references."** Since both `self` and `key` share the same lifetime `'a`, and there are no output references, the names are omitted.

**When elision fails:** If a function takes multiple input lifetimes and returns a reference, you must name them explicitly so Rust knows which input the output references:

```rust
// ❌ Ambiguous — which lifetime does the returned &str reference?
fn get_name(a: &Boat, b: &Tank) -> &str { ... }

// ✅ Explicit — the returned &str lives as long as the Boat
fn get_name<'a>(a: &'a Boat, b: &Tank) -> &'a str { ... }
```

---

## 16. Traits

*Rust by Example References: [Derive](https://doc.rust-lang.org/rust-by-example/trait/derive.html), [Iterator](https://doc.rust-lang.org/rust-by-example/trait/iter.html), [Traits](https://doc.rust-lang.org/rust-by-example/trait.html)*

### Deriving Traits
Common traits can be automatically implemented for types using the `#[derive(...)]` attribute. In [src/game/types.rs](src/game/types.rs), structures derive basic capabilities:

```rust
#[derive(Debug, Clone)]
pub struct Carrier {
    pub x: i32,
    pub y: i32,
    pub health: f64,
    // ...
}
```

- `Debug` allows formatting with `{:?}` in tracing logs.
- `Clone` permits making copies of carriers using `.clone()`.

### Manual Trait Implementation
For complex logic, traits are implemented manually.

#### 1. `Iterator` (Standard Library)
An iterator must define an associated type `Item` and implement the `next()` method, which returns `Some(item)` on each call and `None` when exhausted. In [src/game/sound.rs](src/game/sound.rs), `SynthSound` generates audio waveforms procedurally, one sample (f32 float) at a time:

```rust
impl Iterator for SynthSound {
    type Item = f32; // Each call to next() yields one audio sample

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_sample >= self.total_samples {
            return None; // Iterator exhausted; stop calling next()
        }
        
        let progress = self.current_sample as f32 / self.total_samples as f32;
        let mut val;

        // Synthesize the next sample based on sound type
        match self.sound_type {
            SoundType::Laser => {
                // Random noise source (using linear congruential generator)
                let noise = self.lcg.next_f32();
                
                // Frequency sweep: 320 Hz at start → 60 Hz at end
                let freq = 320.0 - 260.0 * progress;
                
                // Sinusoidal oscillator at target frequency
                let tone = (2.0 * PI * freq * self.time).sin() * 0.4;
                
                // Mix noise and tone
                val = 0.6 * noise + tone;
                
                // Exponential decay envelope: loud at start, silent by end
                val *= (-18.0 * progress).exp();
            }
            // ... other sound types (Explosion, Missile, etc.)
        }

        // Advance time by one sample duration (inverse of sample rate)
        self.time += 1.0 / self.sample_rate as f32;
        self.current_sample += 1;
        
        // Return the sample, scaled by volume
        Some(val * self.volume)
    }
}
```

**How it works:** The audio mixer calls `next()` thousands of times per second. Each call:
1. Computes how far through the sound we are (`progress`)
2. Synthesizes the waveform (mix of noise, oscillators, envelopes)
3. Increments the time counter
4. Returns the next audio sample

This procedural approach generates sound on-the-fly rather than storing pre-computed audio files, keeping the binary small.

#### 2. `Source` (Rodio Crate)
To play the sound, `SynthSound` implements `rodio::Source` to tell the mixer how to play the iterator:

```rust
impl Source for SynthSound {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> std::num::NonZeroU16 { std::num::NonZeroU16::new(1).unwrap() }
    fn sample_rate(&self) -> std::num::NonZeroU32 { std::num::NonZeroU32::new(self.sample_rate).unwrap() }
    fn total_duration(&self) -> Option<Duration> { Some(self.duration) }
}
```

#### 3. `Widget` (Ratatui Crate)
To draw the game, we implement `Widget` for `&Game` in [src/game/draw.rs](src/game/draw.rs):

```rust
impl Widget for &Game {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.draw(area, buf);
    }
}
```

---

## 17. `macro_rules!` (Macros)

*Rust by Example Reference: [macro_rules!](https://doc.rust-lang.org/rust-by-example/macros.html)*

Declarative macros (`macro_rules!`) write code that writes other code. They eliminate duplication when the same logic applies to different types, all while maintaining compile-time type-safety.

In [src/game/input.rs](src/game/input.rs), the lock-on targeting system scans four different entity collections: `boats`, `factories`, `tanks`, and `static_aas`. Each has similar fields (`x`, `y`, `active`, `sinking_timer`) but is a different struct. Without a macro, the search loop would be copy-pasted four times.

Instead, a **local macro** parameterizes the differences:

```rust
pub fn get_locked_target(&self) -> LockedTarget {
    // ... setup: helicopter position, facing direction, etc.
    let mut locked = LockedTarget::None;
    let mut min_dist = f64::MAX;

    // Local macro: defines one parametric search loop
    macro_rules! check_target {
        // Macro signature: names for the parameters we'll substitute
        ($slice:expr, $variant:ident, $x:ident, $y:ident, $active:ident, $sink:ident) => {
            // Macro body: the code to repeat
            for (i, t) in $slice.iter().enumerate() {
                if !t.$active || t.$sink > 0 { continue; }
                let ddx = t.$x - self.heli.x;
                let ddy = (t.$y - self.heli.y) * 2.0;
                let dist = (ddx * ddx + ddy * ddy).sqrt();
                if dist <= 0.0 || dist > MAX_LOCK_ON_RANGE { continue; }
                let dot = fwd_x * (ddx / dist) + fwd_y * (ddy / dist);
                if dot >= 0.707 && dist < min_dist {
                    min_dist = dist;
                    locked = LockedTarget::$variant(i);  // Use variant parameter
                }
            }
        };
    }

    // Call the macro four times with different entity types
    check_target!(self.boats,      Boat,      x, y, active, sinking_timer);
    check_target!(self.factories,  Factory,   x, y, active, sinking_timer);
    check_target!(self.tanks,      Tank,      x, y, active, sinking_timer);
    check_target!(self.static_aas, StaticAA,  x, y, active, sinking_timer);

    locked
}
```

**Macro parameters explained:**
- `$slice:expr` — any Rust expression (e.g., `self.boats`) that evaluates to a collection
- `$variant:ident` — an identifier token (e.g., `Boat`) used to construct the enum variant
- `$x:ident`, `$y:ident`, `$active:ident`, `$sink:ident` — field accessor names that differ per entity (e.g., `Boat` has `x`, `y`, `active`, `sinking_timer`)

**Why macros?** Each entity type is different (they're separate structs), so you can't use a generic function. Macros let you "stamp out" the same logic for each type, substituting the field names and variant names. The result is compiled independently for each type, so there's no runtime cost—just compile-time expansion.

---

## 18. Error Handling

*Rust by Example References: [Unwrapping Options](https://doc.rust-lang.org/rust-by-example/error/option_unwrap.html), [Result](https://doc.rust-lang.org/rust-by-example/error/result.html), [? operator](https://doc.rust-lang.org/rust-by-example/error/result/enter_question_mark.html)*

### Error Propagation with `?`
The `?` operator unwraps a `Result` on success or **returns the error immediately** to the caller. This eliminates verbose error handling boilerplate.

In [src/main.rs](src/main.rs), terminal setup calls multiple operations that can fail. Each `?` short-circuits the function if an error occurs:

```rust
fn main() -> io::Result<()> {
    enable_raw_mode()?;                // If this fails, return the error immediately
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;  // Same here
    // ... rest of setup
    Ok(())
}
```

**Without `?`, you'd write:**

```rust
fn main() -> io::Result<()> {
    match enable_raw_mode() {
        Ok(_) => {},
        Err(e) => return Err(e),       // Boilerplate: handle error, re-wrap, return
    }
    
    let mut stdout = io::stdout();
    match execute!(stdout, EnterAlternateScreen) {
        Ok(_) => {},
        Err(e) => return Err(e),       // Same pattern repeated
    }
    // ...
    Ok(())
}
```

The `?` operator eliminates this repetition: one character instead of five lines per operation.

### Option Combinators
Rust provides methods like `is_some_and`, `filter`, and `map` on the `Option` type to perform logic without manual pattern matching.

In [src/game/physics.rs](src/game/physics.rs), a target coordinate option is filtered based on flag checks:

```rust
if let Some((bx, by)) = ciws_pos.filter(|_| !interception_rolled && min_dist < BOAT_DETECTION_RANGE) {
    new_ir = true;
    // ...
}
```

---

## 19. Std Library Types

*Rust by Example References: [Vectors](https://doc.rust-lang.org/rust-by-example/std/vec.html), [HashMaps](https://doc.rust-lang.org/rust-by-example/std/hash.html)*

### Vectors
Vectors are growable, heap-allocated arrays (`Vec<T>`). In [src/game/physics.rs](src/game/physics.rs), they are cleared, modified, and pruned. The `retain_mut` method updates matching elements in-place and removes dead ones:

```rust
// Age and filter out explosions older than 15 frames
fn update_explosions(&mut self) {
    self.explosions.retain_mut(|e| {
        e.age += 1;
        e.age < 15
    });
}
```

### HashMaps
HashMaps store key-value pairs (`HashMap<K, V>`). In [src/game/game.rs](src/game/game.rs), maps track joystick states:

```rust
pub struct Game {
    pub joystick_axes: HashMap<u8, i16>,
    pub joystick_buttons: HashMap<u8, bool>,
    // ...
}
```

---

## 20. Std Misc (Concurrency)

*Rust by Example References: [Threads](https://doc.rust-lang.org/rust-by-example/std_misc/threads.html), [Channels](https://doc.rust-lang.org/rust-by-example/std_misc/channels.html)*

### Threading and Message Channels
Rungling Bay decouples input polling and sound mixing from the main graphics/physics game loop using background OS threads spawned via `std::thread::spawn` and coordinated using `std::sync::mpsc::channel`.

```
                    ┌─────────────────────────┐
                    │  Keyboard Event Thread  │
                    └────────────┬────────────┘
                                 │
                            Key / Resize
                                 ▼
┌──────────────────┐    InputMsg Channel     ┌──────────────────┐
│   Main Thread    ├─────────────────────────►    Main Thread    │
│  (Window Loop)   │                         │   (Game Tick)    │
└──────────────────┘                         └──────────┬───────┘
                                                        │
                                                    SoundType
                                                        ▼
                                             ┌──────────────────┐
                                             │   Audio Stream   │
                                             │  Mixer Thread    │
                                             └──────────────────┘
```

1. **Input Thread**: Crossterm blocks waiting for keyboard presses. To prevent this blocking from pausing physics, events are read on a separate thread and queued via a channel sender (`tx`):
   ```rust
   let (tx, rx) = mpsc::channel::<InputMsg>();
   thread::spawn(move || {
       loop {
           match event::read() {
               Ok(Event::Key(key)) => { tx.send(InputMsg::Key(key)).unwrap(); }
               Ok(Event::Resize(w, h)) => { tx.send(InputMsg::Resize(w, h)).unwrap(); }
               _ => {}
           }
       }
   });
   ```

2. **Audio Thread**: Audio synthesis math is computationally intensive. Sounds are pushed onto a queue channel sender (`audio_tx`), keeping the main game loop running at a steady 25 frames per second:
   ```rust
   while let Ok(sound_type) = audio_rx.recv() {
       let source = game::sound::SynthSound::new(44100, dur, sound_type, volume);
       let player = rodio::Player::connect_new(handle.mixer());
       player.append(source);
       player.detach();
   }
   ```

---

## 21. Testing

*Rust by Example Reference: [Unit Testing](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html)*

Rust has first-class support for unit testing. Test blocks are marked with `#[cfg(test)]` (compiled only during tests) and individual tests with `#[test]`.

At the end of [src/game/physics.rs](src/game/physics.rs), unit tests validate critical math operations:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::types::*;

    #[test]
    fn test_aabb_collision() {
        assert!(aabb(10.0, 10.0, 10.5, 10.5, 1.0, 1.0));
        assert!(!aabb(10.0, 10.0, 12.0, 12.0, 1.0, 1.0));
    }

    #[test]
    fn test_get_locked_target() {
        let mut game = Game::new(80, 40, None);

        // Put heli at (10.0, 10.0) facing East (dir = 2)
        game.heli.x = 10.0;
        game.heli.y = 10.0;
        game.heli.dir = 2; // East

        game.boats.push(Boat {
            x: 20.0, // In front of heli
            y: 10.0,
            active: true,
            sinking_timer: 0,
            // ...
        });

        let locked = game.get_locked_target();
        assert_eq!(locked, LockedTarget::Boat(0));
    }
}
```
