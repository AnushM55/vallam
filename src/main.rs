mod swc;

use std::os::raw::c_void;
use std::process::Command;
use std::ptr;

use swc::*;

// --- State ---

struct Screen {
    swc: *mut swc_screen,
    windows: Vec<*mut Window>,
    num_windows: usize,
}

struct Window {
    swc: *mut swc_window,
    screen: *mut Screen,
    fullscreen: bool,
}

static mut DISPLAY: *mut wl_display = ptr::null_mut();
static mut FOCUSED_WINDOW: *mut Window = ptr::null_mut();
static mut ACTIVE_SCREEN: *mut Screen = ptr::null_mut();

// --- Layout ---

unsafe fn arrange(screen: *mut Screen) {
    let s = &*screen;
    if s.num_windows == 0 {
        return;
    }

    let sg = &(*s.swc).usable_geometry;
    let nc = (s.num_windows as f64).sqrt().ceil() as u32;
    let nr = (s.num_windows as u32) / nc + 1;

    let mut col: u32 = 0;
    let mut row: u32 = 0;
    let mut remaining_in_col = nr;

    for win_ptr in &s.windows {
        let w = &**win_ptr;
        if w.fullscreen {
            let geo = swc_rectangle {
                x: sg.x,
                y: sg.y,
                width: sg.width,
                height: sg.height,
            };
            swc_window_set_geometry(w.swc, &geo);
            continue;
        }

        let geometry = swc_rectangle {
            x: sg.x + (sg.width * col / nc) as i32,
            y: sg.y + (sg.height * row / nr) as i32,
            width: sg.width / nc,
            height: sg.height / nr,
        };
        swc_window_set_geometry(w.swc, &geometry);

        row += 1;
        remaining_in_col -= 1;
        if remaining_in_col == 0 {
            col += 1;
            remaining_in_col = nr;
            row = 0;
        }
    }
}

// --- Screen helpers ---

unsafe fn screen_add_window(screen: *mut Screen, window: *mut Window) {
    let s = &mut *screen;
    (*window).screen = screen;
    s.windows.push(window);
    s.num_windows += 1;
    swc_window_show((*window).swc);
    arrange(screen);
}

unsafe fn screen_remove_window(screen: *mut Screen, window: *mut Window) {
    let s = &mut *screen;
    (*window).screen = ptr::null_mut();
    s.windows.retain(|w| *w != window);
    s.num_windows -= 1;
    swc_window_hide((*window).swc);
    arrange(screen);
}

// --- Focus ---

unsafe fn focus(window: *mut Window) {
    if !FOCUSED_WINDOW.is_null() {
        swc_window_set_border((*FOCUSED_WINDOW).swc, 0xff888888, 1, 0, 0);
    }

    if !window.is_null() {
        swc_window_set_border((*window).swc, 0xff333388, 1, 0, 0);
        swc_window_focus((*window).swc);
    } else {
        swc_window_focus(ptr::null_mut());
    }

    FOCUSED_WINDOW = window;
}

unsafe fn focus_next(screen: *mut Screen, window: *mut Window) {
    let s = &*screen;
    if s.num_windows <= 1 {
        return;
    }

    for (i, &w) in s.windows.iter().enumerate() {
        if w == window {
            let next = if i + 1 < s.windows.len() {
                s.windows[i + 1]
            } else {
                s.windows[0]
            };
            focus(next);
            return;
        }
    }
}

unsafe fn focus_prev(screen: *mut Screen, window: *mut Window) {
    let s = &*screen;
    if s.num_windows <= 1 {
        return;
    }

    for (i, &w) in s.windows.iter().enumerate() {
        if w == window {
            let prev = if i > 0 {
                s.windows[i - 1]
            } else {
                *s.windows.last().unwrap()
            };
            focus(prev);
            return;
        }
    }
}

// --- Callbacks ---

unsafe extern "C" fn screen_usable_geometry_changed(data: *mut c_void) {
    let screen = data as *mut Screen;
    arrange(screen);
}

unsafe extern "C" fn screen_entered(data: *mut c_void) {
    ACTIVE_SCREEN = data as *mut Screen;
}

static SCREEN_HANDLER: swc_screen_handler = swc_screen_handler {
    destroy: None,
    geometry_changed: None,
    usable_geometry_changed: Some(screen_usable_geometry_changed),
    entered: Some(screen_entered),
};

unsafe extern "C" fn window_destroy(data: *mut c_void) {
    let window = data as *mut Window;
    let screen = (*window).screen;

    if FOCUSED_WINDOW == window && !screen.is_null() {
        let s = &*screen;
        let mut next_focus: *mut Window = ptr::null_mut();

        for (i, &w) in s.windows.iter().enumerate() {
            if w == window {
                if i + 1 < s.windows.len() && s.windows[i + 1] != window {
                    next_focus = s.windows[i + 1];
                } else if i > 0 && s.windows[i - 1] != window {
                    next_focus = s.windows[i - 1];
                }
                break;
            }
        }

        focus(next_focus);
    }

    if !screen.is_null() {
        screen_remove_window(screen, window);
    }

    drop(Box::from_raw(window));
}

unsafe extern "C" fn window_entered(data: *mut c_void) {
    let window = data as *mut Window;
    focus(window);
}

static WINDOW_HANDLER: swc_window_handler = swc_window_handler {
    destroy: Some(window_destroy),
    title_changed: None,
    app_id_changed: None,
    parent_changed: None,
    entered: Some(window_entered),
    move_: None,
    resize: None,
};

