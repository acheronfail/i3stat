pub use istat::i3::I3Button as MouseButton;
use x11::xlib;
use xcb::x::{self, Drawable, WarpPointer};
use xcb::xtest::FakeInput;
use xcb::{Connection, Extension};

// Use `xmodmap -pke` to list key codes
#[allow(unused)]
#[repr(u8)]
pub enum KeyCode {
    Escape = 9,
    Num1 = 10,
    Num2 = 11,
    Num3 = 12,
    Num4 = 13,
    Num5 = 14,
    Num6 = 15,
    Num7 = 16,
    Num8 = 17,
    Num9 = 18,
    Num0 = 19,
    Minus = 20,
    Equal = 21,
    BackSpace = 22,
    Tab = 23,
    Q = 24,
    W = 25,
    E = 26,
    R = 27,
    T = 28,
    Y = 29,
    U = 30,
    I = 31,
    O = 32,
    P = 33,
    Bracketleft = 34,
    Bracketright = 35,
    Return = 36,
    ControlL = 37,
    A = 38,
    S = 39,
    D = 40,
    F = 41,
    G = 42,
    H = 43,
    J = 44,
    K = 45,
    L = 46,
    Semicolon = 47,
    Apostrophe = 48,
    Grave = 49,
    ShiftL = 50,
    Backslash = 51,
    Z = 52,
    X = 53,
    C = 54,
    V = 55,
    B = 56,
    N = 57,
    M = 58,
    Comma = 59,
    Period = 60,
    Slash = 61,
    ShiftR = 62,
    KpMultiply = 63,
    AltL = 64,
    Space = 65,
    CapsLock = 66,
}

pub fn x_click(x_display: impl AsRef<str>, button: MouseButton, x: i16, y: i16) {
    // connect to the X server
    let (conn, screen_num) = Connection::connect_with_extensions(
        Some(x_display.as_ref()),
        &[Extension::Xkb, Extension::Test],
        &[],
    )
    .unwrap();

    // get X's root window and its geometry
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let root_window = screen.root();
    let root_geometry = conn
        .wait_for_reply(conn.send_request(&x::GetGeometry {
            drawable: Drawable::Window(root_window),
        }))
        .unwrap();

    // move mouse to click location
    conn.send_and_check_request(&WarpPointer {
        src_window: root_window,
        dst_window: root_window,
        src_x: root_geometry.x(),
        src_y: root_geometry.y(),
        src_width: root_geometry.width(),
        src_height: root_geometry.height(),
        dst_x: x,
        dst_y: y,
    })
    .unwrap();

    // setup fake button
    let mut fake_button = FakeInput {
        r#type: xlib::ButtonPress as _,
        detail: button as _,
        time: x::CURRENT_TIME,
        root: root_window,
        root_x: x,
        root_y: y,
        deviceid: 0, // 0 = none
    };

    // click: button press and release
    conn.send_and_check_request(&fake_button).unwrap();
    fake_button.r#type = xlib::ButtonRelease as _;
    conn.send_and_check_request(&fake_button).unwrap();
}
