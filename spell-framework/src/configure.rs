use crate::layer_properties::popup::{PopupAnchor, PopupGravity};
use smithay_client_toolkit::{
    shell::{
        wlr_layer::{Anchor, KeyboardInteractivity, Layer},
        xdg::popup::Popup,
    },
    shm::slot::{Buffer, SlotPool},
};
use std::{
    cell::{Cell, RefCell},
    fs,
    io::Write,
    os::unix::net::UnixDatagram,
    path::Path,
    rc::Rc,
    sync::Mutex,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer as TracingTraitLayer,
    filter::Filtered,
    fmt::{self, format::DefaultFields},
    layer::{Layered, SubscriberExt},
    registry::Registry,
    reload::Layer as LoadLayer,
};

pub struct PopupSettings {
    pub(crate) pool: Rc<RefCell<SlotPool>>,
    pub(crate) popup: Popup,
    pub(crate) popup_conf: PopupConf,
    pub(crate) buffer: Buffer,
}

pub struct PopupConf {
    pub width: u32,
    pub height: u32,
    pub anchor: PopupAnchor,
    pub gravity: PopupGravity,
    pub anchor_rect: (i32, i32, i32, i32),
}

impl From<u32> for Dimension {
    fn from(value: u32) -> Self {
        Dimension::Pixel(value)
    }
}

/// This enum provides multiple ways for defininf the Dimensions of a widget. Thus
/// making the prcess dynamic rather than being anchored to just pixels. if `Full`
/// or `Percentage` is provided in Dimension, output name is compulsary to be defined.
/// To maintain backward compatibility, Dimension implements into from `u32` in which
/// case it simply returns an instant of [`Dimension::Pixel`].
#[derive(Debug, Clone, Default)]
pub enum Dimension {
    /// Full screen Dimension of the selected monitor.
    #[default]
    Full,
    /// Whole number percentage Dimension of width/height, relative to the selected monitor.
    Percentage(u32),
    /// Definition of widgets in static pixels.
    Pixel(u32),
}

// FIXME: Privatise WindowConf values and move the documentation to WindowConfBuilder.

/// WindowConf is an essential struct passed on to widget constructor functions (like invoke_spell
/// of generated code) for defining the specifications of the widget.
///
/// ## Panics
///
/// 1. Event loops ([cast_spell](crate::cast_spell)) will panic if 0 is provided as width or height.
/// 2. Builder will also panic if percentage or full is used without specifying the monitor explicitly.
#[derive(Debug, Clone)]
pub struct WindowConf {
    /// Defines the widget width in pixels, fullscreen width or pecentage width of full screen.
    /// On setting values greater than the provided pixels of
    /// monitor, the widget offsets from monitor's rectangular monitor space. It is important to
    /// note that the value should be the maximum width the widget will ever attain, not the
    /// current width in case of resizeable widgets. This value has full screen width as its default.
    pub width: Dimension,
    /// Defines the widget height in pixels, fullscreen width or pecentage height of full screen.
    /// On setting values greater than the provided pixels of
    /// monitor, the widget offsets from monitor's rectangular monitor space. It is important to
    /// note that the value should be the maximum height the widget will ever attain, not the
    /// current height in case of resizeable widgets. This value has full screen height as its default.
    pub height: Dimension,
    /// width calculated from provided Dimension of width. Not intended for external use.
    pub evaluated_width: u32,
    /// height provided from evaluated Dimension of height. Not intended for external use.
    pub evaluated_height: u32,
    /// Defines the Anchors to which the window needs to be attached. View [`Anchor`] for
    /// related explaination of usage. If both values are None, then widget is displayed in the
    /// center of screen.
    pub anchor: [Option<Anchor>; 4],
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
    /// Creates a builder instance for creation of WindowConf, to view defaults
    /// head over to documentation of [`WindowConf`]'s parameters.
    pub fn builder() -> WindowConfBuilder {
        WindowConfBuilder::default()
    }
}

/// A builder method for [`WindowConf`]. For default values, refer to parameters
/// of [`WindowConf`].
#[derive(Default)]
pub struct WindowConfBuilder {
    max_width: Dimension,
    max_height: Dimension,
    anchor: [Option<Anchor>; 4],
    margin: (i32, i32, i32, i32),
    layer_type: Option<Layer>,
    board_interactivity: KeyboardInteractivity,
    exclusive_zone: Option<i32>,
    monitor_name: Option<String>,
    natural_scroll: bool,
}

impl WindowConfBuilder {
    /// Sets [`WindowConf::width`].
    pub fn width<I: Into<Dimension>>(&mut self, width: I) -> &mut Self {
        let new = self;
        new.max_width = width.into();
        new
    }

    /// Sets [`WindowConf::height`].
    pub fn height<I: Into<Dimension>>(&mut self, height: I) -> &mut Self {
        let x = self;
        x.max_height = height.into();
        x
    }

    /// Sets first anchor of [`WindowConf::anchor`].
    pub fn anchor_1(&mut self, anchor: Anchor) -> &mut Self {
        let x = self;
        x.anchor[0] = Some(anchor);
        x
    }

    /// Sets second anchor of [`WindowConf::anchor`].
    pub fn anchor_2(&mut self, anchor: Anchor) -> &mut Self {
        let x = self;
        x.anchor[1] = Some(anchor);
        x
    }

    /// Sets third anchor of [`WindowConf::anchor`].
    pub fn anchor_3(&mut self, anchor: Anchor) -> &mut Self {
        let x = self;
        x.anchor[2] = Some(anchor);
        x
    }

    /// Sets fourth anchor of [`WindowConf::anchor`].
    pub fn anchor_4(&mut self, anchor: Anchor) -> &mut Self {
        let x = self;
        x.anchor[3] = Some(anchor);
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
    pub fn exclusive_zone(&mut self, dimension: i32) -> &mut Self {
        let x = self;
        x.exclusive_zone = Some(dimension);
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
    /// are set to zero or monitor is not specified when full or percentage dimension is used.
    pub fn build(&self) -> Result<WindowConf, Box<dyn std::error::Error>> {
        Ok(WindowConf {
            width: if let Dimension::Percentage(x) = self.max_width
                && x == 0
            {
                return Err("width is zero in percentage".into());
            } else if let Dimension::Pixel(y) = self.max_width
                && y == 0
            {
                return Err("width is zero in pixel".into());
            } else {
                self.max_width.clone()
            },
            height: if let Dimension::Percentage(x) = self.max_height
                && x == 0
            {
                return Err("height is zero in percentage".into());
            } else if let Dimension::Pixel(y) = self.max_height
                && y == 0
            {
                return Err("height is zero in pixel".into());
            } else {
                self.max_height.clone()
            },
            evaluated_width: 0,
            evaluated_height: 0,
            anchor: self.anchor,
            margin: self.margin,
            layer_type: match self.layer_type {
                None => Layer::Top,
                Some(val) => val,
            },
            board_interactivity: Cell::new(self.board_interactivity),
            exclusive_zone: self.exclusive_zone,
            monitor_name: {
                let needs_monitor =
                    matches!(self.max_width, Dimension::Full | Dimension::Percentage(_))
                        || matches!(self.max_height, Dimension::Full | Dimension::Percentage(_));

                if needs_monitor && self.monitor_name.is_none() {
                    return Err(
                        "Provide explicit monitor name if using Full or Percentage dimensions"
                            .into(),
                    );
                } else {
                    self.monitor_name.clone()
                }
            },
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
        .with_ansi(false)
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
