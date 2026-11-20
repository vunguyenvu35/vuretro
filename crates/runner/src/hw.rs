//! Hardware rendering: a core that draws with OpenGL (Nintendo 64, PSP, GameCube, PS2, 3DS)
//! does not hand over pixels. It draws into a framebuffer the frontend made, in the
//! frontend's GL context, and says "the frame is in the framebuffer" (`RETRO_HW_FRAME_BUFFER_VALID`).
//!
//! The picture is read back into an ordinary 0RGB frame, so everything the session does with
//! a frame (filters, screenshots, previews) works the same for these cores. Reading back
//! costs some time per frame; drawing the texture straight to the window is the next step.
//!
//! The C callbacks carry no user pointer, and everything here runs on the main thread (the
//! thread that owns the GL context), so the state is thread-local.

use anyhow::{anyhow, bail, Result};
use glow::HasContext;
use libretro_sys::*;
use std::cell::RefCell;
use std::ffi::{c_char, c_void, CStr};
use std::rc::Rc;

/// Looks a GL function up by name in the window's context.
pub type Loader = Box<dyn Fn(&CStr) -> *const c_void>;

struct Bridge {
    gl: Rc<glow::Context>,
    loader: Loader,
    fbo: glow::Framebuffer,
    colour: glow::Texture,
    depth: Option<glow::Renderbuffer>,
    /// Largest picture the framebuffer holds.
    size: (usize, usize),
    /// The core draws with the origin at the bottom left, so rows come out upside down.
    flip: bool,
    destroy: retro_hw_context_reset_t,
}

thread_local! {
    static BRIDGE: RefCell<Option<Bridge>> = const { RefCell::new(None) };
}

/// The framebuffer id the core draws into (`get_current_framebuffer`).
unsafe extern "C" fn current_framebuffer() -> usize {
    BRIDGE.with(|b| b.borrow().as_ref().map_or(0, |b| native_id(b.fbo)))
}

/// A GL function by name (`get_proc_address`).
unsafe extern "C" fn proc_address(sym: *const c_char) -> retro_proc_address_t {
    if sym.is_null() {
        return None;
    }
    let name = CStr::from_ptr(sym);
    let ptr = BRIDGE.with(|b| {
        b.borrow()
            .as_ref()
            .map_or(std::ptr::null(), |b| (b.loader)(name))
    });
    if ptr.is_null() {
        None
    } else {
        // SAFETY: a non-null address from the GL loader is a function; the core casts it to
        // the right signature before calling it.
        Some(std::mem::transmute::<*const c_void, unsafe extern "C" fn()>(ptr))
    }
}

/// The number GL gave the framebuffer.
fn native_id(fbo: glow::Framebuffer) -> usize {
    // `glow` hides the id in a type that is a plain number on desktop GL.
    fbo.0.get() as usize
}

/// Fill in the two functions the frontend gives the core.
pub fn give_functions(cb: &mut retro_hw_render_callback) {
    cb.get_current_framebuffer = Some(current_framebuffer);
    cb.get_proc_address = Some(proc_address);
}

/// Which OpenGL context a core asks for, or why we cannot give it.
pub fn context_wanted(cb: &retro_hw_render_callback) -> Result<GlRequest> {
    match cb.context_type {
        RETRO_HW_CONTEXT_OPENGL_CORE => Ok(GlRequest {
            major: cb.version_major.max(3),
            minor: if cb.version_major < 3 {
                2
            } else {
                cb.version_minor
            },
            core: true,
        }),
        // a "compatibility" context: fixed-function GL, which not every system can give
        RETRO_HW_CONTEXT_OPENGL => Ok(GlRequest {
            major: cb.version_major.max(2),
            minor: cb.version_minor,
            core: false,
        }),
        other => bail!("the core wants a type of renderer vuretro does not have yet ({other})"),
    }
}

/// The OpenGL version and profile of the window's context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlRequest {
    pub major: u32,
    pub minor: u32,
    pub core: bool,
}

