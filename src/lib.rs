// Poly MCP Library
// This crate provides MCP (Model Context Protocol) modules that can be integrated into other applications

pub mod modules;

// Re-export commonly used items
pub use modules::{
    clipboard::ClipboardModule,
    filesystem::FilesystemModule,
    diagnostics::DiagnosticsModule,
    silent::SilentModule,
    time::TimeModule,
    network::NetworkModule,
    context::ContextModule,
    git::GitModule,
    input::InputModule,
    transform::TransformModule,
};

/// VARP premium integration — spawns `varp-bridge` binary at runtime.
/// No VARP source dependency. Requires: varp-bridge in PATH + VARP_LICENSE_KEY env.
#[cfg(feature = "premium")]
pub use modules::varp_bridge::VarpModule;
