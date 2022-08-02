#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
pub fn source_string(f: &std::fs::File) -> String {
    use filepath::FilePath;
    use url::Url;
    match f.path() {
        Ok(path) => match Url::from_file_path(&path) {
            Ok(url) => url.to_string(),
            Err(_) => path.display().to_string(),
        },
        Err(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::io::AsRawFd;
                format!("File Descriptor: {:x}", f.as_raw_fd())
            }
            #[cfg(windows)]
            {
                use std::os::windows::io::AsRawHandle;
                format!("File Handle: {:p}", f.as_raw_handle())
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
pub fn source_string(f: &std::fs::File) -> String {
    use std::os::unix::io::AsRawFd;
    format!("File Descriptor: {:x}", f.as_raw_fd())
}
