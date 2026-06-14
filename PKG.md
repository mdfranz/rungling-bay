# 📦 Rungling Bay: 3rd Party Rust Packages

This document enumerates and classifies all third-party Rust packages (crates) utilized in the **Rungling Bay** project. It serves as a dependency audit map to understand how external libraries support the game's rendering, audio synthesis, input handling, and logging architectures.

---

## 🛠️ Direct Dependencies

These are the primary dependencies declared in [`Cargo.toml`](./Cargo.toml) and compiled directly into the application.

| Crate Name | Resolved Version | Classification | Core Purpose | Crates.io Link |
| :--- | :--- | :--- | :--- | :--- |
| **`ratatui`** | `0.30.1` | User Interface | Provides screen buffering and state diffing for rendering the terminal display. | [ratatui](https://crates.io/crates/ratatui) |
| **`crossterm`** | `0.29.0` | OS Integration & Input | Handles terminal raw mode, alternate screen buffers, and raw keyboard/resize events. | [crossterm](https://crates.io/crates/crossterm) |
| **`rodio`** | `0.22.2` | Sound Engine | Manages audio output device sink, mixing, and plays custom waveforms. | [rodio](https://crates.io/crates/rodio) |
| **`tracing`** | `0.1.44` | Diagnostics | Provides structured log instrumentation across the gameplay loops. | [tracing](https://crates.io/crates/tracing) |
| **`tracing-subscriber`** | `0.3.23` | Diagnostics | Formats log event traces and directs them to appenders. | [tracing-subscriber](https://crates.io/crates/tracing-subscriber) |
| **`tracing-appender`** | `0.2.5` | Diagnostics | Outputs traces to non-blocking rolling files (`rungling-bay.log`). | [tracing-appender](https://crates.io/crates/tracing-appender) |
| **`rand`** | `0.9.4` | Mathematics / Utilities | Supplies random coordinates and random timers for enemy spawning. | [rand](https://crates.io/crates/rand) |

---

## 🔍 Detailed Classification & Architectural Roles

### 1. Terminal UI & Draw Buffers
*   **`ratatui` (v0.30.1)**: Used as the screen display buffer. Rather than using the native widget framework (layouts, blocks, paragraphs), the game implements a custom Ratatui `Widget` inside [draw.rs](./src/game/draw.rs). It prints runes cell-by-cell into a 2D buffer. Ratatui's engine performs frame-by-frame diffing to only send the changed cells to the terminal, eliminating screen flicker.
*   **`crossterm` (v0.29.0)**: Used as the backend terminal interface. In [main.rs](./src/main.rs), it configures terminal raw mode, enters/leaves the alternate screen, and processes keyboard input messages.

### 2. Audio & Waveform Playback
*   **`rodio` (v0.22.2)**: Used to drive the C64-style sound engine. It registers a background playback thread in [main.rs](./src/main.rs) and opens the default audio sink.
*   It feeds procedural sound iterators ([SynthSound](./src/game/sound.rs#L55)) generating mono audio waves directly into the audio device mixer.

### 3. Application Diagnostics & Logging
*   **`tracing` Suite (v0.1.44, v0.3.23, v0.2.5)**: Implements structured logging.
    *   `tracing`: Provides macros (`info!`, `warn!`, `debug!`) used throughout the physics and input threads.
    *   `tracing-subscriber`: Intercepts events, builds structured formatting.
    *   `tracing-appender`: Manages the non-blocking file output writer so logging does not block the hot gameplay thread.

### 4. Utilities
*   **`rand` (v0.9.4)**: Provides utility methods for non-deterministic random state generators. Used to spawn enemy boats, drones, and timers. Note that to prevent expensive thread-local RNG access inside the hot audio synthesis loop, a lightweight local Linear Congruential Generator (`Lcg` in [sound.rs](./src/game/sound.rs#L44)) is used instead.

---

## ⚓ Key Transitive Dependencies

These notable indirect dependencies are resolved automatically by Cargo as dependencies of our primary crates:

*   **`cpal` (v0.17.3)** *(dependency of `rodio`)*: The low-level Cross-Platform Audio Library. Connects directly to OS sound servers (ALSA on Linux, CoreAudio on macOS, WASAPI on Windows).
*   **`alsa` (v0.11.0) & `alsa-sys` (v0.4.0)** *(dependencies of `cpal`)*: Provide direct bindings to Linux ALSA sound APIs.
*   **`crossbeam-channel` (v0.5.14)** *(dependency of `tracing-appender`)*: Provides fast, thread-safe channels for multi-threaded log dispatching.
*   **`libc` (v0.2.169)**: Provides low-level system call bindings to the operating system C library.

---

## ⚙️ OS & System Library Mapping

When running on Linux, the following system-level packages must be present to compile and link these dependencies:

*   `libasound2-dev` (ALSA headers/libraries) - required to link `alsa-sys` and `cpal`.
*   `pkg-config` - required by build scripts to locate system ALSA libraries.
