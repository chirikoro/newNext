use std::path::Path;

/// Build the project for production (SSG pages)
pub async fn run(output: &str) {
    tracing::info!("🔨 Building Hayabusa project for production...");
    tracing::info!("   Output directory: {}", output);

    // Ensure output directory exists
    if let Err(e) = std::fs::create_dir_all(output) {
        tracing::error!("Failed to create output directory: {}", e);
        return;
    }

    // Run cargo build in release mode
    let status = std::process::Command::new("cargo")
        .args(["build", "--release"])
        .status();

    match status {
        Ok(s) if s.success() => {
            tracing::info!("   Release build successful.");

            // Copy public/ assets to output directory
            let public_dir = Path::new("public");
            if public_dir.exists() {
                copy_dir_recursive(public_dir, Path::new(output))
                    .expect("Failed to copy public assets");
                tracing::info!("   Static assets copied to {}/", output);
            }

            tracing::info!("✅ Production build complete!");
            tracing::info!("   Run `hayabusa start` to start the production server");
        }
        _ => {
            tracing::error!("   Build failed. Please check your code.");
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