/// Make the framebuffer the core draws into, then tell the core its context is ready.
/// The window's GL context must be current.
pub fn start(
    gl: Rc<glow::Context>,
    loader: Loader,
    cb: &retro_hw_render_callback,
    max: (usize, usize),
) -> Result<()> {
    stop();
    let (w, h) = (max.0.max(1), max.1.max(1));
    // SAFETY: the window's context is current on this thread.
    let (fbo, colour, depth) = unsafe {
        let colour = gl.create_texture().map_err(|e| anyhow!(e))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(colour));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            w as i32,
            h as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        // the window samples this texture: without a mipmap chain the default filter would
        // leave it incomplete (black)
        for (p, v) in [
            (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
            (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
        }
        let fbo = gl.create_framebuffer().map_err(|e| anyhow!(e))?;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(colour),
            0,
        );
        let depth = if cb.depth || cb.stencil {
            let rb = gl.create_renderbuffer().map_err(|e| anyhow!(e))?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(rb));
            let (format, attachment) = if cb.stencil {
                (glow::DEPTH24_STENCIL8, glow::DEPTH_STENCIL_ATTACHMENT)
            } else {
                (glow::DEPTH_COMPONENT24, glow::DEPTH_ATTACHMENT)
            };
            gl.renderbuffer_storage(glow::RENDERBUFFER, format, w as i32, h as i32);
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                attachment,
                glow::RENDERBUFFER,
                Some(rb),
            );
            Some(rb)
        } else {
            None
        };
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        if status != glow::FRAMEBUFFER_COMPLETE {
            bail!("the framebuffer for the core is not complete (status {status:#x})");
        }
        (fbo, colour, depth)
    };
    BRIDGE.with(|b| {
        *b.borrow_mut() = Some(Bridge {
            gl,
            loader,
            fbo,
            colour,
            depth,
            size: (w, h),
            flip: cb.bottom_left_origin,
            destroy: cb.context_destroy,
        })
    });
    // the core asks for functions and the framebuffer id while this runs: no borrow is held
    if let Some(reset) = cb.context_reset {
        // SAFETY: the function is the core's own, and its context is ready.
        unsafe { reset() };
    }
    // the window can show the core's texture as it is: frames need not be read back
    crate::frontend::with(|fe| fe.hw_direct = true);
    Ok(())
}

/// Tell the core its context is going away, then free the framebuffer. Needs the window's GL
/// context to be current. Does nothing when there is no hardware rendering.
pub fn stop() {
    // the bridge stays in place while the core cleans up: it may look up GL functions to do it
    let destroy = BRIDGE.with(|b| b.borrow().as_ref().and_then(|b| b.destroy));
    if let Some(destroy) = destroy {
        // SAFETY: the core's own function, and its context still exists.
        unsafe { destroy() };
    }
    let Some(bridge) = BRIDGE.with(|b| b.borrow_mut().take()) else {
        return;
    };
    // a picture that was left on the card goes with it
    crate::frontend::with(|fe| {
        fe.hw_direct = false;
        fe.gpu_frame = None;
    });
    // SAFETY: the context is current and these are the objects made in `start`.
    unsafe {
        bridge.gl.delete_framebuffer(bridge.fbo);
        bridge.gl.delete_texture(bridge.colour);
        if let Some(rb) = bridge.depth {
            bridge.gl.delete_renderbuffer(rb);
        }
    }
}

/// The core's own picture, as the window can draw it.
pub struct Picture {
    pub texture: glow::Texture,
    /// The whole texture.
    pub size: (usize, usize),
    /// The part the picture fills, from the bottom left corner of the texture.
    pub size_used: (usize, usize),
    /// The rows are upside down in the texture.
    pub flip: bool,
}

/// The `w` x `h` picture the core drew last, as a texture (the sizes are cut to what the
/// framebuffer holds). `None` when there is no hardware rendering.
pub fn picture(w: usize, h: usize) -> Option<Picture> {
    BRIDGE.with(|cell| {
        let guard = cell.borrow();
        let b = guard.as_ref()?;
        let used = (w.min(b.size.0), h.min(b.size.1));
        (used.0 > 0 && used.1 > 0).then_some(Picture {
            texture: b.colour,
            size: b.size,
            size_used: used,
            flip: b.flip,
        })
    })
}

/// `w` x `h` cut to what the core's framebuffer can hold.
pub fn clamp(w: usize, h: usize) -> (usize, usize) {
    BRIDGE.with(|cell| {
        cell.borrow()
            .as_ref()
            .map_or((w, h), |b| (w.min(b.size.0), h.min(b.size.1)))
    })
}

/// Is a hardware-rendered game running?
pub fn active() -> bool {
    BRIDGE.with(|b| b.borrow().is_some())
}

