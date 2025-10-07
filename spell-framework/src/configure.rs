use slint::platform::software_renderer::TargetPixel;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use std::{
    cell::Cell,
    fs,
    io::Write,
    os::unix::net::{UnixDatagram, UnixStream},
    path::Path,
    sync::Mutex,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer as TracingTraitLayer,
    filter::Filtered,
    fmt::{
        self, FormatEvent, Layer as TracingLayer,
        format::{DefaultFields, Format, Full},
    },
    layer::{Layered, SubscriberExt},
    registry::Registry,
    reload::{Handle, Layer as LoadLayer},
};

/// Unused Internal struct representation of a pixel, it is similar to slint's
/// representation of [pixel]() but implement few more trait. Currently, redundent
#[allow(dead_code)]
#[derive(Default)]
pub struct Rgba8Pixel {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgba8Pixel {
    pub fn new(a: u8, r: u8, g: u8, b: u8) -> Self {
        Rgba8Pixel { a, r, g, b }
    }
}

impl TargetPixel for Rgba8Pixel {
    fn blend(&mut self, color: slint::platform::software_renderer::PremultipliedRgbaColor) {
        let a: u16 = (u8::MAX - color.alpha) as u16;
        // self.a = a as u8;
        let out_a = color.alpha as u16 + (self.a as u16 * (255 - color.alpha) as u16) / 255;
        self.a = out_a as u8;
        self.r = (self.r as u16 * a / 255) as u8 + color.red;
        self.g = (self.g as u16 * a / 255) as u8 + color.green;
        self.b = (self.b as u16 * a / 255) as u8 + color.blue;
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        let a = 0xFF;
        Self::new(a, red, green, blue)
    }

    fn background() -> Self {
        // TODO This needs to be decided to see how it should be 0xFF or 0x00;
        // I think there is a bug in slint which is causing the leak of This
        // value.
        let a: u8 = 0x00;
        Self::new(a, 0, 0, 0)
    }
}

impl std::marker::Copy for Rgba8Pixel {}
impl std::clone::Clone for Rgba8Pixel {
    fn clone(&self) -> Self {
        *self
    }
}

/// WindowConf is an essential struct passed on to widget constructor functions (like [invoke_spell](crate::wayland_adapter::SpellWin::invoke_spell))
/// for defining the specifications of the widget.
///
/// ## Panics
///
/// event_loop will panic if 0 is provided as width and height.
#[derive(Debug, Clone)]
pub struct WindowConf {
    /// Defines the widget width in pixels. On setting values greater than the provided pixels of
    /// monitor, the widget offsets from monitor's prectangular monitor space. It is important to
    /// note that the value should be the maximum width the widget will ever attain, not the
    /// current width in case of resizeable widgets.
    pub width: u32,
    /// Defines the widget height in pixels. On setting values greater than the provided pixels of
    /// monitor, the widget offsets from monitor's prectangular monitor space. It is important to
    /// note that the value should be the maximum width the widget will ever attain, not the
    /// current width in case of resizeable widgets.
    pub height: u32,
    /// Defines the Anchors to which the window needs to be attached. View [`Anchor`] for
    /// related explaination of usage. If both values are None, then widget is displayed in the
    /// center of screen.
    pub anchor: (Option<Anchor>, Option<Anchor>),
    /// Defines the margin of widget from monitor edges, negative values make the widget go outside
    /// of monitor pixels if anchored to some edge(s). Otherwise, the widget moves to the opposite
    /// direction to the given pixels.
    pub margin: (i32, i32, i32, i32),
    /// Defines the possible layer on which to define the widget. View [`Layer`] for more details.
    pub layer_type: Layer,
    /// Defines the relation of widget with Keyboard. View [`KeyboardInteractivity`] for more
    /// details.
    pub board_interactivity: Cell<KeyboardInteractivity>,
    /// Defines if the widget is exclusive of not, if marked true, further laying of widgets in the
    /// same area is avoided. View [wayland docs](https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_exclusive_zone)
    /// for details. By default, spell takes the value of width if anchored at top or bottom and
    /// height if anchored left or right and set that to the exclusive zone.
    /// TOTEST, define the case when widget is resizeable.
    pub exclusive_zone: bool,
}

impl WindowConf {
    /// constructor method for initialising an instance of WindowConf.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        max_width: u32,
        max_height: u32,
        anchor: (Option<Anchor>, Option<Anchor>),
        margin: (i32, i32, i32, i32),
        layer_type: Layer,
        board_interactivity: KeyboardInteractivity,
        exclusive_zone: bool,
    ) -> Self {
        WindowConf {
            width: max_width,
            height: max_height,
            anchor,
            margin,
            layer_type,
            board_interactivity: Cell::new(board_interactivity),
            exclusive_zone,
        }
    }
}

pub(crate) type HomeHandle =
    Handle<EnvFilter, Layered<TracingLayer<Registry, DefaultFields, Format<Full, ()>>, Registry>>;
pub(crate) fn set_up_tracing(widget_name: &str) -> HomeHandle {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("runtime dir is not set");
    let logging_dir = runtime_dir + "/spell/";
    let socket_dir = logging_dir.clone() + "/spell.sock";

    let _ = fs::create_dir(Path::new(&logging_dir));
    let _ = fs::remove_file(&socket_dir);

    let stream = UnixDatagram::unbound().unwrap();
    // stream.connect(&socket_dir).unwrap();

    stream
        .set_nonblocking(true)
        .expect("Non blocking couldn't be set");

    let writer = RollingFileAppender::builder()
        .rotation(Rotation::HOURLY) // rotate log files once every hour
        .filename_prefix(widget_name) // log file names will be prefixed with `myapp.`
        .filename_suffix("log") // log file names will be suffixed with `.log`
        .build(&logging_dir) // try to build an appender that stores log files in `/var/log`
        .expect("initializing rolling file appender failed");

    let layer_writer = fmt::layer()
        .without_time()
        .with_target(false)
        .with_writer(writer)
        .with_filter(EnvFilter::new("spell_framework=trace,info"));

    if let Ok(val) = std::env::var("RUST_LOG") {
        println!("Rust log value: {}", val);
    } else {
        println!("Val not set");
    }

    let (layer_env, handle) = LoadLayer::new(EnvFilter::from_default_env());
    let layer_socket = fmt::layer().with_writer(Mutex::new(SocketWriter::new(stream)));
    let subs = tracing_subscriber::registry()
        .with(fmt::layer().without_time().with_target(false))
        .with(layer_env)
        .with(layer_socket)
        .with(layer_writer);
    // .with(layer_std);
    tracing::subscriber::set_global_default(subs).expect("Got error in setting subs");
    handle
}

pub(crate) struct SocketWriter {
    socket: UnixDatagram,
    // formatter: Format<DefaultFields>,
}

impl SocketWriter {
    fn new(socket: UnixDatagram) -> Self {
        SocketWriter { socket }
    }
}

impl Write for SocketWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("runtime dir is not set");
        let logging_dir = runtime_dir + "/spell/";
        let socket_dir = logging_dir.clone() + "/spell.sock";

        self.socket.send_to(buf, Path::new(&socket_dir))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// TODO this will be made public when multiple widgets in the same layer is supported.
// Likely it will be easy after the resize action is implemented
#[allow(dead_code)]
pub enum LayerConf {
    Window(WindowConf),
    Windows(Vec<WindowConf>),
    Lock(u32, u32),
}
