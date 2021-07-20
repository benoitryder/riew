use std::fs;
use std::cell::Cell;
use std::path::{Path, PathBuf};
use sdl2::mouse::{MouseButton, MouseState, MouseWheelDirection};
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use crate::display::{Display, Image, Font};
use lazy_static::lazy_static;


/// The main application
pub struct App {
    display: Display,
    files: Vec<PathBuf>,
    /// Index of current file in `files`
    file_index: Option<usize>,
    /// Current image (None if `file_index` is None)
    image: Option<CurrentImage>,
    /// Current zoom level
    zoom: f32,
    /// True if a redraw is required
    dirty: Cell<bool>,
}

/// Image currently displayed
struct CurrentImage {
    /// Current image
    image: Image,
    /// Pixel displayed at the center of the screen
    pos: (f32, f32),
    /// Rotation angle, in degrees
    angle: i32,
    /// Last drag position, None if drag is not active
    drag: Option<(i32, i32)>,
    /// Displayed pixel information
    pixel_info: Option<((i32, i32), Color)>,
}

lazy_static! {
    /// Zoom steps used when zooming in/out
    static ref ZOOM_STEPS: Vec<f32> = (0..0)
        .chain((  15..  50).step_by(   7))
        .chain((  50.. 100).step_by(  10))
        .chain(( 100.. 200).step_by(  25))
        .chain(( 200.. 600).step_by( 100))
        .chain(( 600..1000).step_by( 200))
        .chain((1000..2000).step_by( 500))
        .chain((2000..5000).step_by(1000))
        .map(|v| v as f32 / 100.).collect();
}


impl App {
    const DEFAULT_WINDOW_SIZE: (u32, u32) = (800, 500);
    const DEFAULT_BG_COLOR: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    const FILE_INFO_COLOR: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    const FILE_INFO_POS: (i32, i32) = (10, 5);
    const PIXEL_INFO_COLOR: Color = Color { r: 255, g: 0, b: 255, a: 255 };
    const PIXEL_INFO_POS: (i32, i32) = (10, 30);
    const OUTLINE_COLOR: Color = Color { r: 0, g: 0, b: 0, a: 255 };

    /// Create the application, initialize files from paths
    pub fn init(paths: &Vec<&Path>) -> Result<Self, String> {
        let mut display = Display::init(Self::DEFAULT_WINDOW_SIZE)?;
        display.bg_color = Self::DEFAULT_BG_COLOR;

        let mut app = Self {
            display: display,
            files: Vec::new(),
            file_index: None,
            image: None,
            zoom: 1.,
            dirty: Cell::new(true),
        };
        app.set_filelist(paths)?;

        Ok(app)
    }

    /// Run the main loop
    pub fn run(&mut self) -> Result<(), String> {
        self.refresh();
        //TODO disable unneeded events
        let mut pump = self.display.sdl_context.event_pump()?;
        loop {
            let event = pump.wait_event();
            match event {
                // quit event, or Escape
                Event::Quit{..} => { return Ok(()) },
                Event::Window{ win_event, .. } => {
                    match win_event {
                        WindowEvent::Resized(..) | WindowEvent::SizeChanged(..) => {
                            self.dirty.set(true);
                        },
                        _ => {},
                    }
                },
                Event::TextInput{ text, .. } => {
                    self.handle_textinput(text.as_str());
                },
                Event::KeyDown{ keycode: Some(keycode), keymod, .. } => {
                    self.handle_keypress(keycode, keymod);
                },
                Event::MouseButtonUp{ mouse_btn, clicks, x, y, .. } => {
                    self.handle_mouse_release(mouse_btn, clicks, (x, y));
                },
                Event::MouseMotion{ mousestate, x, y, .. } => {
                    self.handle_mouse_move(mousestate, (x, y), &pump);
                },
                Event::MouseWheel{ x, y, direction, .. } => {
                    let (dx, dy) = match direction {
                        MouseWheelDirection::Flipped => (-x, -y),
                        _ => (x, y),
                    };
                    self.handle_mousewheel((dx, dy), &pump);
                },
                _ => continue,
            }
            self.refresh();
        }
    }

