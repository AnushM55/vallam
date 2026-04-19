#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

// Forward declarations for wayland types
pub enum wl_display {}
pub enum wl_event_loop {}

pub type wl_fixed_t = i32;
pub type pid_t = i32;

// Opaque libswc types
pub enum libinput_device {}

// --- libinput config constants ---

pub const LIBINPUT_CONFIG_STATUS_SUCCESS: c_int = 0;

pub const LIBINPUT_CONFIG_TAP_DISABLED: c_int = 0;
pub const LIBINPUT_CONFIG_TAP_ENABLED: c_int = 1;

pub const LIBINPUT_CONFIG_DRAG_DISABLED: c_int = 0;
pub const LIBINPUT_CONFIG_DRAG_ENABLED: c_int = 1;

pub const LIBINPUT_CONFIG_DWT_DISABLED: c_int = 0;
pub const LIBINPUT_CONFIG_DWT_ENABLED: c_int = 1;

pub const LIBINPUT_CONFIG_CLICK_METHOD_NONE: c_uint = 0;
pub const LIBINPUT_CONFIG_CLICK_METHOD_BUTTON_AREAS: c_uint = 1 << 0;
pub const LIBINPUT_CONFIG_CLICK_METHOD_CLICKFINGER: c_uint = 1 << 1;

