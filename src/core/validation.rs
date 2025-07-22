use anyhow::{anyhow, Result};
use crate::core::config::is_in_project;

/// Ensure we're in a project with .off-context initialized
pub fn ensure_project_initialized() -> Result<()> {
    if !is_in_project() {
        return Err(anyhow!(
            "❌ Project not initialized. Run 'off-context init' first.\n\
             💡 This command requires a local .off-context directory in the project."
        ));
    }
    Ok(())
}