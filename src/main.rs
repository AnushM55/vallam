mod swc;

use std::cell::{Cell, RefCell};
use std::ffi::CStr;
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
    saved_geometry: swc_rectangle,
    has_saved_geometry: bool,
}

thread_local! {
    static DISPLAY: Cell<*mut wl_display> = const { Cell::new(ptr::null_mut()) };
    static FOCUSED_WINDOW: Cell<*mut Window> = const { Cell::new(ptr::null_mut()) };
    static ACTIVE_SCREEN: Cell<*mut Screen> = const { Cell::new(ptr::null_mut()) };
    static WINDOW_SPAWN_INDEX: Cell<u32> = const { Cell::new(0) };
    static MOVING_WINDOW: Cell<*mut Window> = const { Cell::new(ptr::null_mut()) };
    static RESIZING_WINDOW: Cell<*mut Window> = const { Cell::new(ptr::null_mut()) };
    static FOCUS_HISTORY: RefCell<Vec<*mut Window>> = const { RefCell::new(Vec::new()) };
}

fn get_display() -> *mut wl_display {
    DISPLAY.with(|d| d.get())
}

fn set_display(display: *mut wl_display) {
    DISPLAY.with(|d| d.set(display));
}

fn get_focused_window() -> *mut Window {
    FOCUSED_WINDOW.with(|w| w.get())
}

fn set_focused_window(window: *mut Window) {
    FOCUSED_WINDOW.with(|w| w.set(window));
}

fn get_active_screen() -> *mut Screen {
    ACTIVE_SCREEN.with(|s| s.get())
}

fn set_active_screen(screen: *mut Screen) {
    ACTIVE_SCREEN.with(|s| s.set(screen));
}

fn get_moving_window() -> *mut Window {
    MOVING_WINDOW.with(|w| w.get())
}

fn set_moving_window(window: *mut Window) {
    MOVING_WINDOW.with(|w| w.set(window));
}

fn get_resizing_window() -> *mut Window {
    RESIZING_WINDOW.with(|w| w.get())
}

fn set_resizing_window(window: *mut Window) {
    RESIZING_WINDOW.with(|w| w.set(window));
}

fn note_window_focus(window: *mut Window) {
    if window.is_null() {
        return;
    }

    FOCUS_HISTORY.with(|history| {
        let mut history = history.borrow_mut();
        history.retain(|&w| w != window);
        history.push(window);
    });
}

fn remove_window_from_focus_history(window: *mut Window) {
    if window.is_null() {
        return;
    }

    FOCUS_HISTORY.with(|history| {
        history.borrow_mut().retain(|&w| w != window);
    });
}

fn last_active_window_on_screen(screen: *mut Screen, excluding: *mut Window) -> *mut Window {
    if screen.is_null() {
        return ptr::null_mut();
    }

    let from_history = FOCUS_HISTORY.with(|history| {
        let history = history.borrow();

        history.iter().rev().copied().find(|&candidate| {
            if candidate.is_null() || candidate == excluding {
                return false;
            }

            unsafe { (*candidate).screen == screen }
        })
    });

    if let Some(window) = from_history {
        return window;
    }

    unsafe {
        for &candidate in (*screen).windows.iter().rev() {
            if !candidate.is_null() && candidate != excluding {
                return candidate;
            }
        }
    }

    ptr::null_mut()
}

fn next_spawn_index() -> u32 {
    WINDOW_SPAWN_INDEX.with(|idx| {
        let current = idx.get();
        idx.set(current.wrapping_add(1));
        current
    })
}

// --- Floating placement helpers ---

fn update_fullscreen_windows(screen: *mut Screen) {
    unsafe {
        let s = &*screen;
        let sg = &(*s.swc).usable_geometry;

        for &win_ptr in &s.windows {
            let w = &*win_ptr;
            if w.fullscreen {
                let geo = swc_rectangle {
                    x: sg.x,
                    y: sg.y,
                    width: sg.width,
                    height: sg.height,
                };
                swc_window_set_geometry(w.swc, &geo);
            }
        }
    }
}

