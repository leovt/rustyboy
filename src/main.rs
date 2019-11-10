
mod ppu;
use ppu::{draw_line, LCD_WIDTH, LCD_HEIGHT};

extern crate image as im;
extern crate piston_window;
extern crate fps_counter;
use piston_window::*;

fn main() {
    const ZOOM:u32 = 3;
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow =
        WindowSettings::new("rustyboy", [ZOOM*LCD_WIDTH as u32, ZOOM*LCD_HEIGHT as u32])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    let mut lcd = im::ImageBuffer::from_fn(LCD_WIDTH as u32, LCD_HEIGHT as u32, |x, y| {
        if x % 2 == 0 {
            im::Rgba([x as u8, 0u8, 255u8, 255u8])
        } else {
            im::Rgba([180u8, 0u8, 0u8, 255u8])
        }
    });
    let mut texture_context = TextureContext {
        factory: window.factory.clone(),
        encoder: window.factory.create_command_buffer().into()
    };
    let mut texture: G2dTexture = Texture::from_image(
            &mut texture_context,
            &lcd,
            &TextureSettings::new()
        ).unwrap();

    //window.set_lazy(false);
    //window.set_bench_mode(true);
    window.set_max_fps(60);
    let mut counter:usize = 0;
    let mut fps_ctr = fps_counter::FPSCounter::new();

    let mut ppusa = ppu::PpuStandalone::new();
    const PALETTE:[im::Rgba<u8>;4] = [
        im::Rgba([198,227,195,255]),
        im::Rgba([157,181,154,255]),
        im::Rgba([110,128,8,255]),
        im::Rgba([53,61,52,255]),
        ];

    while let Some(e) = window.next() {
        if let Some(_) = e.render_args() {
            counter += 1;
            let fps = fps_ctr.tick();
            if counter >= fps {
                println!("fps = {}", fps);
                counter = 0;
            }
            ppusa.draw_frame(&mut lcd, &PALETTE);
            texture.update(&mut texture_context, &lcd).unwrap();
        }
        window.draw_2d(&e, |c, g, device| {
            // Update texture before rendering.
            texture_context.encoder.flush(device);
            clear([1.0; 4], g);
            image(&texture, c.transform.zoom(ZOOM as f64), g);
        });
    }
}
