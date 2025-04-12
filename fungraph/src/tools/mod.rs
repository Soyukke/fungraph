mod tool;
pub use tool::*;

mod parameter;
pub use fungraph_derive::ToolParameters;
pub use parameter::*;

//#[cfg(feature = "mcp_tool")]
pub mod mcp_tool;
