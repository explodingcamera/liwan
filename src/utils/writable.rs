use std::{fs, io::Write, path::Path};

/// Checks if the directory is writable by attempting to create a temporary file.
pub fn check_directory_writable(dir: &Path) {
    let test_path = dir.join(".liwan_write_test");

    match fs::OpenOptions::new().create_new(true).write(true).open(&test_path) {
        Ok(mut f) => {
            let _ = f.write_all(b"test");
            let _ = fs::remove_file(&test_path);
        }
        Err(err) => {
            #[cfg(unix)]
            {
                let uid = nix::unistd::geteuid();
                let gid = nix::unistd::getegid();

                let path = dir.display();
                tracing::warn!(
                    %path,
                    uid = uid.as_raw(),
                    gid = gid.as_raw(),
                    error = %err,
                    "Directory is not writable"
                );
            }

            #[cfg(not(unix))]
            {
                tracing::warn!(
                    path = dir.display(),
                    error = %err,
                    "Directory is not writable"
                );
            }
        }
    }
}
