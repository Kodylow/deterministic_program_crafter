use std::path::PathBuf;

use tracing::info;

pub struct Flake {
    pub flake_path: PathBuf,
}

impl Flake {
    pub fn new(crate_name: &str, work_dir: &PathBuf) -> Self {
        let flake_path = work_dir.join(crate_name).join("flake.nix");
        Flake { flake_path }
    }

    pub async fn ensure_flake_nix(
        &self,
        reference_flake_path: &PathBuf,
    ) -> Result<(), anyhow::Error> {
        info!("Reference flake path: {}", reference_flake_path.display());

        if !reference_flake_path.exists() {
            info!(
                "Reference flake.nix not found at {}",
                reference_flake_path.display()
            );
            return Ok(());
        }

        if self.flake_path.exists() {
            info!("Found a flake.nix at {}", self.flake_path.display());
        } else {
            info!("Creating flake.nix at {}", self.flake_path.display());
            let contents = std::fs::read_to_string(reference_flake_path).map_err(|e| {
                info!("Failed to read reference flake.nix: {}", e);
                e
            })?;
            std::fs::write(&self.flake_path, contents).map_err(|e| {
                info!(
                    "Failed to write to flake.nix at {}: {}",
                    self.flake_path.display(),
                    e
                );
                e
            })?;
        }
        Ok(())
    }
}
