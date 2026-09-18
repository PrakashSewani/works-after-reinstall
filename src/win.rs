use std::ffi::OsStr;
use std::iter::once;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;

use winapi::shared::minwindef::{DWORD, LPARAM, LPVOID, WPARAM};
use winapi::shared::ntdef::HANDLE;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
use winapi::um::securitybaseapi::GetTokenInformation;
use winapi::um::winnt::{TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use winapi::um::winuser::{
    HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
};

pub fn is_admin() -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned: DWORD = 0;
        let read = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut TOKEN_ELEVATION as LPVOID,
            size_of::<TOKEN_ELEVATION>() as DWORD,
            &mut returned,
        );
        CloseHandle(token);

        read != 0 && elevation.TokenIsElevated != 0
    }
}

pub fn broadcast_setting_change(area: &str) {
    let area: Vec<u16> = OsStr::new(area).encode_wide().chain(once(0)).collect();
    unsafe {
        let mut result: usize = 0;
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            area.as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            1000,
            &mut result,
        );
    }
}
