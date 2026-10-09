pub mod executor;
pub mod manager;
pub mod parser;

pub use executor::inspect_url;
pub use manager::DownloadManager;
pub use parser::{extract_platform_from_url, parse_progress_line, ProgressUpdate};
