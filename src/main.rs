use std::env;
mod cpu;
mod ppu;
mod debugger;
mod instructions;
use ppu::{LCD_WIDTH, LCD_HEIGHT, Ppu};
use cpu::{Cpu, Mmu};
use debugger::Debugger;

extern crate image as im;
extern crate piston_window;
extern crate fps_counter;
use piston_window::*;

fn main_ppu() {
    const ZOOM:u32 = 3;
    let opengl = OpenGL::V3_2;
    let mut window: PistonWindow =
        WindowSettings::new("rustyboy", [ZOOM*LCD_WIDTH as u32, ZOOM*LCD_HEIGHT as u32])
        .exit_on_esc(true)
        .graphics_api(opengl)
        .build()
        .unwrap();

    let mut lcd = im::ImageBuffer::from_pixel(LCD_WIDTH as u32, LCD_HEIGHT as u32, im::Rgba([0u8;4]));
    let mut texture_context = TextureContext {
        factory: window.factory.clone(),
        encoder: window.factory.create_command_buffer().into()
    };
    let mut texture: G2dTexture = Texture::from_image(
            &mut texture_context,
            &lcd,
            &{let mut ts = TextureSettings::new(); ts.set_mag(Filter::Nearest); ts},
        ).unwrap();

    //window.set_lazy(false);
    //window.set_bench_mode(true);
    window.set_max_fps(60);
    let mut fps_print_ctr:usize = 0;
    let mut fps_ctr = fps_counter::FPSCounter::new();
    let mut ups_ctr = fps_counter::FPSCounter::new();
    let mut ups = 0usize;

    let mut mmu = Mmu::new();
    {
        let args: Vec<String> = env::args().collect();
        mmu.load(&args[1], 0);
        if args.len() > 2 {
            mmu.load_boot_rom(&args[2]);
        } else {
            mmu.load_boot_rom("RBOY_ROM.bin");
        }
    }

    let cpu = Cpu::new(mmu);
    let ppu = Ppu::new();
    let mut dbg = Debugger::new(cpu, ppu);

    let ups_target = 120;
    let cycles_per_second = 4*1024*1024;
    let cycles_per_update = cycles_per_second / ups_target;

    while let Some(e) = window.next() {
        if let Some(_) = e.update_args() {
            dbg.interact(&mut lcd, cycles_per_update);
            ups = ups_ctr.tick();
        }
        if let Some(_) = e.render_args() {
            fps_print_ctr += 1;
            let fps = fps_ctr.tick();
            if fps_print_ctr >= fps {
                println!("fps = {}   ups = {}", fps, ups);
                fps_print_ctr = 0;
            }
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

fn main(){
    main_ppu();
    //debugger::main();
}
