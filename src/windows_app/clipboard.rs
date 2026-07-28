use std::thread;
use std::time::{Duration, Instant};
use windows::core::Error;
use windows::Win32::Foundation::{GlobalFree, HANDLE};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
};

struct ClipboardGuard;

const CF_UNICODETEXT_VALUE: u32 = 13;

impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

fn open_with_retry() -> Result<ClipboardGuard, Error> {
    let started = Instant::now();
    loop {
        if unsafe { OpenClipboard(None) }.is_ok() {
            return Ok(ClipboardGuard);
        }
        if started.elapsed() >= Duration::from_millis(200) {
            return Err(Error::from_thread());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

pub fn write_text(text: &str) -> Result<(), Error> {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * std::mem::size_of::<u16>();
    let memory = unsafe { GlobalAlloc(GMEM_MOVEABLE, byte_len) }?;
    let pointer = unsafe { GlobalLock(memory) };
    if pointer.is_null() {
        unsafe {
            let _ = GlobalFree(Some(memory));
        }
        return Err(Error::from_thread());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr().cast::<u8>(), pointer.cast::<u8>(), byte_len);
        let _ = GlobalUnlock(memory);
    }

    let _guard = match open_with_retry() {
        Ok(guard) => guard,
        Err(error) => {
            unsafe {
                let _ = GlobalFree(Some(memory));
            }
            return Err(error);
        }
    };
    unsafe { EmptyClipboard()? };
    if unsafe { SetClipboardData(CF_UNICODETEXT_VALUE, Some(HANDLE(memory.0))) }.is_err() {
        unsafe {
            let _ = GlobalFree(Some(memory));
        }
        return Err(Error::from_thread());
    }
    // Ownership of memory transfers to the system after SetClipboardData succeeds.
    Ok(())
}

pub fn replay_ctrl_a(count: u8) -> Result<(), Error> {
    let mut inputs = Vec::with_capacity(count as usize * 3);
    for _ in 0..count {
        inputs.extend([
            keyboard_input(VK_CONTROL.0, Default::default()),
            keyboard_input(b'A' as u16, Default::default()),
            keyboard_input(b'A' as u16, KEYEVENTF_KEYUP),
        ]);
    }
    send_inputs(&inputs)
}

fn send_inputs(inputs: &[INPUT]) -> Result<(), Error> {
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        return Err(Error::from_thread());
    }
    Ok(())
}

fn keyboard_input(
    key: u16,
    flags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS,
) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(key),
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}
