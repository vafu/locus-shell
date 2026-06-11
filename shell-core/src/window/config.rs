#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct SurfaceMargins {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl SurfaceMargins {
    pub const ZERO: Self = Self {
        top: 0,
        right: 0,
        bottom: 0,
        left: 0,
    };

    pub const fn uniform(value: i32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Anchors {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl Anchors {
    pub const NONE: Self = Self {
        top: false,
        right: false,
        bottom: false,
        left: false,
    };

    pub const ALL: Self = Self {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };

    pub const fn new(top: bool, right: bool, bottom: bool, left: bool) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn with_edge(mut self, edge: Edge) -> Self {
        match edge {
            Edge::Top => self.top = true,
            Edge::Right => self.right = true,
            Edge::Bottom => self.bottom = true,
            Edge::Left => self.left = true,
        }
        self
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExclusiveZone {
    None,
    Fixed(i32),
    Auto,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct WindowConfig {
    pub layer: Layer,
    pub anchors: Anchors,
    /// Layer-shell surface offsets from screen edges.
    ///
    /// These affect compositor placement and exclusive-zone behavior. Consumers
    /// should use CSS margins/padding for spacing inside the GTK window.
    pub surface_margins: SurfaceMargins,
    pub exclusive_zone: ExclusiveZone,
    pub namespace: Option<&'static str>,
    pub keyboard_interactive: bool,
}

impl WindowConfig {
    pub const fn new(layer: Layer) -> Self {
        Self {
            layer,
            anchors: Anchors::NONE,
            surface_margins: SurfaceMargins::ZERO,
            exclusive_zone: ExclusiveZone::None,
            namespace: None,
            keyboard_interactive: false,
        }
    }

    pub const fn with_anchors(mut self, anchors: Anchors) -> Self {
        self.anchors = anchors;
        self
    }

    pub const fn with_surface_margins(mut self, margins: SurfaceMargins) -> Self {
        self.surface_margins = margins;
        self
    }

    pub const fn with_exclusive_zone(mut self, exclusive_zone: ExclusiveZone) -> Self {
        self.exclusive_zone = exclusive_zone;
        self
    }

    pub const fn with_fixed_exclusive_zone(mut self, exclusive_zone: i32) -> Self {
        self.exclusive_zone = ExclusiveZone::Fixed(exclusive_zone);
        self
    }

    pub const fn with_auto_exclusive_zone(mut self) -> Self {
        self.exclusive_zone = ExclusiveZone::Auto;
        self
    }

    pub const fn with_namespace(mut self, namespace: &'static str) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub const fn with_keyboard_interactivity(mut self, interactive: bool) -> Self {
        self.keyboard_interactive = interactive;
        self
    }
}
