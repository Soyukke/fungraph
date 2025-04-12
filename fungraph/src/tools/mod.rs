mod tool;
pub use tool::*;

mod parameter;
pub use fungraph_derive::ToolParameters;
pub use parameter::*;

//#[cfg(feature = "mcp_tool")]
mod mcp_tool;
//#[cfg(feature = "mcp_tool")]
use mcp_tool::*;
