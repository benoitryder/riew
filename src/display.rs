use std::rc::Rc;
use std::path::PathBuf;
use sdl2::Sdl;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::image::LoadTexture;
use sdl2::video::{WindowContext, FullscreenType};
use sdl2::ttf::{Sdl2TtfContext, Font as TtfFont};
use sdl2::rwops::RWops;
use owning_ref::OwningHandle;

type OwnedTexture = OwningHandle<Rc<TextureCreator<WindowContext>>, Box<Texture<'static>>>;
type OwnedFont = OwningHandle<Rc<Sdl2TtfContext>, Box<TtfFont<'static, 'static>>>;


/// Image to be displayed
///
/// The texture is kept with creator to avoid lifetime issues.
pub struct Image {
    texture: OwnedTexture,
    pub width: u32,
    pub height: u32,
    pub path: String,
}

impl Image {
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

/// Manage fonts (each with an "outline" version)
///
/// The fonts are kept with their TTF context adn RWops to avoid lifetime issues.
struct FontManager {
    normal: (OwnedFont, OwnedFont),
    mono: (OwnedFont, OwnedFont),
}

/// List of available fonts, to be used by the display user
pub enum Font {
    Normal,
    Mono,
}


impl FontManager {
    pub fn init() -> Result<Self, String> {
        let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
        let ttf_context = Rc::new(ttf_context);

        macro_rules! load_font {
            ($context:expr, $name:literal, $size:expr, $outline:expr) => {
                (Self::load_font($context.clone(), include_bytes!(concat!("../res/", $name)), $size, 0)?,
                 Self::load_font($context.clone(), include_bytes!(concat!("../res/", $name)), $size, $outline)?)
            }
        }

        Ok(Self {
            normal: load_font!(ttf_context, "DejaVuSans.ttf", 12, 1),
            mono: load_font!(ttf_context, "DejaVuSansMono.ttf", 12, 1),
        })
    }

    fn load_font(ttf_context: Rc<Sdl2TtfContext>, bytes: &'static [u8], size: u16, outline: u16) -> Result<OwnedFont, String> {
        let mut font = OwningHandle::try_new(ttf_context, |o| -> Result<_, String> {
            let rwops = RWops::from_bytes(bytes)?;
            let font = unsafe { (*o).load_font_from_rwops(rwops, size)? };
            Ok(Box::new(font))
        })?;
        if outline != 0 {
            font.set_outline_width(outline);
        }
        Ok(font)
    }

    pub fn get_font(&self, font: Font) -> &(OwnedFont, OwnedFont) {
        match font {
            Font::Normal => &self.normal,
            Font::Mono => &self.mono,
        }
    }

}


/// SDL context and related data
///
/// On Windows, textures copied to the canvas must be alive until rendered.
/// As a result, a reference to temporary textures is kept until the clear is cleared.
/// This means `clear()` should always be called before rendering a new frame.
pub struct Display {
    pub sdl_context: Sdl,
    fonts: FontManager,
    canvas: WindowCanvas,
    texture_creator: Rc<TextureCreator<WindowContext>>,
    pub bg_color: Color,
    rendered_textures: Vec<OwnedTexture>,
}


impl Display {
    pub fn init(size: (u32, u32)) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
        let window = video_subsystem.window("riew", size.0, size.1)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| e.to_string())?;
        let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
        let texture_creator = Rc::new(canvas.texture_creator());
        let fonts = FontManager::init()?;

