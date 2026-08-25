use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

pub fn main() {
    println!("LOREFORGE M0 Bootstrap - Starting window loop...");
    let event_loop = EventLoop::new().expect("EventLoop creation failed");
    let _window = WindowBuilder::new()
        .with_title("LOREFORGE M0")
        .with_inner_size(winit::dpi::LogicalSize::new(1024, 768))
        .build(&event_loop)
        .expect("Window build failed");

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);
            if let Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } = event
            {
                println!("Closing LOREFORGE window.");
                elwt.exit();
            }
        })
        .unwrap();
}
