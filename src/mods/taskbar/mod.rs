#![allow(dead_code)]

pub mod types;

use std::thread;
use std::time::{Duration, Instant};
use std::ptr;
use types::TaskbarSize;
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS},
    UI::{
        Shell::{SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA},
        WindowsAndMessaging::{
            EnumWindows, GetWindowRect, IsWindowVisible, IsZoomed, SetWindowPos,
            SystemParametersInfoW, SPI_GETWORKAREA, SPI_SETANIMATION, ANIMATIONINFO,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SPI_GETANIMATION,
            SWP_NOZORDER, SWP_NOACTIVATE, FindWindowW,
        },
    },
};

use windows::core::PCWSTR;
extern "system" fn enum_window(handle: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let programs = &mut *(lparam.0 as *mut Vec<HWND>);
        if IsWindowVisible(handle).as_bool() && IsZoomed(handle).as_bool() {
            programs.push(handle);
        }
        BOOL::from(true)
    }
}

pub fn taskbar_automation() {
    let mut programs: Vec<HWND> = Vec::new();

    unsafe {
        EnumWindows(Some(enum_window), LPARAM(&mut programs as *mut _ as isize)).expect("Failed to enumerate windows");
    }
    hide_taskbar(!programs.is_empty());
}

fn hide_taskbar(hide: bool) {
    unsafe {
        let original_animation = get_animation_info();
        set_animation_duration(100); // Faster taskbar animation

        let mut pdata = APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            ..Default::default()
        };

        let current_state = SHAppBarMessage(ABM_GETSTATE, &mut pdata) as u32;
        let new_state = if hide { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP };

        if current_state != new_state {
            pdata.lParam = LPARAM(new_state as isize);
            SHAppBarMessage(ABM_SETSTATE, &mut pdata);

            // Short delay to allow taskbar to start moving
            thread::sleep(Duration::from_millis(50));

            // Animate program windows
            animate_programs(hide);
        }

        // Restore original animation settings
        set_animation_info(&original_animation);
    }
}

unsafe fn animate_programs(expand: bool) {
    let mut programs: Vec<HWND> = Vec::new();
    EnumWindows(
        Some(enum_window),
        LPARAM(&mut programs as *mut _ as isize),
    ).expect("Failed to enumerate windows");

    let taskbar_size = get_taskbar_size();
    let mut work_area = RECT::default();
    SystemParametersInfoW(
        SPI_GETWORKAREA,
        0,
        Some(&mut work_area as *mut _ as _),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    ).expect("Failed to get work area");

    let animation_duration = Duration::from_millis(300);
    let start_time = Instant::now();

    // Find the desktop window
    let desktop_window = FindWindowW(
        PCWSTR::from_raw(windows::core::w!("Progman").as_ptr()),
        PCWSTR::null()
    ).expect("Failed to find desktop window");

    while start_time.elapsed() < animation_duration {
        let progress = start_time.elapsed().as_secs_f32() / animation_duration.as_secs_f32();
        let eased_progress = ease_in_out_cubic(progress);

        for &hwnd in &programs {
            // Skip the desktop window
            if hwnd == desktop_window {
                continue;
            }

            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).expect("Failed to get window rect");

            let start_bottom = if expand { work_area.bottom } else { work_area.bottom + taskbar_size.height as i32 };
            let end_bottom = if expand { work_area.bottom + taskbar_size.height as i32 } else { work_area.bottom };

            let current_bottom = start_bottom + ((end_bottom - start_bottom) as f32 * eased_progress) as i32;

            SetWindowPos(
                hwnd,
                HWND(ptr::null_mut()),
                rect.left,
                rect.top,
                rect.right - rect.left,
                current_bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            ).expect("Failed to set window position");
        }

        thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }

    // Ensure final position
    for &hwnd in &programs {
        // Skip the desktop window
        if hwnd == desktop_window {
            continue;
        }

        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).expect("Failed to get window rect");

        let final_bottom = if expand { work_area.bottom + taskbar_size.height as i32 } else { work_area.bottom };

        SetWindowPos(
            hwnd,
            HWND(ptr::null_mut()),
            rect.left,
            rect.top,
            rect.right - rect.left,
            final_bottom - rect.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        ).expect("Failed to set window position");
    }
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

unsafe fn get_animation_info() -> ANIMATIONINFO {
    let mut info = ANIMATIONINFO {
        cbSize: std::mem::size_of::<ANIMATIONINFO>() as u32,
        iMinAnimate: 0,
    };
    SystemParametersInfoW(
        SPI_GETANIMATION,
        std::mem::size_of::<ANIMATIONINFO>() as u32,
        Some(&mut info as *mut _ as _),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    ).expect("Failed to get animation info");
    info
}

unsafe fn set_animation_info(info: &ANIMATIONINFO) {
    SystemParametersInfoW(
        SPI_SETANIMATION,
        std::mem::size_of::<ANIMATIONINFO>() as u32,
        Some(info as *const _ as _),
        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
    ).expect("Failed to set animation info");
}

unsafe fn set_animation_duration(duration: i32) {
    let mut info = get_animation_info();
    info.iMinAnimate = duration;
    set_animation_info(&info);
}

pub fn get_taskbar_size() -> TaskbarSize {
    unsafe {
        let mut workarea = RECT::default();
        let mut screen_size = DEVMODEW::default();

        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut workarea as *mut _ as _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        ).expect("Cannot get work area size");

        EnumDisplaySettingsW(None, ENUM_CURRENT_SETTINGS, &mut screen_size)
            .expect("Cannot get screen size");

        TaskbarSize {
            height: (screen_size.dmPelsHeight - workarea.bottom as u32).max(0),
            width: (screen_size.dmPelsWidth - workarea.right as u32).max(0),
        }
    }
}