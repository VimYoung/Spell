use slint::platform::software_renderer::TargetPixel;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use std::{cell::Cell, fs, io::Write, os::unix::net::UnixDatagram, path::Path, sync::Mutex};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer as TracingTraitLayer,
    filter::Filtered,
    fmt::{self, format::DefaultFields},
    layer::{Layered, SubscriberExt},
    registry::Registry,
    reload::Layer as LoadLayer,
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
/// event loops like [cast_spell](crate::cast_spell) and [encahnt_spells](crate::enchant_spells) will panic if 0 is provided as width or height.
#[derive(Debug, Clone)]
pub struct WindowConf {
    /// Defines the widget width in pixels. On setting values greater than the provided pixels of
    /// monitor, the widget offsets from monitor's rectangular monitor space. It is important to
    /// note that the value should be the maximum width the widget will ever attain, not the
    /// current width in case of resizeable widgets. This value has no default and needs to be set.
    pub width: u32,
    /// Defines the widget height in pixels. On setting values greater than the provided pixels of
    /// monitor, the widget offsets from monitor's rectangular monitor space. It is important to
    /// note that the value should be the maximum height the widget will ever attain, not the
    /// current height in case of resizeable widgets. This value has no default and needs to be set.
    pub height: u32,
    /// Defines the Anchors to which the window needs to be attached. View [`Anchor`] for
    /// related explaination of usage. If both values are None, then widget is displayed in the
    /// center of screen.
    pub anchor: (Option<Anchor>, Option<Anchor>),
    /// Defines the margin of widget from monitor edges, negative values make the widget go outside
    /// of monitor pixels if anchored to some edge(s). Otherwise, the widget moves to the opposite
    /// direction to the given pixels. Defaults to `0` for all sides.
    pub margin: (i32, i32, i32, i32),
    /// Defines the possible layer on which to define the widget. View [`Layer`] for more details.
    /// Defaults to [`Layer::Top`].
    pub layer_type: Layer,
    /// Defines the relation of widget with Keyboard. View [`KeyboardInteractivity`] for more
    /// details. Defauts to [`KeyboardInteractivity::None`]
    pub board_interactivity: Cell<KeyboardInteractivity>,
    /// Defines if the widget is exclusive of not,if not set to None, else set to number of pixels to
    /// set as exclusive zone as i32. Defaults to no exclusive zone.
    pub exclusive_zone: Option<i32>,
    /// Defines the monitor name on which to spawn the window.
    /// When no monitor is provided, the window is spawned on the default monitor.
    pub monitor_name: Option<String>,
    /// Defines if the method of scrolling for the widget should be natural or
    /// reverse. Defaults to reverse scrolling. Learn more about scrolling types
    /// [here](https://blog.logrocket.com/ux-design/natural-vs-reverse-scrolling/).
    pub natural_scroll: bool,
}

impl WindowConf {
    /// constructor method for initialising an instance of WindowConf.
    #[deprecated(
        since = "1.0.2",
        note = "Use the builder method to access all the configuration. It will be removed in release 1.0.3."
    )]
    pub fn new(
        max_width: u32,
        max_height: u32,
        anchor: (Option<Anchor>, Option<Anchor>),
        margin: (i32, i32, i32, i32),
        layer_type: Layer,
        board_interactivity: KeyboardInteractivity,
        exclusive_zone: Option<i32>,
    ) -> Self {
        WindowConf {
            width: max_width,
            height: max_height,
            anchor,
            margin,
            layer_type,
            board_interactivity: Cell::new(board_interactivity),
            exclusive_zone,
            monitor_name: None,
            natural_scroll: false,
        }
    }

    /// Creates a builder instance for creation of WindowConf, to view defaults
    /// head over to documentation of [`WindowConf`]'s parameters.
    pub fn builder() -> WindowConfBuilder {
        WindowConfBuilder::default()
    }
}

#[derive(Default)]
pub struct WindowConfBuilder {
    max_width: u32,
    max_height: u32,
    anchor: (Option<Anchor>, Option<Anchor>),
    margin: (i32, i32, i32, i32),
    layer_type: Option<Layer>,
    board_interactivity: KeyboardInteractivity,
    exclusive_zone: Option<i32>,
    monitor_name: Option<String>,
    natural_scroll: bool,
}

impl WindowConfBuilder {
    /// Sets [`WindowConf::width`].
    pub fn width<I: Into<u32>>(&mut self, width: I) -> &mut Self {
        let new = self;
        new.max_width = width.into();
        new
    }

