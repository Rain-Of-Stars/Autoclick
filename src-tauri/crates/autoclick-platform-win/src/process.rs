use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::Threading::{
            OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
    },
    core::PWSTR,
};

use crate::PlatformError;

pub fn resolve_process_path(pid: u32) -> Result<Option<String>, PlatformError> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .map_err(|err| PlatformError::Win32(err.to_string()))?;
        let path = query_process_path(handle)?;
        let _ = CloseHandle(handle);
        Ok(path)
    }
}

unsafe fn query_process_path(handle: HANDLE) -> Result<Option<String>, PlatformError> {
    if handle.is_invalid() {
        return Ok(None);
    }

    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(|err| PlatformError::Win32(err.to_string()));

    result?;
    Ok(Some(String::from_utf16_lossy(&buffer[..length as usize])))
}

pub fn process_name_from_path(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::process_name_from_path;

    #[test]
    fn extracts_process_name_from_path() {
        let process_name = process_name_from_path("apps/example.exe");
        assert_eq!(process_name.as_deref(), Some("example.exe"));
    }
}
