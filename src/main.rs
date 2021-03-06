use cgmath::{Matrix4, Rad, Transform, Vector3};
use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::ab_glyph;
use glutin::{
    event::{
        ElementState, Event, KeyboardInput, ModifiersState, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::ControlFlow,
};
use old_school_gfx_glutin_ext::*;
use std::{
    env,
    error::Error,
    f32::consts::PI as PI32,
    io::{self, Write},
};

fn main() -> Result<(), Box<dyn Error>> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "gfx_glyph=warn");
    }

    env_logger::init();

    if cfg!(target_os = "linux") {
        // winit wayland is currently still wip
        if env::var("WINIT_UNIX_BACKEND").is_err() {
            env::set_var("WINIT_UNIX_BACKEND", "x11");
        }
        // disables vsync sometimes on x11
        if env::var("vblank_mode").is_err() {
            env::set_var("vblank_mode", "0");
        }
    }

    let event_loop = glutin::event_loop::EventLoop::new();
    let title = "garbage-term";
    let window_builder = glutin::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(glutin::dpi::PhysicalSize::new(1024, 576));

    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        glutin::ContextBuilder::new()
            .with_gfx_color_depth::<Srgba8, Depth>()
            .build_windowed(window_builder, &event_loop)?
            .init_gfx::<Srgba8, Depth>();

    let font = ab_glyph::FontArc::try_from_slice(include_bytes!("../DejaVuSansMono.ttf"))?;
    let mut glyph_brush = gfx_glyph::GlyphBrushBuilder::using_font(font)
        .initial_cache_size((1024, 1024))
        .build(factory.clone());

    let mut text: String = "hello world".into();

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    // let mut running = true;
    let font_size: f32 = 18.0;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);

    let mut modifiers = ModifiersState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::ModifiersChanged(new_mods) => modifiers = new_mods,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    window_ctx.resize(size);
                    window_ctx.update_gfx(&mut main_color, &mut main_depth);
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Back),
                            ..
                        },
                    ..
                } => {
                    text.pop();
                }
                WindowEvent::ReceivedCharacter(c) => {
                    if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    }
                }

                _ => (),
            },
            Event::MainEventsCleared => {
                encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

                let (width, height, ..) = main_color.get_dimensions();
                let (width, height) = (f32::from(width), f32::from(height));
                let scale = font_size * window_ctx.window().scale_factor() as f32;

                // The section is all the info needed for the glyph brush to render a 'section' of text.
                let section = gfx_glyph::Section::default()
                    .add_text(
                        Text::new(&text)
                            .with_scale(scale)
                            .with_color([0.9, 0.3, 0.3, 1.0]),
                    )
                    .with_bounds((width, height));

                // Adds a section & layout to the queue for the next call to `use_queue().draw(..)`,
                // this can be called multiple times for different sections that want to use the
                // same font and gpu cache.
                // This step computes the glyph positions, this is cached to avoid unnecessary
                // recalculation.
                glyph_brush.queue(&section);

                use gfx_glyph::*;

                // Rotation
                let offset =
                    Matrix4::from_translation(Vector3::new(-width / 2.0, -height / 2.0, 0.0));
                let rotation =
                    offset.inverse_transform().unwrap() * Matrix4::from_angle_z(Rad(0.0)) * offset;

                // Default projection
                let projection: Matrix4<f32> = gfx_glyph::default_transform(&main_color).into();

                // Here an example transform is used as a cheap zoom out (controlled with ctrl-scroll)
                let zoom = Matrix4::from_scale(1.0);

                // Combined transform
                let transform = zoom * projection * rotation;

                // Finally once per frame you want to actually draw all the sections you've submitted
                // with `queue` calls.
                //
                // Note: Drawing in the case the text is unchanged from the previous frame (a common case)
                // is essentially free as the vertices are reused & gpu cache updating interaction
                // can be skipped.
                glyph_brush
                    .use_queue()
                    .transform(transform)
                    .draw(&mut encoder, &main_color)
                    .unwrap();

                encoder.flush(&mut device);
                window_ctx.swap_buffers().unwrap();
                device.cleanup();

                loop_helper.loop_sleep();
                loop_helper.loop_start();
            }
            Event::LoopDestroyed => println!(),
            _ => (),
        }
    });
}