    /// Sets [`WindowConf::height`].
    pub fn height<I: Into<u32>>(&mut self, height: I) -> &mut Self {
        let x = self;
        x.max_height = height.into();
        x
    }

    /// Sets first anchor of [`WindowConf::anchor`].
    pub fn anchor_1(&mut self, anchor: Anchor) -> &mut Self {
        let x = self;
        x.anchor.0 = Some(anchor);
        x
    }

    /// Sets second anchor of [`WindowConf::anchor`].
    pub fn anchor_2(&mut self, anchor: Anchor) -> &mut Self {
        let x = self;
        x.anchor.1 = Some(anchor);
        x
    }

    /// Sets [`WindowConf::margin`].
    pub fn margins(&mut self, top: i32, right: i32, bottom: i32, left: i32) -> &mut Self {
        let x = self;
        x.margin = (top, right, bottom, left);
        x
    }

    /// Sets [`WindowConf::layer_type`].
    pub fn layer_type(&mut self, layer: Layer) -> &mut Self {
        let x = self;
        x.layer_type = Some(layer);
        x
    }

    /// Sets [`WindowConf::board_interactivity`].
    pub fn board_interactivity(&mut self, board: KeyboardInteractivity) -> &mut Self {
        let x = self;
        x.board_interactivity = board;
        x
    }

    /// Sets [`WindowConf::exclusive_zone`].
    pub fn exclusive_zone(&mut self, dimention: i32) -> &mut Self {
        let x = self;
        x.exclusive_zone = Some(dimention);
        x
    }

    /// Sets [`WindowConf::monitor_name`].
    pub fn monitor(&mut self, name: String) -> &mut Self {
        let x = self;
        x.monitor_name = Some(name);
        x
    }

    /// Sets [`WindowConf::natural_scroll`].
    pub fn natural_scroll(&mut self, scroll: bool) -> &mut Self {
        let x = self;
        x.natural_scroll = scroll;
        x
    }

    /// Creates an instnce of [`WindowConf`] with the provided configurations.
    /// This function result in an error if width and height are not set or they
    /// are set to zero.
    pub fn build(&self) -> Result<WindowConf, Box<dyn std::error::Error>> {
        Ok(WindowConf {
            width: if self.max_width != 0 {
                self.max_width
            } else {
                return Err("width is either not defined or set to zero".into());
            },
            height: if self.max_width != 0 {
                self.max_height
            } else {
                return Err("height is either not defined or set to zero".into());
            },
            anchor: self.anchor,
            margin: self.margin,
            layer_type: match self.layer_type {
                None => Layer::Top,
                Some(val) => val,
            },
            board_interactivity: Cell::new(self.board_interactivity),
            exclusive_zone: self.exclusive_zone,
            monitor_name: self.monitor_name.clone(),
            natural_scroll: self.natural_scroll,
        })
    }
}

