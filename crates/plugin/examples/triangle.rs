//! Triangle overlay example. Mirrors `plugin/examples/hello_triangle/triangle.c`.

use gsr_plugin::Plugin;

/// Minimal example plugin.
pub struct Triangle;

impl Plugin for Triangle {
    fn name(&self) -> &str {
        "triangle"
    }
}

fn main() {
    let plugin = Triangle;
    println!("example plugin: {}", plugin.name());
}