    /// Update the list of files
    pub fn set_filelist(&mut self, paths: &Vec<&Path>) -> Result<(), String> {
        let mut files = Vec::<PathBuf>::new();
        for path in paths {
            if path.is_dir() {
                for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                    let entry_path = entry.map_err(|e| e.to_string())?.path();
                    if path.is_dir() && is_image_path(&entry_path) {
                        files.push(entry_path);
                    }
                }
            } else {
                assert!(path.is_file());
                let owned_path = path.to_path_buf();
                if is_image_path(&owned_path) {
                    files.push(owned_path);
                }
            }
        }

        files.sort_unstable();
        files.dedup();

        self.files = files;

        //TODO load the first file from parameters
        self.change_file(Some(0));
        self.zoom_adjust();

        Ok(())
    }

    /// Change current file
    pub fn change_file(&mut self, index: Option<usize>) {
        // wrap index around file length
        let new_index =
            if self.files.is_empty() {
                None
            } else {
                index.map(|i| i % self.files.len())
            };
        if new_index == self.file_index {
            return;
        }

        self.file_index = new_index;

        self.image = {
            let index = try_some!(self.file_index);
            match self.display.load_image(&self.files[index]) {
                Ok(image) => {
                    let (sx, sy) = size_as!(image.size(), f32);
                    Some(CurrentImage {
                        image,
                        pos: (sx / 2., sy / 2.),  // centered
                        angle: 0,
                        drag: None,
                        pixel_info: None,
                    })
                }
                Err(e) => {
                    eprintln!("failed to load image: {}", e);
                    None
                }
            }
        };
        self.dirty.set(true);
    }

    /// Change current file, relative
    pub fn change_file_rel(&mut self, offset: i32) {
        let nfiles = self.files.len() as i32;
        let index =
            if nfiles == 0 {
                None
            } else {
                // get a positive offset (index will be wrapped in change_file() if needed)
                let offset = (offset % nfiles + nfiles) as usize;
                Some(self.file_index.unwrap_or(0) + offset)
            };
        self.change_file(index)
    }

    /// Move image to absolute position
    pub fn move_to(&mut self, pos: (f32, f32)) {
        let image = try_some!(self.image.as_mut());
        image.pos = pos;
        self.clamp_pos();
        self.dirty.set(true);
    }

    /// Move image, relatively to current position
    pub fn move_rel(&mut self, offset: (f32, f32)) {
        let pos = {
            let image = try_some!(self.image.as_ref());
            let (dx, dy) = offset;
            let (px, py) = image.pos;
            (px + dx, py + dy)
        };
        self.move_to(pos);
    }

    /// Scroll pages, preserve zoom (step is 1 for one screen height)
    pub fn scroll(&mut self, step: f32) {
        let image = try_some!(self.image.as_mut());
        let (_, out_sy) = size_as!(self.display.size(), f32);
        let (_, img_sy) = size_as!(image.image.size(), f32);
        let (_, pos_y) = image.pos;

        let dy = step * out_sy / self.zoom;
        // small margin to avoid avoid blocking near the bottom
        const MARGIN: f32 = 1.5;
        if dy >= 0. && pos_y + dy / 2. + MARGIN > img_sy {
            self.change_file_rel(1);
            self.move_to((0., 0.));
        } else if dy < 0. && pos_y + dy / 2. - MARGIN < 0. {
            self.change_file_rel(-1);
            self.move_to((0., std::f32::MAX));
        } else {
            self.move_rel((0., dy));
        }
    }

    /// Clamp image position if needed
    fn clamp_pos(&mut self) {
        let image = try_some!(self.image.as_mut());
        let (out_sx, out_sy) = size_as!(self.display.size(), f32);
        let (img_sx, img_sy) = size_as!(image.image.size(), f32);
        let (dst_sx, dst_sy) = (out_sx / self.zoom, out_sy / self.zoom);

        let (px, py) = image.pos;
        // center or clamp
        let px = if img_sx <= dst_sx {
            img_sx / 2.
        } else {
            clamp!(px, dst_sx / 2., img_sx - dst_sx / 2.)
        };
        let py = if img_sy <= dst_sy {
            img_sy / 2.
        } else {
            clamp!(py, dst_sy / 2., img_sy - dst_sy / 2.)
        };

        image.pos = (px, py);
        self.dirty.set(true);
    }

    /// Adjust zoom level to display the whole image
    pub fn zoom_adjust(&mut self) {
        let image = try_some!(self.image.as_ref());
        let (out_sx, out_sy) = size_as!(self.display.size(), f32);
        let (img_sx, img_sy) = size_as!(image.image.size(), f32);
        self.zoom = 1f32.min(out_sx / img_sx).min(out_sy / img_sy);
        self.clamp_pos();
        self.dirty.set(true);
    }

    /// Zoom in, by one step
    pub fn zoom_in(&mut self, center: Option<(f32, f32)>) {
        if let Some(zoom) = ZOOM_STEPS.iter().filter(|z| **z > self.zoom).next() {
            self.set_zoom(*zoom, center);
        }
    }

    /// Zoom out, by one step
    pub fn zoom_out(&mut self, center: Option<(f32, f32)>) {
        if let Some(zoom) = ZOOM_STEPS.iter().rev().filter(|z| **z < self.zoom).next() {
            self.set_zoom(*zoom, center);
        }
    }

    /// Set zoom level, zoom on given point or current position
    pub fn set_zoom(&mut self, zoom: f32, center: Option<(f32, f32)>) {
        let image = try_some!(self.image.as_mut());

        // clamp zoom to sensible values 
        let zoom = clamp!(zoom, 0.001, 1000.);

        // Shift the image position if not zooming on it.
        // Translate the position by how much the zoom center moved.
        //   |CP| = |CP'| * k
        //
        //   |CP| = |CP'| * k  => (P-C) = (P'-C) * k
        // C*(k-1) = k*P'-P
        // C*(k-1) = k*P'-P
        if let Some((cx, cy)) = center {
            let k = zoom / self.zoom;
            let (px, py) = image.pos;
            let (dx, dy) = ((px - cx) / k, (py - cy) / k);
            image.pos = (cx + dx, cy + dy);
        }

        self.zoom = zoom;
        self.clamp_pos();
        self.dirty.set(true);
    }

    /// Return true if the whole image fits in the display
    fn is_adjusted(&self) -> bool {
        let image = try_some!(self.image.as_ref(), true);

        let (out_sx, out_sy) = size_as!(self.display.size(), f32);
        let (img_sx, img_sy) = size_as!(image.image.size(), f32);

        // Round because of possible accuracy issues for large images
        out_sx >= (img_sx * self.zoom).round() && out_sy >= (img_sy * self.zoom).round()
    }

    /// Rotate image to given angle, in degrees
    pub fn rotate_to(&mut self, angle: i32) {
        let image = try_some!(self.image.as_mut());
        image.angle = angle % 360;
        self.dirty.set(true);
    }

    /// Rotate image by given angle, in degrees
    pub fn rotate_rel(&mut self, angle: i32) {
        let image = try_some!(self.image.as_mut());
        image.angle = (image.angle + angle) % 360;
        self.dirty.set(true);
    }

    /// Redraw the screen, forcily
    pub fn redraw(&mut self) {
        self.display.clear();

        //TODO don't redraw the text each time, keep it in a texture
        let file_text =
            if self.file_index.is_none() {
                format!("[no file]")
            } else if let Some(image) = self.image.as_ref() {
                self.display.draw_image(&image.image, image.pos, self.zoom, image.angle);
                format!("{}  ( {} × {} )  [ {} / {} ]  {} %",
                             image.image.path,
                             image.image.width,
                             image.image.height,
                             self.file_index.unwrap() + 1, self.files.len(),
                             (self.zoom * 100.) as u32)
            } else {
                format!("[invalid file]  [ {} / {} ]",
                             self.file_index.unwrap() + 1, self.files.len())
            };
        self.display.draw_text_outline(Font::Normal, file_text.as_str(), Self::FILE_INFO_COLOR, Self::OUTLINE_COLOR, Self::FILE_INFO_POS);

        if let Some((pixel_pos, color)) = self.image.as_ref().and_then(|i| i.pixel_info).as_ref() {
            let mut pos = Self::PIXEL_INFO_POS;
            pos = self.display.draw_text_outline(
                Font::Normal, format!("( {} , {} )  ", pixel_pos.0, pixel_pos.1).as_str(),
                Self::PIXEL_INFO_COLOR, Self::OUTLINE_COLOR, pos);
            self.display.draw_rectangle(Rect::new(pos.0, pos.1, 15, 15), *color);
            pos.0 += 15;
            pos = self.display.draw_text_outline(
                Font::Normal, format!("  #{:02X}{:02X}{:02X}  ", color.r, color.g, color.b).as_str(),
                Self::PIXEL_INFO_COLOR, Self::OUTLINE_COLOR, pos);
            pos = self.display.draw_text_outline(
                Font::Normal, format!(" {}", color.r).as_str(),
                Color::RGB(255, 0, 0), Self::OUTLINE_COLOR, pos);
            pos = self.display.draw_text_outline(
                Font::Normal, format!(" {}", color.g).as_str(),
                Color::RGB(0, 255, 0), Self::OUTLINE_COLOR, pos);
            /*pos =*/ self.display.draw_text_outline(
                Font::Normal, format!(" {}", color.b).as_str(),
                Color::RGB(0, 0, 255), Self::OUTLINE_COLOR, pos);
        }

        self.display.refresh();
        self.dirty.set(false);
    }

    /// Redraw the screen if dirty
    pub fn refresh(&mut self) {
        if self.dirty.get() {
            self.redraw();
        }
    }

    /// Push an event to exit
    pub fn quit(&self) {
        let event_subsystem = self.display.sdl_context.event().unwrap();
        // flush all events to be sure to quit right away
        event_subsystem.flush_events(0, std::u32::MAX);
        event_subsystem.push_event(Event::Quit{ timestamp: 0 }).unwrap();
    }

    /// Handle text input events
    fn handle_textinput(&mut self, text: &str) {
        match text {
            // zoom
            "a" => self.zoom_adjust(),
            "z" => self.set_zoom(1., None),
            "-" => self.zoom_out(None),
            "+" => self.zoom_in(None),
            // rotation
            "r" => self.rotate_rel(90),
            "R" => self.rotate_rel(-90),

            "q" => self.quit(),
            "f" => self.display.toggle_fullscreen(),

            _ => {},
        }
    }

    /// Handle keyboard events
    fn handle_keypress(&mut self, keycode: Keycode, keymod: Mod) {
        // remove uninteresting mods
        let keymod = keymod & !(Mod::NUMMOD | Mod::CAPSMOD | Mod::MODEMOD);
        let nomod = keymod.is_empty();
        match keycode {
            Keycode::Escape if nomod => self.quit(),

            // space, backspace: scroll pages, preserve zoom
            Keycode::Space if nomod => self.scroll(1.),
            Keycode::Backspace if nomod => self.scroll(-1.),

            Keycode::PageUp => {
                self.change_file_rel(Self::filelist_step_from_mod(keymod));
                self.zoom_adjust();
            },
            Keycode::PageDown => {
                self.change_file_rel(-Self::filelist_step_from_mod(keymod));
                self.zoom_adjust();
            },

            // arrows
            Keycode::Up => {
                self.move_rel((0., -Self::move_step_from_mod(keymod)));
            },
            Keycode::Down => {
                self.move_rel((0., Self::move_step_from_mod(keymod)));
            },
            Keycode::Right => if self.is_adjusted() {
                self.change_file_rel(Self::filelist_step_from_mod(keymod));
                self.zoom_adjust();
            } else {
                self.move_rel((Self::move_step_from_mod(keymod), 0.));
            },
            Keycode::Left => if self.is_adjusted() {
                self.change_file_rel(-Self::filelist_step_from_mod(keymod));
                self.zoom_adjust();
            } else {
                self.move_rel((-Self::move_step_from_mod(keymod), 0.));
            }

            //TODO F5: reload list

            _ => {},
        }
    }

    /// Handle mouse wheel events
    fn handle_mousewheel(&mut self, step: (i32, i32), pump: &sdl2::EventPump) {
        let (_, step_y) = step;

        let alt_mod = {
            let state = pump.keyboard_state();
            state.is_scancode_pressed(Scancode::LAlt)
        };

        if alt_mod {
            if step_y > 0 {
                self.display.set_bg_brightness_rel(-0.1);
            } else {
                self.display.set_bg_brightness_rel(0.1);
            }
            self.dirty.set(true);

        } else {
            // zoom in/out

            let mouse_pos = {
                let mouse_state = pump.mouse_state();
                size_as!((mouse_state.x(), mouse_state.y()), f32)
            };
            let center = self.screen_to_image_pos(mouse_pos);
            if step_y > 0 {
                self.zoom_in(center);
            } else if step_y < 0 {
                self.zoom_out(center);
            }
        }
    }

    /// Handle mouse click release
    fn handle_mouse_release(&mut self, button: MouseButton, _clicks: u8, _pos: (i32, i32)) {
        let dragging = self.image.as_ref().and_then(|i| i.drag).is_some();
        match button {
            MouseButton::Left => {
                if dragging {
                    let image = self.image.as_mut().unwrap();
                    image.drag = None;  // end drag
                } else {
                    self.change_file_rel(-1);
                    self.zoom_adjust();
                }
            },
            MouseButton::Right => {
                if dragging {
                    // ignore click
                } else {
                    self.change_file_rel(1);
                    self.zoom_adjust();
                }
            },
            _ => {},
        }
    }

    /// Handle mouse click press
    fn handle_mouse_move(&mut self, state: MouseState, pos: (i32, i32), pump: &sdl2::EventPump) {
        if state.is_mouse_button_pressed(MouseButton::Left) {
            // Don't muse relative move for better precision
            // Also, it deals better with cursor leaving temporarily the window
            if let Some((x, y)) = self.image.as_ref().and_then(|i| i.drag) {
                let (px, py) = pos;
                self.move_rel(size_as!((x - px, y - py), f32));
            }
            let image = try_some!(self.image.as_mut());
            image.drag = Some(pos);
        } else {
            let keyboard_state = pump.keyboard_state();
            if keyboard_state.is_scancode_pressed(Scancode::LCtrl) {
                let pixel_pos = size_as!(try_some!(self.screen_to_image_pos(size_as!(pos, f32))), i32);
                let image = try_some!(self.image.as_mut());
                let pixel_color = self.display.draw_pixel_and_get_color(&image.image, pixel_pos).unwrap();
                image.pixel_info = Some((pixel_pos, pixel_color));
                self.dirty.set(true);
            }
        }
    }

    /// Get filelist step from a keyboard modifier
    fn filelist_step_from_mod(keymod: Mod) -> i32 {
        match keymod {
            Mod::LSHIFTMOD | Mod::RSHIFTMOD => 5,
            Mod::NOMOD | _ => 1,
        }
    }

    /// Get move step from a keyboard modifier
    fn move_step_from_mod(keymod: Mod) -> f32 {
        match keymod {
            Mod::LALTMOD | Mod::RALTMOD => 10.,
            Mod::LSHIFTMOD | Mod::RSHIFTMOD => 500.,
            Mod::NOMOD | _ => 50.,
        }
    }

    /// Convert screen position to image position
    fn screen_to_image_pos(&self, pos: (f32, f32)) -> Option<(f32, f32)> {
        let image = self.image.as_ref()?;
        let (out_sx, out_sy) = size_as!(self.display.size(), f32);
        let (pos_x, pos_y) = image.pos;
        let cx = pos_x + (pos.0 - out_sx / 2.) / self.zoom;
        let cy = pos_y + (pos.1 - out_sy / 2.) / self.zoom;
        let (img_sx, img_sy) = size_as!(image.image.size(), f32);
        if cx < 0. || cx > img_sx || cy < 0. || cy > img_sy {
            return None;
        }
        Some((cx, cy))
    }
}


/// Check if a path is an image path (based on extension)
fn is_image_path(path: &PathBuf) -> bool {
    const EXTENSIONS: [&'static str; 10] = [
        "tga", "bmp", "pnm", "gif", "jpg", "jpeg", "tif", "tiff", "png", "webp",
    ];

    if let Some(os_ext) = path.extension() {
        if let Some(ext) = os_ext.to_str() {
            return EXTENSIONS.contains(&ext.to_lowercase().as_str());
        }
    }
    false
}

