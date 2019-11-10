
mod ppu;
use ppu::{draw_line, LCD_WIDTH, LCD_HEIGHT};

extern crate image as im;
extern crate piston_window;
use piston_window::*;

fn main() {
    const ZOOM:u32 = 3;
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow =
        WindowSettings::new("rustyboy", [ZOOM*LCD_WIDTH, ZOOM*LCD_HEIGHT])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    let mut lcd = im::ImageBuffer::from_fn(LCD_WIDTH, LCD_HEIGHT, |x, y| {
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

    let rust_logo = "rust.png";
    let rust_logo: G2dTexture = Texture::from_path(
            &mut window.create_texture_context(),
            &rust_logo,
            Flip::None,
            &TextureSettings::new()
        ).unwrap();
    window.set_lazy(true);
    while let Some(e) = window.next() {
        window.draw_2d(&e, |c, g, _| {
            clear([1.0; 4], g);
            image(&texture, c.transform, g);
        });
    }
}