pub(crate) type HomeHandle = tracing_subscriber::reload::Handle<
    Filtered<
        tracing_subscriber::fmt::Layer<
            Layered<
                Filtered<
                    tracing_subscriber::fmt::Layer<
                        Layered<
                            Filtered<
                                tracing_subscriber::fmt::Layer<
                                    Registry,
                                    DefaultFields,
                                    tracing_subscriber::fmt::format::Format<
                                        tracing_subscriber::fmt::format::Full,
                                        (),
                                    >,
                                >,
                                EnvFilter,
                                Registry,
                            >,
                            Registry,
                        >,
                        DefaultFields,
                        tracing_subscriber::fmt::format::Format<
                            tracing_subscriber::fmt::format::Full,
                            (),
                        >,
                        RollingFileAppender,
                    >,
                    EnvFilter,
                    Layered<
                        Filtered<
                            tracing_subscriber::fmt::Layer<
                                Registry,
                                DefaultFields,
                                tracing_subscriber::fmt::format::Format<
                                    tracing_subscriber::fmt::format::Full,
                                    (),
                                >,
                            >,
                            EnvFilter,
                            Registry,
                        >,
                        Registry,
                    >,
                >,
                Layered<
                    Filtered<
                        tracing_subscriber::fmt::Layer<
                            Registry,
                            DefaultFields,
                            tracing_subscriber::fmt::format::Format<
                                tracing_subscriber::fmt::format::Full,
                                (),
                            >,
                        >,
                        EnvFilter,
                        Registry,
                    >,
                    Registry,
                >,
            >,
            DefaultFields,
            tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Full, ()>,
            std::sync::Mutex<SocketWriter>,
        >,
        EnvFilter,
        Layered<
            Filtered<
                tracing_subscriber::fmt::Layer<
                    Layered<
                        Filtered<
                            tracing_subscriber::fmt::Layer<
                                Registry,
                                DefaultFields,
                                tracing_subscriber::fmt::format::Format<
                                    tracing_subscriber::fmt::format::Full,
                                    (),
                                >,
                            >,
                            EnvFilter,
                            Registry,
                        >,
                        Registry,
                    >,
                    DefaultFields,
                    tracing_subscriber::fmt::format::Format<
                        tracing_subscriber::fmt::format::Full,
                        (),
                    >,
                    RollingFileAppender,
                >,
                EnvFilter,
                Layered<
                    Filtered<
                        tracing_subscriber::fmt::Layer<
                            Registry,
                            DefaultFields,
                            tracing_subscriber::fmt::format::Format<
                                tracing_subscriber::fmt::format::Full,
                                (),
                            >,
                        >,
                        EnvFilter,
                        Registry,
                    >,
                    Registry,
                >,
            >,
            Layered<
                Filtered<
                    tracing_subscriber::fmt::Layer<
                        Registry,
                        DefaultFields,
                        tracing_subscriber::fmt::format::Format<
                            tracing_subscriber::fmt::format::Full,
                            (),
                        >,
                    >,
                    EnvFilter,
                    Registry,
                >,
                Registry,
            >,
        >,
    >,
    Layered<
        Filtered<
            tracing_subscriber::fmt::Layer<
                Layered<
                    Filtered<
                        tracing_subscriber::fmt::Layer<
                            Registry,
                            DefaultFields,
                            tracing_subscriber::fmt::format::Format<
                                tracing_subscriber::fmt::format::Full,
                                (),
                            >,
                        >,
                        EnvFilter,
                        Registry,
                    >,
                    Registry,
                >,
                DefaultFields,
                tracing_subscriber::fmt::format::Format<tracing_subscriber::fmt::format::Full, ()>,
                RollingFileAppender,
            >,
            EnvFilter,
            Layered<
                Filtered<
                    tracing_subscriber::fmt::Layer<
                        Registry,
                        DefaultFields,
                        tracing_subscriber::fmt::format::Format<
                            tracing_subscriber::fmt::format::Full,
                            (),
                        >,
                    >,
                    EnvFilter,
                    Registry,
                >,
                Registry,
            >,
        >,
        Layered<
            Filtered<
                tracing_subscriber::fmt::Layer<
                    Registry,
                    DefaultFields,
                    tracing_subscriber::fmt::format::Format<
                        tracing_subscriber::fmt::format::Full,
                        (),
                    >,
                >,
                EnvFilter,
                Registry,
            >,
            Registry,
        >,
    >,
>;
pub(crate) fn set_up_tracing(widget_name: &str) -> HomeHandle {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").expect("runtime dir is not set");
    let logging_dir = runtime_dir + "/spell/";
    let socket_dir = logging_dir.clone() + "/spell.sock";
    // let socket_cli_dir = logging_dir.clone() + "/spell_cli";

    let _ = fs::create_dir(Path::new(&logging_dir));
    let _ = fs::remove_file(&socket_dir);
    // let _ = fs::File::create(&socket_cli_dir);

    let stream = UnixDatagram::unbound().unwrap();
    stream
        .set_nonblocking(true)
        .expect("Non blocking couldn't be set");

    let writer = RollingFileAppender::builder()
        .rotation(Rotation::HOURLY) // rotate log files once every hour
        .filename_prefix(widget_name) // log file names will be prefixed with `myapp.`
        .filename_suffix("log") // log file names will be suffixed with `.log`
        .build(&logging_dir) // try to build an appender that stores log files in `/var/log`
        .expect("initializing rolling file appender failed");

    // Logs to be stored in case of debugging.
    let layer_writer = fmt::layer()
        .without_time()
        .with_target(false)
        .with_writer(writer)
        .with_filter(EnvFilter::new("spell_framework=trace,info"));

    // Logs on socket read by cli.
    let layer_socket = fmt::Layer::default()
        .without_time()
        .with_target(false)
        .with_writer(Mutex::new(SocketWriter::new(stream)))
        .with_filter(EnvFilter::new("spell_framework=info, warn"));

    let (layer_env, handle) = LoadLayer::new(layer_socket);
    let subs = tracing_subscriber::registry()
        // Logs shown in stdout when program runs.
        .with(
            fmt::layer()
                .without_time()
                .with_target(false)
                .with_filter(EnvFilter::new("spell_framework=info, warn")),
        )
        // Logs for file.
        .with(layer_writer)
        // Logs for cli
        .with(layer_env);
    let _ = tracing::subscriber::set_global_default(subs);
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
