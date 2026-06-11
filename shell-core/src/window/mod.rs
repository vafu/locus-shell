mod config;
mod layer;

pub use config::{Anchors, Edge, ExclusiveZone, Layer, SurfaceMargins, WindowConfig};
pub use layer::{apply_layer_shell_config, create_layer_window};

#[cfg(test)]
mod test;
