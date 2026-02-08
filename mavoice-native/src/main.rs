mod api;
mod app;
mod audio;
mod config;
mod renderer;
mod state_machine;
mod system;

use std::sync::Arc;
use winit::event_loop::EventLoop;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("maVoice native v{}", env!("CARGO_PKG_VERSION"));

    // Build tokio runtime on a background thread
    let tokio_rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime"),
    );

    // Create event loop with custom AppEvent
    let event_loop = EventLoop::<app::AppEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let proxy = event_loop.create_proxy();

    let mut app = app::App::new(tokio_rt, proxy);

    log::info!("Starting event loop");
    event_loop.run_app(&mut app).expect("Event loop failed");
}