fn place_window(screen: *mut Screen, window: *mut Window) {
    if screen.is_null() || window.is_null() {
        return;
    }

    unsafe {
        let usable = &(*(*screen).swc).usable_geometry;

        let width = (usable.width.saturating_mul(3) / 4)
            .max(480)
            .min(usable.width);
        let height = (usable.height.saturating_mul(3) / 4)
            .max(320)
            .min(usable.height);

        let step = 28u32;
        let cycle = 8u32;
        let offset = (next_spawn_index() % cycle) as i32 * step as i32;

        let focused = get_focused_window();
        if !focused.is_null() && focused != window && (*focused).screen == screen {
            let fw = &*focused;
            if !fw.fullscreen {
                let mut active_geo = swc_rectangle {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 0,
                };

                if swc_window_get_geometry(fw.swc, &mut active_geo) {
                    let gap = 36;
                    let x = active_geo.x + active_geo.width as i32 + gap + offset;
                    let y = active_geo.y + offset / 2;

                    swc_window_set_geometry(
                        (*window).swc,
                        &swc_rectangle {
                            x,
                            y,
                            width,
                            height,
                        },
                    );
                    return;
                }
            }
        }

        let base_x = usable.x + ((usable.width - width) / 2) as i32;
        let base_y = usable.y + ((usable.height - height) / 2) as i32;

        swc_window_set_geometry(
            (*window).swc,
            &swc_rectangle {
                x: base_x + offset,
                y: base_y + offset,
                width,
                height,
            },
        );
    }
}

// --- Screen helpers ---

fn screen_add_window(screen: *mut Screen, window: *mut Window) {
    unsafe {
        let s = &mut *screen;
        (*window).screen = screen;

        if !s.windows.contains(&window) {
            s.windows.push(window);
            s.num_windows = s.windows.len();

            swc_window_set_stacked((*window).swc);
            place_window(screen, window);
            swc_window_show((*window).swc);
            return;
        }

        s.num_windows = s.windows.len();
    }
}

fn screen_remove_window(screen: *mut Screen, window: *mut Window) {
    unsafe {
        let s = &mut *screen;
        (*window).screen = ptr::null_mut();

        let before = s.windows.len();
        s.windows.retain(|w| *w != window);

        if s.windows.len() != before {
            swc_window_hide((*window).swc);
        }

        s.num_windows = s.windows.len();
    }
}

fn find_window_by_swc(screen: *mut Screen, swc_window_ptr: *mut swc_window) -> *mut Window {
    if screen.is_null() || swc_window_ptr.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        for &win_ptr in &(*screen).windows {
            if !win_ptr.is_null() && (*win_ptr).swc == swc_window_ptr {
                return win_ptr;
            }
        }
    }

    ptr::null_mut()
}

fn window_at_cursor() -> *mut Window {
    let screen = get_active_screen();
    if screen.is_null() {
        return ptr::null_mut();
    }

    let (mut x, mut y) = (0i32, 0i32);
    if !unsafe { swc_cursor_position(&mut x, &mut y) } {
        return ptr::null_mut();
    }

    let swc_win = unsafe { swc_window_at(x, y) };
    find_window_by_swc(screen, swc_win)
}

fn raise_window(window: *mut Window) {
    if window.is_null() {
        return;
    }

    unsafe {
        let screen = (*window).screen;
        if screen.is_null() {
            return;
        }

        let steps = (*screen).windows.len();
        for _ in 0..steps {
            swc_window_stack((*window).swc, -1);
        }
    }
}

const MOVE_STEP: i32 = 64;
const RESIZE_STEP: i32 = 48;
const MIN_WINDOW_WIDTH: i32 = 120;
const MIN_WINDOW_HEIGHT: i32 = 90;
const DEFAULT_BACKGROUND_COLOR: u32 = 0xff101418;

fn parse_color_argb(value: &str) -> Option<u32> {
    let cleaned = value.trim().trim_start_matches('#');

    match cleaned.len() {
        6 => u32::from_str_radix(cleaned, 16)
            .ok()
            .map(|rgb| 0xff000000 | rgb),
        8 => u32::from_str_radix(cleaned, 16).ok(),
        _ => None,
    }
}

fn configured_background_color() -> u32 {
    std::env::var("WM15_BG")
        .ok()
        .as_deref()
        .and_then(parse_color_argb)
        .unwrap_or(DEFAULT_BACKGROUND_COLOR)
}

