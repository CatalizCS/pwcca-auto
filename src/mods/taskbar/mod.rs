#![allow(dead_code)]

pub mod types;

use types::TaskbarSize;
use windows::Win32::{
    Foundation::{BOOL, FALSE, HWND, LPARAM, RECT, TRUE},
    Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS},
    UI::{
        Shell::{
            SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA,
        },
        WindowsAndMessaging::{
            EnumWindows, IsWindowVisible, IsZoomed, SystemParametersInfoW, SPI_GETWORKAREA,
            SPI_SETANIMATION, ANIMATIONINFO, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        },
    },
};

extern "system" fn enum_window(handle: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let programs = &mut *(lparam.0 as *mut Vec<HWND>);
        if IsWindowVisible(handle) == TRUE && IsZoomed(handle) == TRUE {
            programs.push(handle);
        }
        TRUE
    }
}

pub fn taskbar_automation() {
    let mut programs: Vec<HWND> = Vec::new();

    unsafe {
        let _ = EnumWindows(
            Some(enum_window),
            LPARAM(&mut programs as *mut _ as isize),
        );
    };
    hide_taskbar(!programs.is_empty());
}

fn hide_taskbar(hide: bool) {
    // Disable taskbar animation
    let mut animation_info = ANIMATIONINFO {
        cbSize: std::mem::size_of::<ANIMATIONINFO>() as u32,
        iMinAnimate: 0, // Disable animation
    };
    unsafe {
        SystemParametersInfoW(
            SPI_SETANIMATION,
            std::mem::size_of::<ANIMATIONINFO>() as u32,
            Some(&mut animation_info as *mut _ as _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }

    let mut pdata = APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        ..Default::default()
    };
    unsafe { SHAppBarMessage(ABM_GETSTATE, &mut pdata) };

    let current_state = pdata.lParam.0 as u32;
    let new_state = if hide { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP };

    if current_state != new_state {
        pdata.lParam = LPARAM(new_state as isize);
        let _ = unsafe { SHAppBarMessage(ABM_SETSTATE, &mut pdata) };
    }

    // Re-enable taskbar animation
    animation_info.iMinAnimate = 1; // Enable animation
    unsafe {
        SystemParametersInfoW(
            SPI_SETANIMATION,
            std::mem::size_of::<ANIMATIONINFO>() as u32,
            Some(&mut animation_info as *mut _ as _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
}

pub fn get_taskbar_size() -> TaskbarSize {
    let mut workarea = RECT::default();
    let mut screen_size = DEVMODEW::default();

    let mut taskbar = TaskbarSize::default();

    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut workarea as *mut _ as _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .expect("Cannot get work area size");

        EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut screen_size)
            .expect("Cannot get screen size");
    };

    if workarea.bottom != screen_size.dmPelsHeight as i32 {
        taskbar.height = screen_size.dmPelsHeight - workarea.bottom as u32;
    }

    if workarea.right != screen_size.dmPelsWidth as i32 {
        taskbar.width = screen_size.dmPelsWidth - workarea.right as u32;
    }

    taskbar
}