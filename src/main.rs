use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::Window,
};

struct RenderState {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
}

impl RenderState {
    async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor::default(),
                None, // Trace path
            )
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let state = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
        };

        // Configure surface for the first time
        state.configure_surface();

        state
    }

    fn get_window(&self) -> &Window {
        &self.window
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            // Request compatibility with the sRGB-format texture view we‘re going to create later.
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        // reconfigure the surface
        self.configure_surface();
    }

    fn render(&mut self) {
        // Create texture view
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                // Without add_srgb_suffix() the image we will be working with
                // might not be "gamma correct".
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Renders a GREEN screen
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Create the renderpass which will clear the screen.
        let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // If you wanted to call any drawing commands, they would go here.

        // End the renderpass.
        drop(renderpass);

        // Submit the command in the queue to execute
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<RenderState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(RenderState::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.resize(size);
            }
            _ => (),
        }
    }
}

fn main() {
    // let nestest = RomImage::load(File::open("nes-test-roms/other/").unwrap()).unwrap();
    // const MASTER_CLOCK_RATE: u64 = 236_250_000 / 11;
    // let (mut master_clock, clock_signal) = Clock::<MASTER_CLOCK_RATE>::new();
    // let mut system = ntsc_system(mapper_for(nestest.clone()));
    // system.run(clock_signal);
    // let clock_control = master_clock.run();
    // thread::sleep(Duration::from_secs(5));
    // println!("Done, no deadlocks");

    // drop(clock_control);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut nes = App::default();
    event_loop.run_app(&mut nes).unwrap();
    // let fk = ActiveEventLoop::create_window(&self, window_attributes) event_loop.create_window();

    // mas
    // for _ in 0..20 {
    //     // println!("Tick?");
    //     master_clock.pulse();
    // }
    // thread::sleep(Duration::from_secs(1));
    // for _ in 0..20 {
    //     // println!("Tick?");
    //     master_clock.pulse();
    // }
    // thread::sleep(Duration::from_secs(1));
    // master_clock.stop().unwrap();
    // {
    //     println!("Starting");
    //     master_clock.start();
    //     thread::sleep(Duration::from_millis(1000));
    //     // let mut system = running_system.stop();
    //     println!("Dropping");
    // }
    // // Theory: Split cycles into time slices of work as to play nice with non-realtime OS.
    // let mut cycles = 0;
    // for time_step in 1..=100 {
    //     let catchup_cycles = ((time::Instant::now() - start).as_millis() * 236250) - cycles;
    //     println!("Catchup cycles: {:?}", catchup_cycles);
    //     cycles += catchup_cycles;
    //     // Do work
    //     // let after = time::Instant::now();
    //     let expected_time = start + Duration::from_millis(time_step);
    //     let delay = expected_time - time::Instant::now();
    //     // println!("{:?} {:?}", delay, expected_time);
    //     // let before = time::Instant::now();
    //     thread::sleep(delay);
    //     // println!("{:?} - {:?} = {:?}", after, before, after - before);
    // }
    // let end = time::Instant::now();
    // println!("Expected time step: 100ms, actual: {:?}", end - start);

    // let max_clock = u64::MAX;
    // println!("{}", max_clock / (236250000 / 11));
    // let max_sec = max_clock / (236250000 / 11);
    // let max_min = max_sec / 60;
    // let max_hr = max_min / 60;
    // let max_day = max_hr / 24;
    // println!("{}s {}m {}h {}d", max_sec, max_min, max_hr, max_day);
    // println!("{}", max_clock as f64 / (236250000.0 / 11.0));
}