fn move_focused_window(dx: i32, dy: i32) {
    let focused = get_focused_window();
    if focused.is_null() {
        return;
    }

    unsafe {
        let w = &*focused;
        if w.fullscreen {
            return;
        }

        let mut geo = swc_rectangle {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        if swc_window_get_geometry(w.swc, &mut geo) {
            swc_window_set_position(w.swc, geo.x + dx, geo.y + dy);
        }
    }
}

fn resize_focused_window(dw: i32, dh: i32) {
    let focused = get_focused_window();
    if focused.is_null() {
        return;
    }

    unsafe {
        let w = &*focused;
        if w.fullscreen {
            return;
        }

        let mut geo = swc_rectangle {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        if !swc_window_get_geometry(w.swc, &mut geo) {
            return;
        }

        let new_width = (geo.width as i32 + dw).max(MIN_WINDOW_WIDTH) as u32;
        let new_height = (geo.height as i32 + dh).max(MIN_WINDOW_HEIGHT) as u32;

        swc_window_set_size(w.swc, new_width, new_height);
    }
}

// --- Focus ---

fn focus(window: *mut Window) {
    let focused = get_focused_window();

    unsafe {
        if !focused.is_null() {
            swc_window_set_border((*focused).swc, 0xff888888, 1, 0, 0);
        }

        if !window.is_null() {
            raise_window(window);
            swc_window_set_border((*window).swc, 0xff333388, 1, 0, 0);
            swc_window_focus((*window).swc);
        } else {
            swc_window_focus(ptr::null_mut());
        }
    }

    set_focused_window(window);
    note_window_focus(window);
}

fn focus_next(screen: *mut Screen, window: *mut Window) {
    unsafe {
        let s = &*screen;
        if s.windows.len() <= 1 {
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
}

fn focus_prev(screen: *mut Screen, window: *mut Window) {
    unsafe {
        let s = &*screen;
        if s.windows.len() <= 1 {
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
}

fn adjacent_window(screen: *mut Screen, window: *mut Window, forward: bool) -> *mut Window {
    if screen.is_null() || window.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let s = &*screen;
        if s.windows.len() <= 1 {
            return ptr::null_mut();
        }

        for (i, &w) in s.windows.iter().enumerate() {
            if w == window {
                let index = if forward {
                    (i + 1) % s.windows.len()
                } else if i > 0 {
                    i - 1
                } else {
                    s.windows.len() - 1
                };

                return s.windows[index];
            }
        }
    }

    ptr::null_mut()
}

fn pan_screen_to_window(screen: *mut Screen, target: *mut Window) {
    if screen.is_null() || target.is_null() {
        return;
    }

    unsafe {
        if (*target).fullscreen {
            return;
        }

        let mut target_geo = swc_rectangle {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        if !swc_window_get_geometry((*target).swc, &mut target_geo) {
            return;
        }

        let usable = &(*(*screen).swc).usable_geometry;
        let screen_center_x = usable.x + (usable.width / 2) as i32;
        let screen_center_y = usable.y + (usable.height / 2) as i32;
        let target_center_x = target_geo.x + (target_geo.width / 2) as i32;
        let target_center_y = target_geo.y + (target_geo.height / 2) as i32;

        let dx = screen_center_x - target_center_x;
        let dy = screen_center_y - target_center_y;

        if dx == 0 && dy == 0 {
            return;
        }

        let s = &*screen;
        for &win_ptr in &s.windows {
            if win_ptr.is_null() {
                continue;
            }

            let w = &*win_ptr;
            if w.fullscreen {
                continue;
            }

            let mut geo = swc_rectangle {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            };

            if swc_window_get_geometry(w.swc, &mut geo) {
                swc_window_set_position(w.swc, geo.x + dx, geo.y + dy);
            }
        }
    }
}

// --- Callbacks ---

unsafe extern "C" fn screen_usable_geometry_changed(data: *mut c_void) {
    let screen = data as *mut Screen;
    update_fullscreen_windows(screen);
}

unsafe extern "C" fn screen_entered(data: *mut c_void) {
    set_active_screen(data as *mut Screen);
}

static SCREEN_HANDLER: swc_screen_handler = swc_screen_handler {
    destroy: None,
    geometry_changed: None,
    usable_geometry_changed: Some(screen_usable_geometry_changed),
    entered: Some(screen_entered),
};

unsafe extern "C" fn window_destroy(data: *mut c_void) {
    let window = data as *mut Window;
    let screen = unsafe { (*window).screen };
    let was_focused = get_focused_window() == window;

    if get_moving_window() == window {
        set_moving_window(ptr::null_mut());
    }
    if get_resizing_window() == window {
        set_resizing_window(ptr::null_mut());
    }

    remove_window_from_focus_history(window);

    if was_focused {
        let next_focus = if !screen.is_null() {
            last_active_window_on_screen(screen, window)
        } else {
            ptr::null_mut()
        };

        focus(next_focus);

        if !screen.is_null() && !next_focus.is_null() {
            pan_screen_to_window(screen, next_focus);
        }
    }

    if !screen.is_null() {
        screen_remove_window(screen, window);
    }

    unsafe {
        drop(Box::from_raw(window));
    }
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

    unsafe {
        swc_screen_set_handler(swc, &SCREEN_HANDLER, screen as *mut c_void);
    }
    set_active_screen(screen);
}

unsafe extern "C" fn new_device(device: *mut libinput_device) {
    if device.is_null() {
        return;
    }

    let name = unsafe {
        let ptr = libinput_device_get_name(device);
        if ptr.is_null() {
            "<unknown>".to_string()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };

    let tap_fingers = unsafe { libinput_device_config_tap_get_finger_count(device) };
    if tap_fingers <= 0 {
        return;
    }

    let _ = unsafe { libinput_device_config_tap_set_enabled(device, LIBINPUT_CONFIG_TAP_ENABLED) };
    let _ = unsafe {
        libinput_device_config_tap_set_drag_enabled(device, LIBINPUT_CONFIG_DRAG_ENABLED)
    };

    let methods = unsafe { libinput_device_config_click_get_methods(device) };
    let preferred_click_method = if methods & LIBINPUT_CONFIG_CLICK_METHOD_CLICKFINGER != 0 {
        LIBINPUT_CONFIG_CLICK_METHOD_CLICKFINGER
    } else if methods & LIBINPUT_CONFIG_CLICK_METHOD_BUTTON_AREAS != 0 {
        LIBINPUT_CONFIG_CLICK_METHOD_BUTTON_AREAS
    } else {
        LIBINPUT_CONFIG_CLICK_METHOD_NONE
    };

    if preferred_click_method != LIBINPUT_CONFIG_CLICK_METHOD_NONE {
        let _ = unsafe { libinput_device_config_click_set_method(device, preferred_click_method) };
    }

    if unsafe { libinput_device_config_dwt_is_available(device) } != 0 {
        let dwt_enabled = std::env::var("WM15_TOUCHPAD_DWT")
            .ok()
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                v == "1" || v == "true" || v == "yes" || v == "on"
            })
            .unwrap_or(false);

        let dwt_state = if dwt_enabled {
            LIBINPUT_CONFIG_DWT_ENABLED
        } else {
            LIBINPUT_CONFIG_DWT_DISABLED
        };

        let _ = unsafe { libinput_device_config_dwt_set_enabled(device, dwt_state) };
    }

    println!("wm15: configured touchpad \"{}\"", name);
}

unsafe extern "C" fn new_window(swc: *mut swc_window) {
    let window = Box::into_raw(Box::new(Window {
        swc,
        screen: ptr::null_mut(),
        fullscreen: false,
        saved_geometry: swc_rectangle {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        },
        has_saved_geometry: false,
    }));

    unsafe {
        swc_window_set_handler(swc, &WINDOW_HANDLER, window as *mut c_void);
        swc_window_set_stacked(swc);
    }

    let active_screen = get_active_screen();
    if !active_screen.is_null() {
        screen_add_window(active_screen, window);
    }

    focus(window);

    if !active_screen.is_null() {
        pan_screen_to_window(active_screen, window);
    }
}

static MANAGER: swc_manager = swc_manager {
    new_screen: Some(new_screen),
    new_window: Some(new_window),
    new_device: Some(new_device),
    activate: None,
    deactivate: None,
};

// --- Input handlers ---

const BTN_RIGHT: u32 = 0x111;

unsafe extern "C" fn key_move_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        let mut target = get_focused_window();
        if target.is_null() {
            target = window_at_cursor();
        }
        if target.is_null() {
            return;
        }

        unsafe {
            if (*target).fullscreen {
                return;
            }
        }

        set_resizing_window(ptr::null_mut());
        set_moving_window(target);
        focus(target);

        unsafe {
            swc_window_begin_move((*target).swc);
        }
    } else {
        let moving = get_moving_window();
        if !moving.is_null() {
            unsafe {
                swc_window_end_move((*moving).swc);
            }
            set_moving_window(ptr::null_mut());
        }
    }
}

unsafe extern "C" fn key_resize_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        let mut target = get_focused_window();
        if target.is_null() {
            target = window_at_cursor();
        }
        if target.is_null() {
            return;
        }

        unsafe {
            if (*target).fullscreen {
                return;
            }
        }

        set_moving_window(ptr::null_mut());
        set_resizing_window(target);
        focus(target);

        unsafe {
            swc_window_begin_resize((*target).swc, SWC_WINDOW_EDGE_AUTO);
        }
    } else {
        let resizing = get_resizing_window();
        if !resizing.is_null() {
            unsafe {
                swc_window_end_resize((*resizing).swc);
            }
            set_resizing_window(ptr::null_mut());
        }
    }
}

unsafe extern "C" fn mouse_resize_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        let target = window_at_cursor();
        if target.is_null() {
            return;
        }

        unsafe {
            if (*target).fullscreen {
                return;
            }
        }

        set_moving_window(ptr::null_mut());
        set_resizing_window(target);
        focus(target);

        unsafe {
            swc_window_begin_resize((*target).swc, SWC_WINDOW_EDGE_AUTO);
        }
    } else {
        let resizing = get_resizing_window();
        if !resizing.is_null() {
            unsafe {
                swc_window_end_resize((*resizing).swc);
            }
            set_resizing_window(ptr::null_mut());
        }
    }
}