        Ok(Self {
            sdl_context,
            fonts,
            canvas,
            texture_creator,
            bg_color: Color::RGB(0, 0, 0),
            rendered_textures: Vec::new(),
        })
    }

    /// Return the display size 
    pub fn size(&self) -> (u32, u32) {
        self.canvas.output_size().unwrap()
    }

    pub fn load_image(&self, path: &PathBuf) -> Result<Image, String> {
        let creator = self.texture_creator.clone();
        let texture = OwningHandle::try_new(creator, |o| -> Result<_, String> {
            let t = unsafe { (*o).load_texture(path)? };
            Ok(Box::new(t))
        })?;

        let query = texture.query();
        let image = Image {
            texture,
            width: query.width,
            height: query.height,
            path: path.to_string_lossy().into_owned(),
        };

        Ok(image)
    }

    /// Draw an image
    pub fn draw_image(&mut self, image: &Image, center: (f32, f32), zoom: f32, angle: i32) {
        let (out_sx, out_sy) = size_as!(self.size(), f32);
        let (img_sx, img_sy) = size_as!(image.size(), f32);
        let (dst_sx, dst_sy) = (img_sx * zoom, img_sy * zoom);
        let dst_x = out_sx / 2. - center.0 * zoom;
        let dst_y = out_sy / 2. - center.1 * zoom;

        let dst = Rect::new(dst_x as i32, dst_y as i32, dst_sx as u32, dst_sy as u32);
        self.canvas.copy_ex(&image.texture, None, dst, angle as f64, None, false, false).unwrap();
    }

    /// Draw text
    pub fn draw_text(&mut self, font: Font, text: &str, color: Color, pos: (i32, i32)) -> (i32, i32) {
        let (font, _) = self.fonts.get_font(font);
        Self::draw_text_internal(&mut self.canvas, self.texture_creator.clone(), &mut self.rendered_textures, font, text, color, pos)
    }

    /// Draw text with outline
    pub fn draw_text_outline(&mut self, font: Font, text: &str, color: Color, color_outline: Color, pos: (i32, i32)) -> (i32, i32) {
        let (font, font_outline) = self.fonts.get_font(font);
        let outline = font_outline.get_outline_width() as i32;

        Self::draw_text_internal(&mut self.canvas, self.texture_creator.clone(), &mut self.rendered_textures, font_outline, text, color_outline, (pos.0 - outline, pos.1 - outline));
        Self::draw_text_internal(&mut self.canvas, self.texture_creator.clone(), &mut self.rendered_textures, font, text, color, pos)
    }

    /// Render text and draw it, return the end position
    fn draw_text_internal(canvas: &mut WindowCanvas, texture_creator: Rc<TextureCreator<WindowContext>>, rendered_textures: &mut Vec<OwnedTexture>, font: &OwnedFont, text: &str, color: Color, pos: (i32, i32)) -> (i32, i32) {
        let surface = font.render(text).blended(color).unwrap();
        let size = surface.size();
        let texture = OwningHandle::try_new(texture_creator, |o| -> Result<_, String> {
            let t = unsafe { (*o).create_texture_from_surface(surface).map_err(|e| e.to_string())? };
            Ok(Box::new(t))
        }).unwrap();

        let dst = Rect::new(pos.0, pos.1, size.0, size.1);
        canvas.copy(&texture, None, dst).unwrap();
        rendered_textures.push(texture);
        (dst.right(), pos.1)
    }

    /// Clear the display with the background color
    pub fn clear(&mut self) {
        self.canvas.set_draw_color(self.bg_color);
        self.canvas.clear();
        self.rendered_textures.clear();
    }

    /// Redraw the screen
    pub fn refresh(&mut self) {
        self.canvas.present();
    }

    /// Set fullscreen state
    pub fn set_fullscreen(&mut self, state: bool) {
        let state = if state {
            FullscreenType::Desktop
        } else {
            FullscreenType::Off
        };

        let window = self.canvas.window_mut();
        window.set_fullscreen(state).unwrap();
    }

    /// Toggle fullscreen state
    pub fn toggle_fullscreen(&mut self) {
        let state = self.canvas.window().fullscreen_state();
        let current = match state {
            FullscreenType::Off => false,
            FullscreenType::True => true,
            FullscreenType::Desktop => true,
        };
        self.set_fullscreen(!current);
    }

    /// Change background color brightness
    pub fn set_bg_brightness_rel(&mut self, offset: f32) {
        let mut color = self.bg_color;
        let offset = offset * 256.;
        if offset > 0. {
            color.r = color.r.saturating_add(offset as u8);
            color.g = color.g.saturating_add(offset as u8);
            color.b = color.b.saturating_add(offset as u8);
        } else {
            let offset = -offset;
            color.r = color.r.saturating_sub(offset as u8);
            color.g = color.g.saturating_sub(offset as u8);
            color.b = color.b.saturating_sub(offset as u8);
        }
        self.bg_color = color;
    }

    /// Draw a filled rectangle
    pub fn draw_rectangle(&mut self, rect: Rect, color: Color) {
        self.canvas.set_draw_color(color);
        self.canvas.fill_rect(rect).unwrap();
    }

    /// Draw a single pixel from an image and return its color 
    pub fn draw_pixel_and_get_color(&mut self, image: &Image, pos: (i32, i32)) -> Result<Color, String> {
        // Only render targets can be read, that's why we need to draw the pixel.
        // And the texture cannot be drawn to a new, blank surface.
        self.canvas.copy(&image.texture, Rect::new(pos.0, pos.1, 1, 1), Rect::new(0, 0, 1, 1))?;
        let pixels = self.canvas.read_pixels(None, PixelFormatEnum::RGBA32)?;
        Ok(Color::RGB(pixels[0], pixels[1], pixels[2]))
    }
}

