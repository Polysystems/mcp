// Poly MCP Library
// This crate provides MCP (Model Context Protocol) modules that can be integrated into other applications

pub mod modules;

// Re-export commonly used items
pub use modules::{
    filesystem::FilesystemModule,
    diagnostics::DiagnosticsModule,
    silent::SilentModule,
    time::TimeModule,
    network::NetworkModule,
    context::ContextModule,
    git::GitModule,
    input::InputModule,
};