unsafe extern "C" fn move_left(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        move_focused_window(-MOVE_STEP, 0);
    }
}

unsafe extern "C" fn move_right(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        move_focused_window(MOVE_STEP, 0);
    }
}

unsafe extern "C" fn move_up(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        move_focused_window(0, -MOVE_STEP);
    }
}

unsafe extern "C" fn move_down(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        move_focused_window(0, MOVE_STEP);
    }
}

unsafe extern "C" fn resize_narrower(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        resize_focused_window(-RESIZE_STEP, 0);
    }
}

unsafe extern "C" fn resize_wider(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        resize_focused_window(RESIZE_STEP, 0);
    }
}

unsafe extern "C" fn resize_taller(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        resize_focused_window(0, RESIZE_STEP);
    }
}

unsafe extern "C" fn resize_shorter(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state == 1 {
        resize_focused_window(0, -RESIZE_STEP);
    }
}

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
    Command::new("wofi").args(["--show", "drun"]).spawn().ok();
}

unsafe extern "C" fn quit(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let display = get_display();
    if !display.is_null() {
        unsafe {
            wl_display_terminate(display);
        }
    }
}

unsafe extern "C" fn close_window(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let focused = get_focused_window();
    if !focused.is_null() {
        unsafe {
            swc_window_close((*focused).swc);
        }
    }
}

