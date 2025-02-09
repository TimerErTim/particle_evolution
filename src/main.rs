use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use pixels::{Error, Pixels, SurfaceTexture};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

// Communication Channels and Shared State
struct SharedState {
    draw_request: Arc<Mutex<Option<Vec<u8>>>>,
    events: Arc<Mutex<Vec<WindowEvent>>>,
}

// Simulation Thread Message Types
enum SimulationToMainMessage {
    DrawRequest(Vec<u8>),
    Terminate,
}

// Main Thread Message Types
enum MainToSimulationMessage {
    Events(Vec<WindowEvent>),
}

fn main() -> Result<(), Error> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Create channels for thread communication
    let (sim_to_main_tx, sim_to_main_rx) = bounded(1);
    let (main_to_sim_tx, main_to_sim_rx) = bounded(1);

    // Shared state for synchronization
    let shared_state = Arc::new(SharedState {
        draw_request: Arc::new(Mutex::new(None)),
        events: Arc::new(Mutex::new(Vec::new())),
    });

    // Spawn simulation thread
    let shared_state_clone = Arc::clone(&shared_state);
    let simulation_thread = std::thread::spawn(move || {
        simulation_loop(shared_state_clone, sim_to_main_tx, main_to_sim_rx)
    });

    // Setup pixels renderer
    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(640, 480, surface_texture)?;

    // Main event loop
    event_loop.run(move |event, _, control_flow| {
        // Default control flow
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => {
                // Collect events
                {
                    let mut events = shared_state.events.lock();
                    events.push(event.clone());
                }

                // Forward events to simulation thread
                if let Err(_) = main_to_sim_tx.try_send(MainToSimulationMessage::Events(
                    shared_state.events.lock().clone(),
                )) {
                    eprintln!("Failed to send events to simulation thread");
                }

                // Clear events after sending
                shared_state.events.lock().clear();

                // Handle window close
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => {}
                }
            }
            Event::MainEventsCleared => {
                // Check for draw requests from simulation thread
                if let Ok(SimulationToMainMessage::DrawRequest(frame_data)) =
                    sim_to_main_rx.try_recv()
                {
                    let mut pixels_frame = pixels.frame_mut();
                    pixels_frame.copy_from_slice(&frame_data);

                    if let Err(_) = pixels.render() {
                        eprintln!("Failed to render frame");
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            Event::LoopDestroyed => {
                // Signal simulation thread to terminate
                let _ = sim_to_main_tx.send(SimulationToMainMessage::Terminate);
                let _ = simulation_thread.join();
            }
            _ => {}
        }
    });
}

pub trait Renderer {
    type RenderWorld;

    fn extract_render_world(&self, cr_state: &CoreState) -> Self::RenderWorld;

    fn render_world(&mut self, world: Self::RenderWorld);
}

pub struct RenderState {}

pub struct CoreState {
    world: CoreWorld,
}

pub struct CoreWorld {}

pub struct WinitRenderData {}

fn simulation_loop(
    shared_state: Arc<SharedState>,
    sim_to_main_tx: Sender<SimulationToMainMessage>,
    main_to_sim_rx: Receiver<MainToSimulationMessage>,
) {
    let mut frame_count = 0;
    let start_time = Instant::now();

    loop {
        // Check for messages from main thread
        if let Ok(MainToSimulationMessage::Events(events)) = main_to_sim_rx.try_recv() {
            // Process received events
            for event in events {
                match event {
                    WindowEvent::KeyboardInput { .. } => {
                        // Handle keyboard events
                    }
                    _ => {}
                }
            }
        }

        // Simulation update logic
        let frame_data = simulate_frame(frame_count);

        // Send draw request to main thread
        match sim_to_main_tx.try_send(SimulationToMainMessage::DrawRequest(frame_data)) {
            Ok(_) => frame_count += 1,
            Err(_) => {
                // Skip frame if main thread is busy
                eprintln!("Skipping frame due to busy main thread");
            }
        }

        // Optional: Basic FPS control
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS

        // Optional: Exit condition or performance tracking
        if start_time.elapsed() > Duration::from_secs(60) {
            println!(
                "Simulation ran for 60 seconds. Total frames: {}",
                frame_count
            );
            break;
        }
    }
}

fn simulate_frame(frame_number: usize) -> Vec<u8> {
    // Placeholder simulation logic
    // In a real implementation, this would generate actual pixel data
    let width = 640;
    let height = 480;
    let mut frame_data = vec![0; width * height * 4];
    let asp = vec![] + "123";
    let idk = &mut asp;

    // Simple gradient or animation based on frame number
    for y in 0..height {
        for x in 0..width {
            let index = (y * width + x) * 4;
            frame_data[index] = (x % 256) as u8; // R
            frame_data[index + 1] = (y % 256) as u8; // G
            frame_data[index + 2] = (frame_number % 256) as u8; // B
            frame_data[index + 3] = 255; // A
        }
    }

    frame_data
}
