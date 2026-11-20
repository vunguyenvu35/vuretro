//! The game window on winit + OpenGL: the picture is a texture drawn by a shader, and the
//! window is the GL context a hardware-rendered core will draw into later.
//!
//! The session owns the loop (`while screen.is_open() { frame() }`), so the window does not
//! hand control to winit's event loop; it asks winit to deliver what is waiting
//! (`pump_app_events`) each time the session presents a frame or idles, like minifb did.

use crate::fit::{fit_rect, whole_fit, Rect};
use crate::gl_keys::to_code;
use crate::gl_menu::GameMenu;
use crate::hw::{self, GlRequest};
use anyhow::{anyhow, bail, Result};
use glow::HasContext;
use glutin::config::{Config, ConfigTemplateBuilder};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, GlProfile, NotCurrentGlContext, PossiblyCurrentContext,
    Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::prelude::GlConfig;
use glutin::surface::{GlSurface, Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Duration;
use vuretro_play::ports::{Keyboard, Look, Screen, View};
use vuretro_play::Key as PlayKey;
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::pump_events::EventLoopExtPumpEvents;
use winit::window::{Fullscreen, Window, WindowId};

const VERTEX: &str = "#version 330 core
out vec2 uv;
void main() {
    // one triangle that covers the whole viewport
    vec2 p = vec2(float((gl_VertexID << 1) & 2), float(gl_VertexID & 2));
    uv = vec2(p.x, 1.0 - p.y);
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}";

/// The picture, and the CRT and LCD looks: `cells` is how many window pixels one picture pixel
/// covers, and the last row (CRT) or last row and column (LCD) of every such cell is darker.
/// The levels are the ones the CPU looks in `vuretro-play` use (150/256 and 190/256).
const FRAGMENT: &str = "#version 330 core
uniform sampler2D picture;
uniform int look;        // 0 plain, 1 CRT, 2 LCD
uniform vec2 tex_size;   // the picture in pixels
uniform float cells;     // window pixels per picture pixel
in vec2 uv;
out vec4 colour;
void main() {
    vec3 c = texture(picture, uv).rgb;
    if (look != 0 && cells >= 2.0) {
        vec2 at = floor(fract(uv * tex_size) * cells);
        bool row = at.y == cells - 1.0;
        bool col = at.x == cells - 1.0;
        if (look == 1 && row) c *= 150.0 / 256.0;
        if (look == 2 && (row || col)) c *= 190.0 / 256.0;
    }
    colour = vec4(c, 1.0);
}";

/// What a window is asked to be.
#[derive(Clone)]
struct Spec {
    title: String,
    width: usize,
    height: usize,
    view: View,
    /// Where the window to replace was, in logical units.
    at: Option<(f64, f64)>,
    /// The OpenGL context a hardware-rendered core asked for.
    request: Option<GlRequest>,
}

/// Everything that exists once the window and its GL context do.
///
/// Field order is drop order: the GL objects go with the context, and the window must outlive
/// the surface drawn on it, so it comes last.
struct Gfx {
    gl: Rc<glow::Context>,
    display: glutin::display::Display,
    program: glow::Program,
    vao: glow::VertexArray,
    texture: glow::Texture,
    /// Size of the picture the texture holds now.
    tex_size: (usize, usize),
    view: View,
    look: Look,
    _menu: GameMenu,
    context: PossiblyCurrentContext,
    surface: Surface<WindowSurface>,
    window: Window,
}

/// winit's side: the state the event callbacks fill in.
struct App {
    wanted: Option<Spec>,
    gfx: Option<Gfx>,
    failed: Option<String>,
    down: HashSet<KeyCode>,
    pressed: HashSet<KeyCode>,
    closed: bool,
    active: bool,
}

impl App {
    fn make_window(&mut self, el: &ActiveEventLoop) {
        let Some(spec) = self.wanted.take() else {
            return;
        };
        match Gfx::new(el, &spec) {
            Ok(gfx) => {
                self.gfx = Some(gfx);
                self.closed = false;
                self.active = true;
            }
            Err(e) => self.failed = Some(format!("{e:#}")),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        self.make_window(el);
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        self.make_window(el);
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.closed = true,
            WindowEvent::Focused(on) => self.active = on,
            WindowEvent::Resized(size) => {
                if let (Some(g), Some(w), Some(h)) = (
                    self.gfx.as_ref(),
                    NonZeroU32::new(size.width),
                    NonZeroU32::new(size.height),
                ) {
                    g.surface.resize(&g.context, w, h);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if !event.repeat {
                                self.pressed.insert(code);
                            }
                            self.down.insert(code);
                        }
                        ElementState::Released => {
                            self.down.remove(&code);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl Gfx {
    fn new(el: &ActiveEventLoop, spec: &Spec) -> Result<Self> {
        let mut attrs = Window::default_attributes()
            .with_title(spec.title.clone())
            .with_inner_size(LogicalSize::new(spec.width as f64, spec.height as f64))
            .with_resizable(true);
        let view = spec.view;
        if view.fullscreen {
            attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        if let (Some((x, y)), false) = (spec.at, view.fullscreen) {
            attrs = attrs.with_position(LogicalPosition::new(x, y));
        }

        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attrs.clone()))
            .build(el, ConfigTemplateBuilder::new(), |configs| {
                // the config with the fewest samples: the picture needs none
                configs
                    .reduce(|a, b| {
                        if b.num_samples() < a.num_samples() {
                            b
                        } else {
                            a
                        }
                    })
                    .expect("the display offers at least one config")
            })
            .map_err(|e| anyhow!("no OpenGL display: {e}"))?;
        let window = match window {
            Some(w) => w,
            None => glutin_winit::finalize_window(el, attrs, &config)
                .map_err(|e| anyhow!("cannot open the window: {e}"))?,
        };
        let handle = window.window_handle()?.as_raw();
        let display = config.display();

        let context = Self::create_context(&display, &config, handle, spec.request)?;
        let surface_attrs = window.build_surface_attributes(Default::default())?;
        // SAFETY: the window outlives the surface (both live in `Gfx`, window dropped last).
        let surface = unsafe { display.create_window_surface(&config, &surface_attrs)? };
        let context = context.make_current(&surface)?;
        // the session paces itself; waiting for the display here would pace it twice
        let _ = surface.set_swap_interval(&context, SwapInterval::DontWait);

        // SAFETY: the context is current on this thread.
        let gl = Rc::new(unsafe {
            glow::Context::from_loader_function_cstr(|name| display.get_proc_address(name))
        });
        let (program, vao, texture) = unsafe { Self::init_gl(&gl)? };
        let menu = GameMenu::new(&window);
        Ok(Gfx {
            display,
            window,
            surface,
            context,
            gl,
            program,
            vao,
            texture,
            tex_size: (0, 0),
            view,
            look: Look::Plain,
            _menu: menu,
        })
    }

    /// The context a hardware-rendered core asked for, or OpenGL 3.3 core for the plain
    /// picture. When the exact version cannot be had, 3.3 core is tried before giving up.
    fn create_context(
        display: &glutin::display::Display,
        config: &Config,
        handle: raw_window_handle::RawWindowHandle,
        request: Option<GlRequest>,
    ) -> Result<glutin::context::NotCurrentContext> {
        let plain = GlRequest {
            major: 3,
            minor: 3,
            core: true,
        };
        let mut tries = vec![request.unwrap_or(plain)];
        if request.is_some_and(|r| r != plain) {
            tries.push(plain);
        }
        let mut last = String::new();
        for t in tries {
            let attrs = ContextAttributesBuilder::new()
                .with_profile(if t.core {
                    GlProfile::Core
                } else {
                    GlProfile::Compatibility
                })
                .with_context_api(ContextApi::OpenGl(Some(Version::new(
                    t.major as u8,
                    t.minor as u8,
                ))))
                .build(Some(handle));
            // SAFETY: `handle` belongs to the window made for this config.
            match unsafe { display.create_context(config, &attrs) } {
                Ok(c) => {
                    log::info!(
                        "OpenGL {}.{} {} context",
                        t.major,
                        t.minor,
                        if t.core { "core" } else { "compatibility" }
                    );
                    return Ok(c);
                }
                Err(e) => last = format!("{}.{}: {e}", t.major, t.minor),
            }
        }
        bail!("cannot make an OpenGL context ({last})")
    }

    /// The shader, an empty vertex array and the picture's texture.
    ///
    /// # Safety
    /// A context must be current on this thread.
    unsafe fn init_gl(
        gl: &glow::Context,
    ) -> Result<(glow::Program, glow::VertexArray, glow::Texture)> {
        let program = gl.create_program().map_err(|e| anyhow!(e))?;
        let mut shaders = Vec::new();
        for (kind, src) in [
            (glow::VERTEX_SHADER, VERTEX),
            (glow::FRAGMENT_SHADER, FRAGMENT),
        ] {
            let sh = gl.create_shader(kind).map_err(|e| anyhow!(e))?;
            gl.shader_source(sh, src);
            gl.compile_shader(sh);
            if !gl.get_shader_compile_status(sh) {
                bail!("shader: {}", gl.get_shader_info_log(sh));
            }
            gl.attach_shader(program, sh);
            shaders.push(sh);
        }
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            bail!("shader program: {}", gl.get_program_info_log(program));
        }
        for sh in shaders {
            gl.detach_shader(program, sh);
            gl.delete_shader(sh);
        }
        let vao = gl.create_vertex_array().map_err(|e| anyhow!(e))?;
        let texture = gl.create_texture().map_err(|e| anyhow!(e))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        for (p, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
            (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
        }
        Ok((program, vao, texture))
    }

    /// The window's drawing area in physical pixels.
    fn pixels(&self) -> (u32, u32) {
        let s = self.window.inner_size();
        (s.width, s.height)
    }

    /// Draw `frame` (0RGB) and show it.
    fn draw(&mut self, frame: &[u32], w: usize, h: usize) -> Result<()> {
        self.render(frame, w, h);
        self.surface.swap_buffers(&self.context)?;
        Ok(())
    }

    /// Draw `frame` into the back buffer.
    fn render(&mut self, frame: &[u32], w: usize, h: usize) {
        if w == 0 || h == 0 || frame.len() < w * h {
            return;
        }
        let (pw, ph) = self.pixels();
        let scale = self.window.scale_factor();
        // With a look the picture comes as the game made it: a whole number of window pixels
        // per picture pixel when asked, and the look is drawn in those cells. Without one the
        // session has already enlarged it by whole numbers if it was asked to.
        let (dest, cells) = if self.look == Look::Plain {
            let whole = self.view.integer.then_some(scale);
            (fit_rect((w, h), (pw as usize, ph as usize), whole), 1)
        } else if self.view.integer {
            whole_fit((w, h), (pw as usize, ph as usize))
        } else {
            // stretched: cells as many pixels as the CPU looks used for a picture this tall
            (
                fit_rect((w, h), (pw as usize, ph as usize), None),
                vuretro_play::crt::factor_for(h),
            )
        };
        // SAFETY: the context is current on this thread (it never leaves it); `frame` has
        // at least w*h pixels, which is what the upload reads.
        unsafe {
            let gl = &self.gl;
            // a hardware-rendered core leaves its own state behind: start from a known one
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            for cap in [
                glow::DEPTH_TEST,
                glow::STENCIL_TEST,
                glow::BLEND,
                glow::CULL_FACE,
                glow::SCISSOR_TEST,
            ] {
                gl.disable(cap);
            }
            gl.color_mask(true, true, true, true);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.active_texture(glow::TEXTURE0);
            gl.viewport(0, 0, pw as i32, ph as i32);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            // 0RGB as little-endian bytes is B, G, R, 0: the order GL calls BGRA
            let bytes = std::slice::from_raw_parts(frame.as_ptr() as *const u8, w * h * 4);
            if self.tex_size == (w, h) {
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    0,
                    0,
                    w as i32,
                    h as i32,
                    glow::BGRA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(bytes)),
                );
            } else {
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    w as i32,
                    h as i32,
                    0,
                    glow::BGRA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(bytes)),
                );
                self.tex_size = (w, h);
            }
            let Rect { x, y, w: rw, h: rh } = dest;
            gl.viewport(x, y, rw, rh);
            gl.use_program(Some(self.program));
            let at = |name| gl.get_uniform_location(self.program, name);
            gl.uniform_1_i32(
                at("look").as_ref(),
                match self.look {
                    Look::Plain => 0,
                    Look::Crt => 1,
                    Look::Lcd => 2,
                },
            );
            gl.uniform_2_f32(at("tex_size").as_ref(), w as f32, h as f32);
            gl.uniform_1_f32(at("cells").as_ref(), cells as f32);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}

/// The game window.
pub struct GlScreen {
    event_loop: EventLoop<()>,
    app: App,
    /// A core that draws with OpenGL, and the largest picture it makes.
    hw: Option<HwSetup>,
}

/// What a hardware-rendered core asked for.
#[derive(Clone, Copy)]
pub struct HwSetup {
    pub request: libretro_sys::retro_hw_render_callback,
    pub max: (usize, usize),
}

impl GlScreen {
    pub fn open(
        title: &str,
        base_w: usize,
        base_h: usize,
        factor: usize,
        view: View,
        hw: Option<HwSetup>,
    ) -> Result<Self> {
        let event_loop = EventLoop::new().map_err(|e| anyhow!("no event loop: {e}"))?;
        let mut me = GlScreen {
            event_loop,
            app: App {
                wanted: None,
                gfx: None,
                failed: None,
                down: HashSet::new(),
                pressed: HashSet::new(),
                closed: false,
                active: true,
            },
            hw,
        };
        me.app.wanted = Some(spec_for(
            title,
            base_w,
            base_h,
            factor,
            view,
            None,
            me.request(),
        ));
        me.wait_for_window()?;
        me.start_hw()?;
        // a black picture at once: the first frame of a game can take a while
        let black = vec![0u32; base_w * base_h];
        let _ = me.present(&black, base_w, base_h);
        Ok(me)
    }

    /// Pump events until the wanted window exists (or could not be made).
    fn wait_for_window(&mut self) -> Result<()> {
        for _ in 0..200 {
            self.pump();
            if let Some(e) = self.app.failed.take() {
                bail!(e);
            }
            if self.app.wanted.is_none() && self.app.gfx.is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        bail!("the window did not open")
    }

    /// Let winit deliver what is waiting. New key presses are those since the last pump.
    fn pump(&mut self) {
        self.app.pressed.clear();
        let _ = self
            .event_loop
            .pump_app_events(Some(Duration::ZERO), &mut self.app);
    }

    /// The OpenGL context to ask for, when a core draws with OpenGL.
    fn request(&self) -> Option<GlRequest> {
        self.hw.and_then(|h| hw::context_wanted(&h.request).ok())
    }

    /// Give a hardware-rendered core its framebuffer and tell it the context is ready.
    fn start_hw(&mut self) -> Result<()> {
        let (Some(setup), Some(g)) = (self.hw, self.app.gfx.as_ref()) else {
            return Ok(());
        };
        let display = g.display.clone();
        hw::start(
            g.gl.clone(),
            Box::new(move |name| display.get_proc_address(name)),
            &setup.request,
            setup.max,
        )
    }

    fn gfx(&self) -> Result<&Gfx> {
        self.app.gfx.as_ref().ok_or_else(|| anyhow!("no window"))
    }
}

fn spec_for(
    title: &str,
    base_w: usize,
    base_h: usize,
    factor: usize,
    view: View,
    at: Option<(f64, f64)>,
    request: Option<GlRequest>,
) -> Spec {
    // with whole-number scaling the session enlarges the picture itself, so the window is
    // the enlarged size; otherwise the window is `factor` times the core's picture
    Spec {
        title: title.to_string(),
        width: base_w * factor,
        height: base_h * factor,
        view,
        at,
        request,
    }
}

impl Drop for GlScreen {
    fn drop(&mut self) {
        // the core frees its GL objects while the context still exists
        hw::stop();
    }
}

impl Keyboard for GlScreen {
    fn key_down(&self, key: PlayKey) -> bool {
        to_code(key).is_some_and(|c| self.app.down.contains(&c))
    }
}

impl Screen for GlScreen {
    fn is_open(&self) -> bool {
        !self.app.closed && !crate::frontend::with(|fe| fe.shutdown)
    }
    fn key_pressed(&self, key: PlayKey) -> bool {
        to_code(key).is_some_and(|c| self.app.pressed.contains(&c))
    }
    fn menu_pressed(&mut self) -> Option<usize> {
        GameMenu::next_choice()
    }
    fn is_active(&mut self) -> bool {
        self.app.active
    }
    fn present(&mut self, frame: &[u32], w: usize, h: usize) -> Result<()> {
        self.pump();
        match self.app.gfx.as_mut() {
            Some(g) => g.draw(frame, w, h),
            None => Ok(()),
        }
    }
    fn idle(&mut self) {
        self.pump();
        // a window that shows nothing new still has to stay drawn after an expose
        if let Some(g) = self.app.gfx.as_ref() {
            g.window.request_redraw();
        }
    }
    fn set_title(&mut self, title: &str) {
        if let Some(g) = self.app.gfx.as_ref() {
            g.window.set_title(title);
        }
    }
    fn reopen(
        &mut self,
        title: &str,
        base_w: usize,
        base_h: usize,
        factor: usize,
        view: View,
    ) -> Result<()> {
        let at = self.app.gfx.as_ref().and_then(|g| {
            let sf = g.window.scale_factor();
            g.window
                .outer_position()
                .ok()
                .map(|p| (p.x as f64 / sf, p.y as f64 / sf))
        });
        // the core's GL objects belong to the old context: let it clean up while that one is
        // still current, then start again in the new one
        hw::stop();
        // the new window is made before the old one goes, so the app never has none
        let old = self.app.gfx.take();
        let request = self.request();
        self.app.wanted = Some(spec_for(title, base_w, base_h, factor, view, at, request));
        let made = self.wait_for_window();
        if made.is_err() {
            self.app.gfx = old;
            let _ = self.start_hw();
            return made;
        }
        drop(old);
        self.start_hw()
    }
    fn size(&self) -> (usize, usize) {
        match self.gfx() {
            Ok(g) => {
                let s = g
                    .window
                    .inner_size()
                    .to_logical::<f64>(g.window.scale_factor());
                (s.width as usize, s.height as usize)
            }
            Err(_) => (0, 0),
        }
    }
    fn view(&self) -> View {
        self.app.gfx.as_ref().map(|g| g.view).unwrap_or_default()
    }
    fn can_fullscreen(&self) -> bool {
        true
    }
    fn gpu_look(&self) -> bool {
        true
    }
    fn set_look(&mut self, look: Look) {
        if let Some(g) = self.app.gfx.as_mut() {
            g.look = look;
        }
    }
}

/// `vuretro-runner --window-selftest`: open a window, draw a red and a blue half, read the
/// pixels back from the GPU and check them. It needs a display, and on macOS the main thread,
/// so it is a flag of the runner and not a unit test.
pub fn selftest() -> Result<()> {
    let mut screen = GlScreen::open("vuretro self-test", 64, 32, 4, View::default(), None)?;
    // left half red, right half blue, in the 0RGB the core delivers
    let mut frame = vec![0x00FF_0000u32; 64 * 32];
    for row in frame.chunks_mut(64) {
        row[32..].fill(0x0000_00FF);
    }
    screen.pump();
    let g = screen
        .app
        .gfx
        .as_mut()
        .ok_or_else(|| anyhow!("no window"))?;
    g.render(&frame, 64, 32);
    let (pw, ph) = g.pixels();
    let at = |fx: f64, fy: f64| {
        let mut px = [0u8; 4];
        // SAFETY: the context is current; one RGBA pixel is read into `px`.
        unsafe {
            g.gl.read_pixels(
                (pw as f64 * fx) as i32,
                (ph as f64 * fy) as i32,
                1,
                1,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut px)),
            );
        }
        [px[0], px[1], px[2]]
    };
    let (left, right) = (at(0.25, 0.5), at(0.75, 0.5));
    g.surface.swap_buffers(&g.context)?;
    if left != [255, 0, 0] || right != [0, 0, 255] {
        bail!("wrong colours: left {left:?} (want red), right {right:?} (want blue)");
    }

    // the CRT and LCD looks: a white picture of 8x4 pixels, each covering 3 window pixels
    let white = vec![0x00FF_FFFFu32; 8 * 4];
    let line = |g: &mut Gfx, look: Look, x: i32, y: i32, w: i32, h: i32| -> Vec<u8> {
        g.look = look;
        g.render(&white, 8, 4);
        let mut px = vec![0u8; (w * h * 4) as usize];
        // SAFETY: the context is current; `px` holds w*h RGBA pixels.
        unsafe {
            g.gl.read_pixels(
                x,
                y,
                w,
                h,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut px)),
            );
        }
        px.chunks(4).map(|p| p[0]).collect()
    };
    let (pw, ph) = (pw as i32, ph as i32);
    // CRT: along a column, one row in three is darker (150/256 of white), the rest white
    let column = line(g, Look::Crt, pw / 2, 0, 1, ph);
    let dark = column.iter().filter(|&&v| v < 200).count() as f64 / column.len() as f64;
    let dim = column
        .iter()
        .copied()
        .filter(|&v| v < 200)
        .max()
        .unwrap_or(0);
    if !(0.28..=0.38).contains(&dark) || !(145..=155).contains(&dim) {
        bail!("CRT look: {dark:.2} of the rows dark (want about a third), dark level {dim} (want 149)");
    }
    // LCD: along a row that is not a grid row, one pixel in three is darker (190/256)
    let row = line(g, Look::Lcd, 0, ph - 1, pw, 1);
    let dark = row.iter().filter(|&&v| v < 220).count() as f64 / row.len() as f64;
    let dim = row.iter().copied().filter(|&v| v < 220).max().unwrap_or(0);
    if !(0.28..=0.38).contains(&dark) || !(186..=194).contains(&dim) {
        bail!("LCD look: {dark:.2} of the columns dark (want about a third), dark level {dim} (want 189)");
    }
    g.look = Look::Plain;
    let renderer = unsafe { g.gl.get_parameter_string(glow::RENDERER) };
    let version = unsafe { g.gl.get_parameter_string(glow::VERSION) };
    println!(
        "ok: {renderer}, OpenGL {version}, window {:?}",
        screen.size()
    );
    Ok(())
}