// --- Manager callbacks ---

unsafe extern "C" fn new_screen(swc: *mut swc_screen) {
    let screen = Box::into_raw(Box::new(Screen {
        swc,
        windows: Vec::new(),
        num_windows: 0,
    }));

    swc_screen_set_handler(swc, &SCREEN_HANDLER, screen as *mut c_void);
    ACTIVE_SCREEN = screen;
}

unsafe extern "C" fn new_window(swc: *mut swc_window) {
    let window = Box::into_raw(Box::new(Window {
        swc,
        screen: ptr::null_mut(),
        fullscreen: false,
    }));

    swc_window_set_handler(swc, &WINDOW_HANDLER, window as *mut c_void);
    swc_window_set_tiled(swc);

    if !ACTIVE_SCREEN.is_null() {
        screen_add_window(ACTIVE_SCREEN, window);
    }
    focus(window);
}

static MANAGER: swc_manager = swc_manager {
    new_screen: Some(new_screen),
    new_window: Some(new_window),
    new_device: None,
    activate: None,
    deactivate: None,
};

// --- Keybinding handlers ---

unsafe extern "C" fn spawn_st(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    Command::new("kitty").spawn().ok();
}

unsafe extern "C" fn spawn_dmenu(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    Command::new("dmenu_run-wl").spawn().ok();
}

unsafe extern "C" fn quit(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    wl_display_terminate(DISPLAY);
}

unsafe extern "C" fn close_window(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    if !FOCUSED_WINDOW.is_null() {
        swc_window_close((*FOCUSED_WINDOW).swc);
    }
}

unsafe extern "C" fn focus_next_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    if !FOCUSED_WINDOW.is_null() && !(*FOCUSED_WINDOW).screen.is_null() {
        focus_next((*FOCUSED_WINDOW).screen, FOCUSED_WINDOW);
    }
}

unsafe extern "C" fn focus_prev_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    if !FOCUSED_WINDOW.is_null() && !(*FOCUSED_WINDOW).screen.is_null() {
        focus_prev((*FOCUSED_WINDOW).screen, FOCUSED_WINDOW);
    }
}

unsafe extern "C" fn toggle_fullscreen(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    if FOCUSED_WINDOW.is_null() {
        return;
    }
    let w = &mut *FOCUSED_WINDOW;
    if w.fullscreen {
        w.fullscreen = false;
        swc_window_set_tiled(w.swc);
    } else {
        w.fullscreen = true;
        if !w.screen.is_null() {
            swc_window_set_fullscreen(w.swc, (*w.screen).swc);
        }
    }
    if !w.screen.is_null() {
        arrange(w.screen);
    }
}

unsafe extern "C" fn zoom_in(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    let zoom = swc_get_zoom();
    swc_set_zoom((zoom + 0.1).min(5.0));
}

unsafe extern "C" fn zoom_out(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    let zoom = swc_get_zoom();
    swc_set_zoom((zoom - 0.1).max(0.1));
}

unsafe extern "C" fn zoom_reset(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    swc_set_zoom(1.0);
}

unsafe extern "C" fn stack_window_up(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    if !FOCUSED_WINDOW.is_null() {
        swc_window_stack((*FOCUSED_WINDOW).swc, -1);
    }
}

unsafe extern "C" fn stack_window_down(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }
    if !FOCUSED_WINDOW.is_null() {
        swc_window_stack((*FOCUSED_WINDOW).swc, 1);
    }
}

// --- Main ---

fn main() {
    unsafe {
        DISPLAY = wl_display_create();
        if DISPLAY.is_null() {
            eprintln!("Failed to create wayland display");
            std::process::exit(1);
        }

        let socket = wl_display_add_socket_auto(DISPLAY);
        if socket.is_null() {
            eprintln!("Failed to add wayland socket");
            std::process::exit(1);
        }

        let socket_str = std::ffi::CStr::from_ptr(socket);
        std::env::set_var("WAYLAND_DISPLAY", socket_str.to_str().unwrap());

        // Force wld to use dumb DRM driver (needed for GPUs not in wld's PCI ID table)
        std::env::set_var("WLD_DRM_DUMB", "1");

        if !swc_initialize(DISPLAY, ptr::null_mut(), &MANAGER) {
            eprintln!("Failed to initialize swc");
            std::process::exit(1);
        }

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_Return,
            Some(spawn_st),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_d,
            Some(spawn_dmenu),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_q,
            Some(quit),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO | SWC_MOD_SHIFT,
            XKB_KEY_q,
            Some(close_window),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_j,
            Some(focus_next_handler),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_k,
            Some(focus_prev_handler),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_f,
            Some(toggle_fullscreen),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO | SWC_MOD_SHIFT,
            XKB_KEY_j,
            Some(stack_window_up),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO | SWC_MOD_SHIFT,
            XKB_KEY_k,
            Some(stack_window_down),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_equal,
            Some(zoom_in),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_minus,
            Some(zoom_out),
            ptr::null_mut(),
        );

        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            SWC_MOD_LOGO,
            XKB_KEY_0,
            Some(zoom_reset),
            ptr::null_mut(),
        );

        println!("wm15: running on {}", socket_str.to_str().unwrap());
        wl_display_run(DISPLAY);
        wl_display_destroy(DISPLAY);
    }
}