unsafe extern "C" fn focus_next_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let focused = get_focused_window();
    if !focused.is_null() {
        let screen = unsafe { (*focused).screen };
        if !screen.is_null() {
            focus_next(screen, focused);
        }
    }
}

unsafe extern "C" fn focus_prev_handler(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let focused = get_focused_window();
    if !focused.is_null() {
        let screen = unsafe { (*focused).screen };
        if !screen.is_null() {
            focus_prev(screen, focused);
        }
    }
}

unsafe extern "C" fn toggle_fullscreen(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let focused = get_focused_window();
    if focused.is_null() {
        return;
    }

    unsafe {
        let w = &mut *focused;
        if w.screen.is_null() {
            return;
        }

        if w.fullscreen {
            w.fullscreen = false;
            swc_window_set_stacked(w.swc);

            if w.has_saved_geometry {
                swc_window_set_geometry(w.swc, &w.saved_geometry);
            } else {
                place_window(w.screen, focused);
            }

            w.has_saved_geometry = false;
        } else {
            let mut current_geo = swc_rectangle {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            };

            if swc_window_get_geometry(w.swc, &mut current_geo) {
                w.saved_geometry = current_geo;
                w.has_saved_geometry = true;
            }

            w.fullscreen = true;

            let usable = &(*(*w.screen).swc).usable_geometry;
            let fullscreen_geo = swc_rectangle {
                x: usable.x,
                y: usable.y,
                width: usable.width,
                height: usable.height,
            };
            swc_window_set_geometry(w.swc, &fullscreen_geo);
        }
    }
}

unsafe extern "C" fn zoom_in(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    unsafe {
        let zoom = swc_get_zoom();
        swc_set_zoom((zoom + 0.1).min(5.0));
    }
}

unsafe extern "C" fn zoom_out(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    unsafe {
        let zoom = swc_get_zoom();
        swc_set_zoom((zoom - 0.1).max(0.1));
    }
}

unsafe extern "C" fn zoom_reset(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    unsafe {
        swc_set_zoom(1.0);
    }
}

