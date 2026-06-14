mod game;

use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::info;
use tracing_appender::rolling;
use tracing_subscriber::fmt;

use game::Game;
use game::input::InputMsg;

fn main() -> io::Result<()> {
    // File-based structured logging (matches Go's gobungle.log)
    let file_appender = rolling::never(".", "rungling-bay.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    fmt().with_writer(non_blocking).init();

    info!("Rungling Bay Game Started");

    // Initialize audio system
    let (audio_tx, audio_rx) = mpsc::channel::<game::sound::SoundType>();
    
    // Spawn audio playback thread
    thread::spawn(move || {
        use rodio::DeviceSinkBuilder;
        let handle = match DeviceSinkBuilder::open_default_sink() {
            Ok(res) => res,
            Err(e) => {
                tracing::warn!("Failed to initialize audio speaker: {e} (running in silent mode)");
                while audio_rx.recv().is_ok() {}
                return;
            }
        };

        tracing::info!("Audio system successfully initialized using Rodio");

        let mut last_play_time = std::collections::HashMap::new();

        while let Ok(sound_type) = audio_rx.recv() {
            let now = Instant::now();
            if last_play_time.get(&sound_type).is_some_and(|&prev| now.duration_since(prev) < Duration::from_millis(60)) {
                continue;
            }
            last_play_time.insert(sound_type, now);

            let (dur, volume) = match sound_type {
                game::sound::SoundType::Warning => (Duration::from_millis(500), 0.20),
                game::sound::SoundType::Laser => (Duration::from_millis(55), 0.28),
                game::sound::SoundType::Missile => (Duration::from_millis(400), 0.20),
                game::sound::SoundType::Explosion => (Duration::from_millis(800), 0.38),
                game::sound::SoundType::Speedboat => (Duration::from_millis(300), 0.18),
            };

            let source = game::sound::SynthSound::new(44100, dur, sound_type, volume);
            let player = rodio::Player::connect_new(handle.mixer());
            player.append(source);
            player.detach();
        }
    });

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut game = Game::new(size.width as i32, size.height as i32, Some(audio_tx));

    // Input channel: keyboard thread → main loop
    let (tx, rx) = mpsc::channel::<InputMsg>();
    thread::spawn(move || {
        loop {
            match event::read() {
                Ok(Event::Key(key)) if tx.send(InputMsg::Key(key)).is_err() => break,
                Ok(Event::Resize(w, h)) if tx.send(InputMsg::Resize(w, h)).is_err() => break,
                _ => {}
            }
        }
    });

    // Main loop: Option B (channel-based, game state owned by main thread)
    let tick_dur = Duration::from_millis(40); // 25 FPS
    let mut deadline = Instant::now() + tick_dur;
    let mut running = true;

    while running {
        // Block until a key arrives or the tick fires, whichever is first
        let timeout = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(InputMsg::Key(key)) => {
                if !game.handle_raw_key(&key) {
                    running = false;
                }
                // Drain any additional buffered events before the tick
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
                            info!(width = w, height = h, "Screen resized");
                        }
                    }
                }
            }
            Ok(InputMsg::Resize(w, h)) => {
                game.width = w as i32;
                game.height = h as i32;
                info!(width = w, height = h, "Screen resized");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                running = false;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        // Physics + render on the 40ms tick boundary
        if Instant::now() >= deadline {
            if !game.quit_confirming && !game.game_over {
                game.update_physics(); // includes apply_joystick_input + get_locked_target
            }

            terminal.draw(|frame| {
                frame.render_widget(&game, frame.area());
            })?;

            deadline += tick_dur;
        }
    }

    info!("Rungling Bay Game Shutting Down Gracefully");

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