/// Read the `w` x `h` picture the core just drew as 0RGB pixels, top row first (the size comes
/// back too: it cannot be larger than the framebuffer).
pub fn read_frame(w: usize, h: usize) -> Option<(Vec<u32>, usize, usize)> {
    BRIDGE.with(|cell| {
        let guard = cell.borrow();
        let b = guard.as_ref()?;
        let (w, h) = (w.min(b.size.0), h.min(b.size.1));
        if w == 0 || h == 0 {
            return None;
        }
        let mut px = vec![0u32; w * h];
        // SAFETY: the context is current; `px` holds exactly w*h four-byte pixels, which is
        // what a BGRA/UNSIGNED_BYTE read of w x h pixels writes (pack alignment 4).
        unsafe {
            let gl = &b.gl;
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 4);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(b.fbo));
            let bytes = std::slice::from_raw_parts_mut(px.as_mut_ptr() as *mut u8, w * h * 4);
            gl.read_pixels(
                0,
                0,
                w as i32,
                h as i32,
                glow::BGRA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(bytes)),
            );
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
        }
        finish_rows(&mut px, w, h, b.flip);
        Some((px, w, h))
    })
}

/// Drop the alpha byte, and turn the rows over when the core drew from the bottom.
fn finish_rows(px: &mut [u32], w: usize, h: usize, flip: bool) {
    for p in px.iter_mut() {
        *p &= 0x00FF_FFFF;
    }
    if flip {
        for y in 0..h / 2 {
            let (top, bottom) = px.split_at_mut((h - 1 - y) * w);
            top[y * w..(y + 1) * w].swap_with_slice(&mut bottom[..w]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_uint;

    fn cb(kind: c_uint, major: c_uint, minor: c_uint) -> retro_hw_render_callback {
        retro_hw_render_callback {
            context_type: kind,
            context_reset: None,
            get_current_framebuffer: None,
            get_proc_address: None,
            depth: false,
            stencil: false,
            bottom_left_origin: false,
            version_major: major,
            version_minor: minor,
            cache_context: false,
            context_destroy: None,
            debug_context: false,
        }
    }

    #[test]
    fn a_core_context_is_at_least_3_2_and_keeps_a_higher_version() {
        let r = context_wanted(&cb(RETRO_HW_CONTEXT_OPENGL_CORE, 3, 3)).unwrap();
        assert_eq!(
            r,
            GlRequest {
                major: 3,
                minor: 3,
                core: true
            }
        );
        let r = context_wanted(&cb(RETRO_HW_CONTEXT_OPENGL_CORE, 0, 0)).unwrap();
        assert_eq!((r.major, r.minor, r.core), (3, 2, true));
        let r = context_wanted(&cb(RETRO_HW_CONTEXT_OPENGL_CORE, 4, 5)).unwrap();
        assert_eq!((r.major, r.minor), (4, 5));
    }

    #[test]
    fn a_compatibility_context_is_not_a_core_one() {
        let r = context_wanted(&cb(RETRO_HW_CONTEXT_OPENGL, 3, 0)).unwrap();
        assert_eq!(
            r,
            GlRequest {
                major: 3,
                minor: 0,
                core: false
            }
        );
    }

    #[test]
    fn other_renderers_are_refused_with_a_reason() {
        for kind in [RETRO_HW_CONTEXT_VULKAN, RETRO_HW_CONTEXT_OPENGLES2, 7, 99] {
            assert!(context_wanted(&cb(kind, 0, 0)).is_err(), "type {kind}");
        }
    }

    #[test]
    fn rows_are_turned_over_and_alpha_is_dropped() {
        // 2 wide, 3 tall, top row first as drawn from the bottom: rows A B C
        let mut px = vec![
            0xFF00_0001,
            0xFF00_0002,
            0xFF00_0003,
            0xFF00_0004,
            0xFF00_0005,
            0xFF00_0006,
        ];
        finish_rows(&mut px, 2, 3, true);
        assert_eq!(px, [5, 6, 3, 4, 1, 2]);
        let mut same = vec![0xFF00_0001, 0xFF00_0002];
        finish_rows(&mut same, 2, 1, false);
        assert_eq!(same, [1, 2]);
    }

    #[test]
    fn asking_for_the_framebuffer_without_hardware_rendering_gives_zero() {
        assert!(!active());
        assert_eq!(unsafe { current_framebuffer() }, 0);
        assert!(unsafe { proc_address(c"glClear".as_ptr()) }.is_none());
        assert!(read_frame(8, 8).is_none());
    }
}