unsafe extern "C" fn pan_to_next_window(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let focused = get_focused_window();
    if focused.is_null() {
        return;
    }

    let screen = unsafe { (*focused).screen };
    if screen.is_null() {
        return;
    }

    let target = adjacent_window(screen, focused, true);
    if target.is_null() {
        return;
    }

    focus(target);
    pan_screen_to_window(screen, target);
}

unsafe extern "C" fn pan_to_prev_window(_data: *mut c_void, _time: u32, _value: u32, state: u32) {
    if state != 1 {
        return;
    }

    let focused = get_focused_window();
    if focused.is_null() {
        return;
    }

    let screen = unsafe { (*focused).screen };
    if screen.is_null() {
        return;
    }

    let target = adjacent_window(screen, focused, false);
    if target.is_null() {
        return;
    }

    focus(target);
    pan_screen_to_window(screen, target);
}

fn add_key_binding(modifiers: u32, key: u32, handler: swc_binding_handler) {
    unsafe {
        swc_add_binding(
            swc_binding_type::SWC_BINDING_KEY,
            modifiers,
            key,
            handler,
            ptr::null_mut(),
        );
    }
}

fn add_button_binding(modifiers: u32, button: u32, handler: swc_binding_handler) {
    unsafe {
        swc_add_binding(
            swc_binding_type::SWC_BINDING_BUTTON,
            modifiers,
            button,
            handler,
            ptr::null_mut(),
        );
    }
}

// --- Main ---

fn main() {
    let display = unsafe { wl_display_create() };
    if display.is_null() {
        eprintln!("Failed to create wayland display");
        std::process::exit(1);
    }
    set_display(display);

    let socket = unsafe { wl_display_add_socket_auto(display) };
    if socket.is_null() {
        eprintln!("Failed to add wayland socket");
        std::process::exit(1);
    }

    let socket_str = unsafe { std::ffi::CStr::from_ptr(socket) };
    std::env::set_var("WAYLAND_DISPLAY", socket_str.to_str().unwrap());

    // Force wld to use dumb DRM driver (needed for GPUs not in wld's PCI ID table)
    std::env::set_var("WLD_DRM_DUMB", "1");

    if !unsafe { swc_initialize(display, ptr::null_mut(), &MANAGER) } {
        eprintln!("Failed to initialize swc");
        std::process::exit(1);
    }

    let background = configured_background_color();
    unsafe {
        swc_wallpaper_color_set(background);
    }

    add_key_binding(SWC_MOD_LOGO, XKB_KEY_Return, Some(spawn_st));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_d, Some(spawn_dmenu));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_q, Some(quit));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_SHIFT, XKB_KEY_q, Some(close_window));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_j, Some(focus_next_handler));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_k, Some(focus_prev_handler));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_f, Some(toggle_fullscreen));
    add_key_binding(
        SWC_MOD_LOGO | SWC_MOD_SHIFT,
        XKB_KEY_j,
        Some(pan_to_next_window),
    );
    add_key_binding(
        SWC_MOD_LOGO | SWC_MOD_SHIFT,
        XKB_KEY_k,
        Some(pan_to_prev_window),
    );
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_equal, Some(zoom_in));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_minus, Some(zoom_out));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_0, Some(zoom_reset));

    add_key_binding(SWC_MOD_LOGO, XKB_KEY_m, Some(key_move_handler));
    add_key_binding(SWC_MOD_LOGO, XKB_KEY_r, Some(key_resize_handler));

    add_key_binding(SWC_MOD_LOGO | SWC_MOD_CTRL, XKB_KEY_h, Some(move_left));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_CTRL, XKB_KEY_l, Some(move_right));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_CTRL, XKB_KEY_k, Some(move_up));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_CTRL, XKB_KEY_j, Some(move_down));

    add_key_binding(SWC_MOD_LOGO | SWC_MOD_ALT, XKB_KEY_h, Some(resize_narrower));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_ALT, XKB_KEY_l, Some(resize_wider));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_ALT, XKB_KEY_j, Some(resize_taller));
    add_key_binding(SWC_MOD_LOGO | SWC_MOD_ALT, XKB_KEY_k, Some(resize_shorter));

    add_button_binding(SWC_MOD_LOGO, BTN_RIGHT, Some(mouse_resize_handler));

    println!("wm15: running on {}", socket_str.to_str().unwrap());
    unsafe {
        wl_display_run(display);
        wl_display_destroy(display);
    }
}