/// `vuretro-runner --hw-selftest <test core> <rom>`: load the test core in its OpenGL mode
/// (`VURETRO_TEST_HW`), run it in a real window and check that the picture it drew with
/// OpenGL comes back right way up: blue on top, red below.
pub fn selftest_hw(core_path: &std::path::Path, rom: &std::path::Path) -> Result<()> {
    use std::sync::{Arc, Mutex};
    std::env::set_var("VURETRO_TEST_HW", "1");
    crate::frontend::init(
        "/nonexistent",
        "/nonexistent",
        Arc::new(Mutex::new(Default::default())),
        Default::default(),
    );
    crate::frontend::with(|fe| fe.hw_allowed = true);
    let mut core = crate::core::Core::load(core_path)?;
    let av = core.load_game(rom)?;
    let request = crate::frontend::with(|fe| fe.hw)
        .ok_or_else(|| anyhow!("the core did not ask for hardware rendering"))?;
    let (w, h) = (
        av.geometry.base_width as usize,
        av.geometry.base_height as usize,
    );
    let setup = HwSetup {
        request,
        max: (w, h),
    };
    let screen = GlScreen::open(
        "vuretro hw self-test",
        w,
        h,
        1,
        View::default(),
        Some(setup),
    )?;
    if !hw::active() {
        bail!("the core's framebuffer was not set up");
    }
    for _ in 0..3 {
        core.run();
    }
    let (frame, fw, fh) = crate::frontend::with(|fe| (fe.frame.clone(), fe.width, fe.height));
    if (fw, fh) != (w, h) {
        bail!("the picture is {fw}x{fh}, expected {w}x{h}");
    }
    let (top, bottom) = (frame[fw * 2 + 5], frame[fw * (fh - 3) + 5]);
    drop(screen);
    if top != 0x0000_00FF || bottom != 0x00FF_0000 {
        bail!("wrong picture: top {top:#08x} (want blue 0x0000ff), bottom {bottom:#08x} (want red 0xff0000)");
    }
    println!("ok: the core drew with OpenGL and the picture came back right way up");
    Ok(())
}