// --- Rectangles ---

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct swc_rectangle {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

// --- Screens ---

#[repr(C)]
pub struct swc_screen_handler {
    pub destroy: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub geometry_changed: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub usable_geometry_changed: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub entered: Option<unsafe extern "C" fn(data: *mut c_void)>,
}

#[repr(C)]
pub struct swc_screen {
    pub geometry: swc_rectangle,
    pub usable_geometry: swc_rectangle,
}

// --- Windows ---

#[repr(C)]
pub struct swc_window_handler {
    pub destroy: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub title_changed: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub app_id_changed: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub parent_changed: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub entered: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub move_: Option<unsafe extern "C" fn(data: *mut c_void)>,
    pub resize: Option<unsafe extern "C" fn(data: *mut c_void)>,
}

#[repr(C)]
pub struct swc_window {
    pub title: *mut c_char,
    pub app_id: *mut c_char,
    pub parent: *mut swc_window,
    pub motion_throttle_ms: u32,
    pub min_width: u32,
    pub min_height: u32,
    pub max_width: u32,
    pub max_height: u32,
}

// --- Cursor ---

pub const SWC_CURSOR_DEFAULT: c_uint = 0;
pub const SWC_CURSOR_BOX: c_uint = 1;
pub const SWC_CURSOR_CROSS: c_uint = 2;
pub const SWC_CURSOR_SIGHT: c_uint = 3;
pub const SWC_CURSOR_UP: c_uint = 4;
pub const SWC_CURSOR_DOWN: c_uint = 5;

pub const SWC_CURSOR_MODE_CLIENT: c_uint = 0;
pub const SWC_CURSOR_MODE_COMPOSITOR: c_uint = 1;

// --- Window edges ---

pub const SWC_WINDOW_EDGE_AUTO: c_uint = 0;
pub const SWC_WINDOW_EDGE_TOP: c_uint = 1 << 0;
pub const SWC_WINDOW_EDGE_BOTTOM: c_uint = 1 << 1;
pub const SWC_WINDOW_EDGE_LEFT: c_uint = 1 << 2;
pub const SWC_WINDOW_EDGE_RIGHT: c_uint = 1 << 3;

// --- Bindings ---

pub const SWC_MOD_CTRL: c_uint = 1 << 0;
pub const SWC_MOD_ALT: c_uint = 1 << 1;
pub const SWC_MOD_LOGO: c_uint = 1 << 2;
pub const SWC_MOD_SHIFT: c_uint = 1 << 3;
pub const SWC_MOD_ANY: c_uint = !0;

#[repr(C)]
pub enum swc_binding_type {
    SWC_BINDING_KEY = 0,
    SWC_BINDING_BUTTON = 1,
}

pub type swc_binding_handler =
    Option<unsafe extern "C" fn(data: *mut c_void, time: u32, value: u32, state: u32)>;

pub type swc_axis_binding_handler =
    Option<unsafe extern "C" fn(data: *mut c_void, time: u32, axis: u32, value120: i32)>;

// --- Manager ---

#[repr(C)]
pub struct swc_manager {
    pub new_screen: Option<unsafe extern "C" fn(screen: *mut swc_screen)>,
    pub new_window: Option<unsafe extern "C" fn(window: *mut swc_window)>,
    pub new_device: Option<unsafe extern "C" fn(device: *mut libinput_device)>,
    pub activate: Option<unsafe extern "C" fn()>,
    pub deactivate: Option<unsafe extern "C" fn()>,
}

// --- Key constants ---

pub const XKB_KEY_Return: u32 = 0xff0d;
pub const XKB_KEY_r: u32 = 0x0072;
pub const XKB_KEY_q: u32 = 0x0071;
pub const XKB_KEY_space: u32 = 0x0020;
pub const XKB_KEY_h: u32 = 0x0068;
pub const XKB_KEY_j: u32 = 0x006a;
pub const XKB_KEY_k: u32 = 0x006b;
pub const XKB_KEY_l: u32 = 0x006c;
pub const XKB_KEY_f: u32 = 0x0066;
pub const XKB_KEY_Escape: u32 = 0xff1b;
pub const XKB_KEY_Tab: u32 = 0xff09;
pub const XKB_KEY_d: u32 = 0x0064;
pub const XKB_KEY_m: u32 = 0x006d;
pub const XKB_KEY_minus: u32 = 0x002d;
pub const XKB_KEY_equal: u32 = 0x003d;
pub const XKB_KEY_0: u32 = 0x0030;

pub const XKB_KEY_XF86MonBrightnessUp: u32 = 0x1008ff02;
pub const XKB_KEY_XF86MonBrightnessDown: u32 = 0x1008ff03;
pub const XKB_KEY_XF86AudioLowerVolume: u32 = 0x1008ff11;
pub const XKB_KEY_XF86AudioMute: u32 = 0x1008ff12;
pub const XKB_KEY_XF86AudioRaiseVolume: u32 = 0x1008ff13;

// --- FFI functions ---

#[link(name = "swc")]
extern "C" {
    // Core initialization
    pub fn swc_initialize(
        display: *mut wl_display,
        event_loop: *mut wl_event_loop,
        manager: *const swc_manager,
    ) -> bool;

    pub fn swc_finalize();

    // Screen
    pub fn swc_screen_set_handler(
        screen: *mut swc_screen,
        handler: *const swc_screen_handler,
        data: *mut c_void,
    );

    // Window
    pub fn swc_window_set_handler(
        window: *mut swc_window,
        handler: *const swc_window_handler,
        data: *mut c_void,
    );
    pub fn swc_window_close(window: *mut swc_window);
    pub fn swc_window_show(window: *mut swc_window);
    pub fn swc_window_hide(window: *mut swc_window);
    pub fn swc_window_focus(window: *mut swc_window);
    pub fn swc_window_set_stacked(window: *mut swc_window);
    pub fn swc_window_set_tiled(window: *mut swc_window);
    pub fn swc_window_set_fullscreen(window: *mut swc_window, screen: *mut swc_screen);
    pub fn swc_window_set_position(window: *mut swc_window, x: i32, y: i32);
    pub fn swc_window_set_size(window: *mut swc_window, width: u32, height: u32);
    pub fn swc_window_set_geometry(window: *mut swc_window, geometry: *const swc_rectangle);
    pub fn swc_window_get_geometry(window: *const swc_window, geometry: *mut swc_rectangle)
        -> bool;
    pub fn swc_window_get_pid(window: *mut swc_window) -> pid_t;
    pub fn swc_window_set_border(
        window: *mut swc_window,
        inner_color: u32,
        inner_width: u32,
        outer_color: u32,
        outer_width: u32,
    );
    pub fn swc_window_begin_move(window: *mut swc_window);
    pub fn swc_window_end_move(window: *mut swc_window);
    pub fn swc_window_begin_resize(window: *mut swc_window, edges: u32);
    pub fn swc_window_end_resize(window: *mut swc_window);
    pub fn swc_window_at(x: i32, y: i32) -> *mut swc_window;
    pub fn swc_window_stack(window: *mut swc_window, direction: i32);

    // Bindings
    pub fn swc_add_binding(
        type_: swc_binding_type,
        modifiers: u32,
        value: u32,
        handler: swc_binding_handler,
        data: *mut c_void,
    ) -> c_int;

    pub fn swc_add_axis_binding(
        modifiers: u32,
        axis: u32,
        handler: swc_axis_binding_handler,
        data: *mut c_void,
    ) -> c_int;

    // Pointer forwarding
    pub fn swc_pointer_send_button(time: u32, button: u32, state: u32);
    pub fn swc_pointer_send_axis(time: u32, axis: u32, value120: i32);

    // Cursor
    pub fn swc_cursor_position(x: *mut i32, y: *mut i32) -> bool;
    pub fn swc_set_cursor(kind: c_uint);
    pub fn swc_set_cursor_mode(mode: c_uint);

    // Overlay
    pub fn swc_overlay_set_box(x1: i32, y1: i32, x2: i32, y2: i32, color: u32, border_width: u32);
    pub fn swc_overlay_clear();

    // Zoom
    pub fn swc_set_zoom(level: f32);
    pub fn swc_get_zoom() -> f32;

    // Wallpaper
    pub fn swc_wallpaper_color_set(color: u32);
}

// libinput device configuration
#[link(name = "input")]
extern "C" {
    pub fn libinput_device_get_name(device: *mut libinput_device) -> *const c_char;

    pub fn libinput_device_config_tap_get_finger_count(device: *mut libinput_device) -> c_int;
    pub fn libinput_device_config_tap_set_enabled(
        device: *mut libinput_device,
        enable: c_int,
    ) -> c_int;
    pub fn libinput_device_config_tap_set_drag_enabled(
        device: *mut libinput_device,
        enable: c_int,
    ) -> c_int;

    pub fn libinput_device_config_click_get_methods(device: *mut libinput_device) -> c_uint;
    pub fn libinput_device_config_click_set_method(
        device: *mut libinput_device,
        method: c_uint,
    ) -> c_int;

    pub fn libinput_device_config_dwt_is_available(device: *mut libinput_device) -> c_int;
    pub fn libinput_device_config_dwt_set_enabled(
        device: *mut libinput_device,
        enable: c_int,
    ) -> c_int;
}

// Wayland server functions
#[link(name = "wayland-server")]
extern "C" {
    pub fn wl_display_create() -> *mut wl_display;
    pub fn wl_display_add_socket_auto(display: *mut wl_display) -> *const c_char;
    pub fn wl_display_get_event_loop(display: *mut wl_display) -> *mut wl_event_loop;
    pub fn wl_display_run(display: *mut wl_display);
    pub fn wl_display_terminate(display: *mut wl_display);
    pub fn wl_display_destroy(display: *mut wl_display);
}
