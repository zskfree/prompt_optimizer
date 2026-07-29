mod clipboard;
mod selection;
mod settings;
mod startup;

use prompt_optimizer::api::ApiClient;
use prompt_optimizer::config::{self, Config, ConfigError};
use prompt_optimizer::hotkey::{parse_hotkey, HotkeyKind, HotkeySpec};
use std::ffi::c_void;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use std::thread;
use windows::core::{w, Error as WindowsError, PCWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, COLORREF, ERROR_ALREADY_EXISTS, HINSTANCE, HWND, LPARAM, LRESULT,
    POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateRoundRectRgn, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
    GetMonitorInfoW, GetStockObject, InvalidateRect, MapWindowPoints, MonitorFromRect,
    SelectObject, SetBkMode, SetTextColor, SetWindowRgn, UpdateWindow, DEFAULT_GUI_FONT, DT_CENTER,
    DT_END_ELLIPSIS, DT_SINGLELINE, DT_VCENTER, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
    TRANSPARENT,
};
use windows::Win32::System::Diagnostics::Debug::MessageBeep;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, VK_CONTROL,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyIcon, DestroyMenu, DestroyWindow, DispatchMessageW, GetClientRect,
    GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetMessageW, GetWindowLongPtrW,
    GetWindowRect, GetWindowThreadProcessId, IsWindow, KillTimer, LoadCursorW, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    SetLayeredWindowAttributes, SetTimer, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
    SetWindowsHookExW, ShowWindow, TrackPopupMenu, TranslateMessage, UnhookWindowsHookEx,
    CREATESTRUCTW, CW_USEDEFAULT, GUITHREADINFO, GWLP_USERDATA, HHOOK, HICON, HWND_TOPMOST,
    IDC_ARROW, IMAGE_FLAGS, KBDLLHOOKSTRUCT, LLKHF_INJECTED, LR_DEFAULTCOLOR, LWA_ALPHA,
    MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MF_SEPARATOR, MF_STRING, MSG, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SW_HIDE, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WH_KEYBOARD_LL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CONTEXTMENU, WM_DESTROY,
    WM_ERASEBKGND, WM_HOTKEY, WM_KEYDOWN, WM_KEYUP, WM_NCCREATE, WM_NULL, WM_PAINT, WM_RBUTTONUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const APP_NAME: PCWSTR = w!("PromptOptimizer");
const WINDOW_CLASS: PCWSTR = w!("PromptOptimizer.HiddenWindow");
const STATUS_WINDOW_CLASS: PCWSTR = w!("PromptOptimizer.StatusPopup");
const HOTKEY_ID: i32 = 0x504F;
const TRAY_ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;
const WM_WORKER_DONE: u32 = WM_APP + 2;
const WM_GESTURE_HOTKEY: u32 = WM_APP + 3;
const MENU_SETTINGS: u32 = 1001;
const MENU_RELOAD: u32 = 1002;
const MENU_EXIT: u32 = 1003;
const STATUS_HEIGHT: i32 = 30;
const STATUS_MIN_WIDTH: i32 = 112;
const STATUS_MAX_WIDTH: i32 = 300;
const ICON_FILE: &[u8] = include_bytes!("../assets/prompt-optimizer.ico");
const GESTURE_TIMER_ID: usize = 2;
const GESTURE_INTERVAL_MS: u32 = 520;

#[derive(Clone, Copy)]
enum PopupSide {
    Right,
    Left,
}

#[derive(Clone, Copy)]
struct PopupAnchor {
    side: PopupSide,
    edge_x: i32,
    y: i32,
    max_width: i32,
    work_left: i32,
    work_right: i32,
}

struct StatusPopupState {
    hwnd: isize,
    visible: bool,
    anchor: Option<PopupAnchor>,
}

static STATUS_POPUP: Mutex<StatusPopupState> = Mutex::new(StatusPopupState {
    hwnd: 0,
    visible: false,
    anchor: None,
});

struct GestureHookState {
    hook: isize,
    hwnd: isize,
    required_taps: u8,
    taps: u8,
    a_down: bool,
    last_tap_time: u32,
}

fn advance_gesture_tap(
    current_taps: u8,
    last_tap_time: u32,
    now: u32,
    required_taps: u8,
) -> (u8, u8, bool) {
    let expired = current_taps > 0 && now.wrapping_sub(last_tap_time) > GESTURE_INTERVAL_MS;
    let replay = if expired { current_taps } else { 0 };
    let taps = if expired {
        1
    } else {
        current_taps.saturating_add(1)
    };
    if taps >= required_taps {
        (0, replay, true)
    } else {
        (taps, replay, false)
    }
}

static GESTURE_HOOK: Mutex<GestureHookState> = Mutex::new(GestureHookState {
    hook: 0,
    hwnd: 0,
    required_taps: 0,
    taps: 0,
    a_down: false,
    last_tap_time: 0,
});

impl PopupAnchor {
    fn position(self, width: i32) -> (i32, i32) {
        let width = width.min(self.max_width);
        let x = match self.side {
            PopupSide::Right => self.edge_x,
            PopupSide::Left => self.edge_x - width,
        }
        .clamp(
            self.work_left,
            (self.work_right - width).max(self.work_left),
        );
        (x, self.y)
    }
}

#[derive(Debug)]
pub struct AppError(String);

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AppError {}

impl From<WindowsError> for AppError {
    fn from(value: WindowsError) -> Self {
        Self(value.to_string())
    }
}

enum WorkerCommand {
    Optimize {
        task_id: u64,
        config: Config,
        text: String,
    },
    TestApi {
        settings_hwnd: isize,
        config: Config,
    },
    Shutdown,
}

enum WorkerResult {
    Optimize {
        task_id: u64,
        result: Result<String, String>,
    },
    TestApi {
        settings_hwnd: isize,
        result: Result<(), String>,
    },
}

struct AppState {
    config: Config,
    config_path: PathBuf,
    exe_path: PathBuf,
    hotkey: HotkeySpec,
    hotkey_registered: bool,
    busy: bool,
    next_task_id: u64,
    active_task_id: Option<u64>,
    worker_tx: Sender<WorkerCommand>,
    worker_rx: Receiver<WorkerResult>,
    icon: HICON,
    taskbar_created: u32,
    settings_hwnd: HWND,
}

pub fn run() -> Result<(), AppError> {
    let mutex = unsafe { CreateMutexW(None, true, w!("Local\\PromptOptimizer.SingleInstance")) }?;
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            MessageBoxW(
                None,
                w!("PromptOptimizer 已在运行。"),
                APP_NAME,
                MB_OK | MB_ICONINFORMATION,
            );
            let _ = CloseHandle(mutex);
        }
        return Ok(());
    }

    let exe_path = std::env::current_exe().map_err(|error| AppError(error.to_string()))?;
    let config_path = exe_path.with_file_name("config.json");
    let (config, first_run, startup_warning) = load_startup_config(&config_path)?;
    let hotkey = parse_hotkey(&config.hotkey).map_err(|error| AppError(error.to_string()))?;

    let instance = HINSTANCE(unsafe { GetModuleHandleW(None) }?.0);
    register_window_class(instance)?;
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WINDOW_CLASS,
            APP_NAME,
            WINDOW_STYLE::default(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(instance),
            None,
        )
    }?;

    let icon = create_embedded_icon(32)?;
    let (command_tx, command_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    start_worker(command_rx, result_tx, hwnd.0 as isize);

    let mut state = Box::new(AppState {
        config,
        config_path,
        exe_path,
        hotkey,
        hotkey_registered: false,
        busy: false,
        next_task_id: 1,
        active_task_id: None,
        worker_tx: command_tx,
        worker_rx: result_rx,
        icon,
        taskbar_created: unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        settings_hwnd: HWND::default(),
    });
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *state as *mut AppState) as isize);
    }

    add_tray_icon(hwnd, &state)?;
    state.hotkey_registered = activate_hotkey(hwnd, &state.hotkey).is_ok();
    if !state.hotkey_registered {
        notify(
            hwnd,
            state.icon,
            "错误",
            "热键注册失败，请修改配置后重新加载",
            true,
        );
    }
    if let Err(error) = startup::set_auto_start(state.config.auto_start, &state.exe_path) {
        notify(
            hwnd,
            state.icon,
            "开机自启设置失败",
            &error.to_string(),
            true,
        );
    }
    if let Some(warning) = startup_warning {
        notify(hwnd, state.icon, "配置已重置", &warning, true);
    } else if first_run {
        notify(
            hwnd,
            state.icon,
            "首次运行",
            "已创建配置，请在设置中填写 API Key",
            false,
        );
    }
    if first_run {
        unsafe {
            open_config(hwnd, &mut state);
        }
    }

    let mut message = MSG::default();
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    }
    let _ = state.worker_tx.send(WorkerCommand::Shutdown);
    if state.hotkey_registered {
        deactivate_hotkey(hwnd, &state.hotkey);
    }
    delete_tray_icon(hwnd);
    unsafe {
        if !state.settings_hwnd.is_invalid() && IsWindow(Some(state.settings_hwnd)).as_bool() {
            let _ = DestroyWindow(state.settings_hwnd);
        }
        let _ = DestroyIcon(state.icon);
        let _ = CloseHandle(mutex);
    }
    Ok(())
}

fn load_startup_config(path: &std::path::Path) -> Result<(Config, bool, Option<String>), AppError> {
    match config::load_or_create(path) {
        Ok((config, created)) => Ok((config, created, None)),
        Err(error @ ConfigError::InvalidJson { .. }) => {
            let warning = error.to_string();
            let config = config::load_existing(path).map_err(|next| AppError(next.to_string()))?;
            Ok((config, true, Some(warning)))
        }
        Err(error) => Err(AppError(error.to_string())),
    }
}

fn register_window_class(instance: HINSTANCE) -> Result<(), AppError> {
    let class = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
        hInstance: instance,
        lpszClassName: WINDOW_CLASS,
        lpfnWndProc: Some(window_proc),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(AppError(WindowsError::from_thread().to_string()));
    }
    let status_class = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
        hInstance: instance,
        lpszClassName: STATUS_WINDOW_CLASS,
        lpfnWndProc: Some(status_window_proc),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&status_class) } == 0 {
        return Err(AppError(WindowsError::from_thread().to_string()));
    }
    settings::register(instance).map_err(AppError::from)?;
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam.0 as *const CREATESTRUCTW;
        if !create.is_null() && !(*create).lpCreateParams.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize);
        }
    }

    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if !pointer.is_null() {
        let state = &mut *pointer;
        if message == state.taskbar_created {
            let _ = add_tray_icon(hwnd, state);
            return LRESULT(0);
        }
        match message {
            WM_HOTKEY if wparam.0 == HOTKEY_ID as usize => {
                on_hotkey(hwnd, state);
                return LRESULT(0);
            }
            WM_GESTURE_HOTKEY => {
                on_hotkey(hwnd, state);
                return LRESULT(0);
            }
            WM_TIMER if wparam.0 == GESTURE_TIMER_ID => {
                replay_pending_gesture(hwnd);
                return LRESULT(0);
            }
            WM_TRAY => {
                let event = lparam.0 as u32;
                if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                    show_tray_menu(hwnd, state);
                }
                return LRESULT(0);
            }
            WM_WORKER_DONE => {
                on_worker_done(hwnd, state);
                return LRESULT(0);
            }
            settings::WM_APPLY_CONFIG => {
                let request = lparam.0 as *mut settings::ApplyRequest;
                if request.is_null() {
                    return LRESULT(0);
                }
                match apply_config(hwnd, state, (*request).config.clone(), true) {
                    Ok(()) => {
                        (*request).error = None;
                        return LRESULT(1);
                    }
                    Err(error) => {
                        (*request).error = Some(error);
                        return LRESULT(0);
                    }
                }
            }
            settings::WM_TEST_API => {
                let request = lparam.0 as *mut settings::ApiTestRequest;
                if request.is_null() {
                    return LRESULT(0);
                }
                if state.busy {
                    (*request).error = Some("后台任务正在运行，请稍候".into());
                    return LRESULT(0);
                }
                let settings_hwnd = wparam.0 as isize;
                state.busy = true;
                update_tooltip(hwnd, state.icon, "正在测试 API…");
                if state
                    .worker_tx
                    .send(WorkerCommand::TestApi {
                        settings_hwnd,
                        config: (*request).config.clone(),
                    })
                    .is_err()
                {
                    state.busy = false;
                    (*request).error = Some("API 工作线程不可用".into());
                    return LRESULT(0);
                }
                (*request).error = None;
                return LRESULT(1);
            }
            settings::WM_SETTINGS_CLOSED => {
                if state.settings_hwnd.0 as usize == wparam.0 {
                    state.settings_hwnd = HWND::default();
                }
                return LRESULT(0);
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            _ => {}
        }
    } else if message == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

unsafe fn on_hotkey(hwnd: HWND, state: &mut AppState) {
    if state.busy {
        notify(
            hwnd,
            state.icon,
            "PromptOptimizer",
            "后台任务正在运行，请稍候",
            false,
        );
        return;
    }
    if state
        .config
        .active_api()
        .is_none_or(|api| api.api_key.trim().is_empty())
    {
        notify(
            hwnd,
            state.icon,
            "缺少 API Key",
            "请打开设置并填写当前 API 配置的 Key",
            true,
        );
        return;
    }
    match selection::read_selected_text() {
        Ok(Some(text)) => {
            let task_id = state.next_task_id;
            state.next_task_id = state.next_task_id.wrapping_add(1).max(1);
            state.busy = true;
            state.active_task_id = Some(task_id);
            update_tooltip(hwnd, state.icon, "正在优化…");
            show_status_popup("优化中…", false, true);
            if state
                .worker_tx
                .send(WorkerCommand::Optimize {
                    task_id,
                    config: state.config.clone(),
                    text,
                })
                .is_err()
            {
                state.busy = false;
                state.active_task_id = None;
                notify(hwnd, state.icon, "内部错误", "API 工作线程不可用", true);
            }
        }
        Ok(None) => notify(
            hwnd,
            state.icon,
            "PromptOptimizer",
            "未检测到选中文本",
            false,
        ),
        Err(error) => notify(hwnd, state.icon, "读取选区失败", &error.to_string(), true),
    }
}

unsafe fn on_worker_done(hwnd: HWND, state: &mut AppState) {
    let Ok(result) = state.worker_rx.try_recv() else {
        return;
    };
    match result {
        WorkerResult::Optimize { task_id, result } => {
            if state.active_task_id != Some(task_id) {
                return;
            }
            state.busy = false;
            state.active_task_id = None;
            update_tooltip(
                hwnd,
                state.icon,
                &format!("运行中 | 热键: {}", state.hotkey.display),
            );
            match result {
                Ok(text) => match clipboard::write_text(&text) {
                    Ok(()) => {
                        if state.config.play_sound {
                            let _ = MessageBeep(MB_OK);
                        }
                        notify(hwnd, state.icon, "PromptOptimizer", "已复制", false);
                    }
                    Err(error) => {
                        notify(hwnd, state.icon, "写入剪贴板失败", &error.to_string(), true)
                    }
                },
                Err(error) => notify(hwnd, state.icon, "优化失败", &error, true),
            }
        }
        WorkerResult::TestApi {
            settings_hwnd,
            result,
        } => {
            state.busy = false;
            update_tooltip(
                hwnd,
                state.icon,
                &format!("运行中 | 热键: {}", state.hotkey.display),
            );
            let settings_hwnd = HWND(settings_hwnd as *mut c_void);
            if state.settings_hwnd == settings_hwnd && IsWindow(Some(settings_hwnd)).as_bool() {
                settings::complete_api_test(settings_hwnd, result);
            }
        }
    }
}

unsafe fn show_tray_menu(hwnd: HWND, state: &mut AppState) {
    let Ok(menu) = CreatePopupMenu() else {
        return;
    };
    let _ = AppendMenuW(menu, MF_STRING, MENU_SETTINGS as usize, w!("设置"));
    let _ = AppendMenuW(menu, MF_STRING, MENU_RELOAD as usize, w!("重新加载配置"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
    let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT as usize, w!("退出"));
    let mut point = POINT::default();
    let _ = GetCursorPos(&mut point);
    let _ = SetForegroundWindow(hwnd);
    let command = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD,
        point.x,
        point.y,
        None,
        hwnd,
        None,
    );
    let _ = DestroyMenu(menu);
    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
    match command.0 as u32 {
        MENU_SETTINGS => open_config(hwnd, state),
        MENU_RELOAD => reload_config(hwnd, state),
        MENU_EXIT => {
            let _ = DestroyWindow(hwnd);
        }
        _ => {}
    }
}

unsafe fn open_config(hwnd: HWND, state: &mut AppState) {
    if !state.settings_hwnd.is_invalid() && IsWindow(Some(state.settings_hwnd)).as_bool() {
        settings::focus(state.settings_hwnd);
        return;
    }
    let instance = HINSTANCE(match GetModuleHandleW(None) {
        Ok(module) => module.0,
        Err(error) => {
            notify(hwnd, state.icon, "打开设置失败", &error.to_string(), true);
            return;
        }
    });
    match settings::show(hwnd, instance, state.icon, &state.config) {
        Ok(settings_hwnd) => state.settings_hwnd = settings_hwnd,
        Err(error) => notify(hwnd, state.icon, "打开设置失败", &error.to_string(), true),
    }
}

unsafe fn reload_config(hwnd: HWND, state: &mut AppState) {
    if state.busy {
        notify(
            hwnd,
            state.icon,
            "PromptOptimizer",
            "优化进行中，暂不能重载配置",
            false,
        );
        return;
    }
    let new_config = match config::load_existing(&state.config_path) {
        Ok(config) => config,
        Err(error) => {
            notify(hwnd, state.icon, "配置重载失败", &error.to_string(), true);
            return;
        }
    };
    match apply_config(hwnd, state, new_config, false) {
        Ok(()) => {
            if !state.settings_hwnd.is_invalid() && IsWindow(Some(state.settings_hwnd)).as_bool() {
                settings::refresh(state.settings_hwnd, &state.config);
            }
            notify(hwnd, state.icon, "PromptOptimizer", "配置已重新加载", false);
        }
        Err(error) => notify(hwnd, state.icon, "配置重载失败", &error, true),
    }
}

unsafe fn apply_config(
    hwnd: HWND,
    state: &mut AppState,
    new_config: Config,
    persist: bool,
) -> Result<(), String> {
    if state.busy {
        return Err("后台任务进行中，请完成后再保存配置".into());
    }
    new_config.validate().map_err(|error| error.to_string())?;
    let new_hotkey = parse_hotkey(&new_config.hotkey).map_err(|error| error.to_string())?;
    let old_hotkey = state.hotkey.clone();
    let old_registered = state.hotkey_registered;
    let old_auto_start = state.config.auto_start;
    let hotkey_changed = new_hotkey != old_hotkey || !old_registered;

    if hotkey_changed {
        if old_registered {
            deactivate_hotkey(hwnd, &old_hotkey);
        }
        if let Err(error) = activate_hotkey(hwnd, &new_hotkey) {
            state.hotkey_registered = old_registered && activate_hotkey(hwnd, &old_hotkey).is_ok();
            return Err(format!("新热键无法注册：{error}"));
        }
    }

    if let Err(error) = startup::set_auto_start(new_config.auto_start, &state.exe_path) {
        rollback_hotkey(
            hwnd,
            state,
            &new_hotkey,
            &old_hotkey,
            hotkey_changed,
            old_registered,
        );
        let _ = startup::set_auto_start(old_auto_start, &state.exe_path);
        return Err(format!("开机自启设置失败：{error}"));
    }

    if persist {
        if let Err(error) = config::save(&state.config_path, &new_config) {
            let _ = startup::set_auto_start(old_auto_start, &state.exe_path);
            rollback_hotkey(
                hwnd,
                state,
                &new_hotkey,
                &old_hotkey,
                hotkey_changed,
                old_registered,
            );
            return Err(error.to_string());
        }
    }

    state.config = new_config;
    state.hotkey = new_hotkey;
    state.hotkey_registered = if hotkey_changed { true } else { old_registered };
    update_tooltip(
        hwnd,
        state.icon,
        &format!("运行中 | 热键: {}", state.hotkey.display),
    );
    Ok(())
}

unsafe fn rollback_hotkey(
    hwnd: HWND,
    state: &mut AppState,
    new_hotkey: &HotkeySpec,
    old_hotkey: &HotkeySpec,
    hotkey_changed: bool,
    old_registered: bool,
) {
    if hotkey_changed {
        deactivate_hotkey(hwnd, new_hotkey);
        state.hotkey_registered = old_registered && activate_hotkey(hwnd, old_hotkey).is_ok();
    }
}

fn start_worker(
    command_rx: Receiver<WorkerCommand>,
    result_tx: Sender<WorkerResult>,
    hwnd_raw: isize,
) {
    thread::spawn(move || {
        let client = ApiClient::new();
        while let Ok(command) = command_rx.recv() {
            match command {
                WorkerCommand::Optimize {
                    task_id,
                    config,
                    text,
                } => {
                    let result = client
                        .optimize_request(&config, &text, task_id)
                        .map_err(|error| error.to_string());
                    if result_tx
                        .send(WorkerResult::Optimize { task_id, result })
                        .is_err()
                    {
                        break;
                    }
                    unsafe {
                        let hwnd = HWND(hwnd_raw as *mut c_void);
                        let _ = PostMessageW(Some(hwnd), WM_WORKER_DONE, WPARAM(0), LPARAM(0));
                    }
                }
                WorkerCommand::TestApi {
                    settings_hwnd,
                    config,
                } => {
                    let result = client
                        .test_connection(&config)
                        .map_err(|error| error.to_string());
                    if result_tx
                        .send(WorkerResult::TestApi {
                            settings_hwnd,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                    unsafe {
                        let hwnd = HWND(hwnd_raw as *mut c_void);
                        let _ = PostMessageW(Some(hwnd), WM_WORKER_DONE, WPARAM(0), LPARAM(0));
                    }
                }
                WorkerCommand::Shutdown => break,
            }
        }
    });
}

fn activate_hotkey(hwnd: HWND, hotkey: &HotkeySpec) -> Result<(), WindowsError> {
    match hotkey.kind {
        HotkeyKind::Chord {
            modifiers,
            virtual_key,
        } => unsafe {
            RegisterHotKey(
                Some(hwnd),
                HOTKEY_ID,
                HOT_KEY_MODIFIERS(modifiers),
                virtual_key,
            )
        },
        HotkeyKind::CtrlMultiTapA { taps } => install_gesture_hook(hwnd, taps),
    }
}

fn deactivate_hotkey(hwnd: HWND, hotkey: &HotkeySpec) {
    match hotkey.kind {
        HotkeyKind::Chord { .. } => unsafe {
            let _ = UnregisterHotKey(Some(hwnd), HOTKEY_ID);
        },
        HotkeyKind::CtrlMultiTapA { .. } => uninstall_gesture_hook(hwnd),
    }
}

fn install_gesture_hook(hwnd: HWND, required_taps: u8) -> Result<(), WindowsError> {
    let module = HINSTANCE(unsafe { GetModuleHandleW(None) }?.0);
    let hook =
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), Some(module), 0) }?;
    let mut state = GESTURE_HOOK
        .lock()
        .map_err(|_| WindowsError::from_thread())?;
    state.hook = hook.0 as isize;
    state.hwnd = hwnd.0 as isize;
    state.required_taps = required_taps;
    state.taps = 0;
    state.a_down = false;
    state.last_tap_time = 0;
    Ok(())
}

fn uninstall_gesture_hook(hwnd: HWND) {
    unsafe {
        let _ = KillTimer(Some(hwnd), GESTURE_TIMER_ID);
    }
    let hook = if let Ok(mut state) = GESTURE_HOOK.lock() {
        let hook = state.hook;
        *state = GestureHookState {
            hook: 0,
            hwnd: 0,
            required_taps: 0,
            taps: 0,
            a_down: false,
            last_tap_time: 0,
        };
        hook
    } else {
        0
    };
    if hook != 0 {
        unsafe {
            let _ = UnhookWindowsHookEx(HHOOK(hook as *mut c_void));
        }
    }
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 || lparam.0 == 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }
    let event = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
    if event.flags.0 & LLKHF_INJECTED.0 != 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let message = wparam.0 as u32;
    let key_down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
    let key_up = message == WM_KEYUP || message == WM_SYSKEYUP;
    let is_a = event.vkCode == b'A' as u32;
    let is_ctrl = matches!(event.vkCode, 0x11 | 0xA2 | 0xA3);
    let ctrl_down = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
    let mut replay = 0;
    let mut trigger_hwnd = 0;
    let mut suppress = false;

    if let Ok(mut state) = GESTURE_HOOK.lock() {
        if state.hook != 0 && is_a && key_down && ctrl_down {
            if !state.a_down {
                let (taps, expired_replay, triggered) = advance_gesture_tap(
                    state.taps,
                    state.last_tap_time,
                    event.time,
                    state.required_taps,
                );
                state.taps = taps;
                replay = expired_replay;
                state.a_down = true;
                state.last_tap_time = event.time;
                let hwnd = HWND(state.hwnd as *mut c_void);
                let _ = KillTimer(Some(hwnd), GESTURE_TIMER_ID);
                SetTimer(Some(hwnd), GESTURE_TIMER_ID, GESTURE_INTERVAL_MS, None);
                if triggered {
                    let _ = KillTimer(Some(hwnd), GESTURE_TIMER_ID);
                    trigger_hwnd = state.hwnd;
                }
            }
            suppress = true;
        } else if state.hook != 0 && is_a && key_up && state.a_down {
            state.a_down = false;
            suppress = true;
        } else if state.hook != 0
            && state.taps > 0
            && ((is_ctrl && key_up) || (key_down && !is_ctrl))
        {
            replay = state.taps;
            state.taps = 0;
            let hwnd = HWND(state.hwnd as *mut c_void);
            let _ = KillTimer(Some(hwnd), GESTURE_TIMER_ID);
        }
    }

    if replay > 0 {
        let _ = clipboard::replay_ctrl_a(replay);
    }
    if trigger_hwnd != 0 {
        let _ = PostMessageW(
            Some(HWND(trigger_hwnd as *mut c_void)),
            WM_GESTURE_HOTKEY,
            WPARAM(0),
            LPARAM(0),
        );
    }
    if suppress {
        LRESULT(1)
    } else {
        CallNextHookEx(None, code, wparam, lparam)
    }
}

fn replay_pending_gesture(hwnd: HWND) {
    unsafe {
        let _ = KillTimer(Some(hwnd), GESTURE_TIMER_ID);
    }
    let replay = if let Ok(mut state) = GESTURE_HOOK.lock() {
        let replay = state.taps;
        state.taps = 0;
        replay
    } else {
        0
    };
    if replay > 0 {
        let _ = clipboard::replay_ctrl_a(replay);
    }
}

fn add_tray_icon(hwnd: HWND, state: &AppState) -> Result<(), AppError> {
    let mut data = tray_data(hwnd, state.icon);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    copy_wide(
        &mut data.szTip,
        &format!("运行中 | 热键: {}", state.hotkey.display),
    );
    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        return Err(AppError(WindowsError::from_thread().to_string()));
    }
    Ok(())
}

fn delete_tray_icon(hwnd: HWND) {
    let data = tray_data(hwnd, HICON::default());
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn update_tooltip(hwnd: HWND, icon: HICON, text: &str) {
    let mut data = tray_data(hwnd, icon);
    data.uFlags = NIF_TIP;
    copy_wide(&mut data.szTip, text);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

fn notify(hwnd: HWND, icon: HICON, title: &str, message: &str, is_error: bool) {
    let _ = (hwnd, icon, title);
    show_status_popup(message, is_error, false);
}

fn show_status_popup(message: &str, is_error: bool, new_task: bool) {
    let concise: String = message.chars().take(80).collect();
    let width = status_text_width(&concise);
    let text = wide(&concise);
    let Ok(mut state) = STATUS_POPUP.lock() else {
        return;
    };
    if state.hwnd == 0 {
        let Ok(module) = (unsafe { GetModuleHandleW(None) }) else {
            return;
        };
        let popup = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(
                    WS_EX_TOPMOST.0 | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_LAYERED.0,
                ),
                STATUS_WINDOW_CLASS,
                PCWSTR(text.as_ptr()),
                WS_POPUP,
                0,
                0,
                width,
                STATUS_HEIGHT,
                None,
                None,
                Some(HINSTANCE(module.0)),
                None,
            )
        };
        let Ok(popup) = popup else {
            return;
        };
        state.hwnd = popup.0 as isize;
        unsafe {
            let _ = SetLayeredWindowAttributes(popup, COLORREF(0), 206, LWA_ALPHA);
        }
    }
    if new_task || !state.visible || state.anchor.is_none() {
        state.anchor = Some(capture_popup_anchor());
    }
    let popup = HWND(state.hwnd as *mut c_void);
    let anchor = state.anchor.unwrap_or_else(capture_popup_anchor);
    let display_width = width.min(anchor.max_width);
    let (x, y) = anchor.position(display_width);
    unsafe {
        let _ = SetWindowTextW(popup, PCWSTR(text.as_ptr()));
        let region = CreateRoundRectRgn(0, 0, display_width + 1, STATUS_HEIGHT + 1, 10, 10);
        let _ = SetWindowRgn(popup, Some(region), false);
        let _ = SetWindowPos(
            popup,
            Some(HWND_TOPMOST),
            x,
            y,
            display_width,
            STATUS_HEIGHT,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = InvalidateRect(Some(popup), None, false);
        let _ = UpdateWindow(popup);
        let _ = KillTimer(Some(popup), 1);
        let keep_open = new_task || message.starts_with("正在优化") || message == "优化中…";
        if !keep_open {
            let duration = if is_error { 2400 } else { 1200 };
            SetTimer(Some(popup), 1, duration, Some(status_popup_timer));
        }
    }
    state.visible = true;
}

fn status_text_width(message: &str) -> i32 {
    let text_width: i32 = message
        .chars()
        .map(|character| if character.is_ascii() { 7 } else { 14 })
        .sum();
    (text_width + 26).clamp(STATUS_MIN_WIDTH, STATUS_MAX_WIDTH)
}

fn capture_popup_anchor() -> PopupAnchor {
    let anchor = focused_input_rect().unwrap_or_else(|| {
        let mut point = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut point);
        }
        RECT {
            left: point.x,
            top: point.y,
            right: point.x + 1,
            bottom: point.y + 1,
        }
    });
    let monitor = unsafe { MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST) };
    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let work = if unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
        monitor_info.rcWork
    } else {
        anchor
    };

    let right_edge = anchor.right + 8;
    let left_edge = anchor.left - 8;
    let right_space = work.right - right_edge;
    let left_space = left_edge - work.left;
    let (side, edge_x, available) = if right_space >= left_space {
        (PopupSide::Right, right_edge, right_space)
    } else {
        (PopupSide::Left, left_edge, left_space)
    };
    let mut y = anchor.bottom + 8;
    if y + STATUS_HEIGHT > work.bottom {
        y = anchor.top - STATUS_HEIGHT - 8;
    }
    y = y.clamp(work.top, (work.bottom - STATUS_HEIGHT).max(work.top));
    PopupAnchor {
        side,
        edge_x,
        y,
        max_width: available.clamp(STATUS_MIN_WIDTH, STATUS_MAX_WIDTH),
        work_left: work.left,
        work_right: work.right,
    }
}

fn focused_input_rect() -> Option<RECT> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0.is_null() {
        return None;
    }
    let thread_id = unsafe { GetWindowThreadProcessId(foreground, None) };
    let mut info = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    if thread_id != 0 && unsafe { GetGUIThreadInfo(thread_id, &mut info) }.is_ok() {
        if !info.hwndCaret.0.is_null() {
            let mut points = [
                POINT {
                    x: info.rcCaret.left,
                    y: info.rcCaret.top,
                },
                POINT {
                    x: info.rcCaret.right,
                    y: info.rcCaret.bottom,
                },
            ];
            unsafe {
                MapWindowPoints(Some(info.hwndCaret), None, &mut points);
            }
            return Some(RECT {
                left: points[0].x,
                top: points[0].y,
                right: points[1].x.max(points[0].x + 1),
                bottom: points[1].y.max(points[0].y + 1),
            });
        }
        if !info.hwndFocus.0.is_null() {
            let mut rect = RECT::default();
            if unsafe { GetWindowRect(info.hwndFocus, &mut rect) }.is_ok() {
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;
                if width <= 1000 && height <= 500 {
                    return Some(rect);
                }
            }
        }
    }
    None
}

unsafe extern "system" fn status_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_PAINT => {
            paint_status_window(hwnd);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn paint_status_window(hwnd: HWND) {
    let mut paint = PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let mut rect = RECT::default();
    let _ = GetClientRect(hwnd, &mut rect);
    let background = COLORREF(0x003A_3A3A);
    let brush = CreateSolidBrush(background);
    FillRect(dc, &rect, brush);
    let _ = DeleteObject(brush.into());

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let previous = SelectObject(dc, font);
    let _ = SetBkMode(dc, TRANSPARENT);
    let _ = SetTextColor(dc, COLORREF(0x00F4_F4F4));
    let mut text = [0_u16; 96];
    let length = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut text);
    let mut text_rect = RECT {
        left: 10,
        top: 0,
        right: rect.right - 10,
        bottom: rect.bottom,
    };
    DrawTextW(
        dc,
        &mut text[..length.max(0) as usize],
        &mut text_rect,
        windows::Win32::Graphics::Gdi::DRAW_TEXT_FORMAT(
            DT_CENTER.0 | DT_END_ELLIPSIS.0 | DT_SINGLELINE.0 | DT_VCENTER.0,
        ),
    );
    let _ = SelectObject(dc, previous);
    let _ = EndPaint(hwnd, &paint);
}

unsafe extern "system" fn status_popup_timer(hwnd: HWND, _: u32, _: usize, _: u32) {
    let _ = KillTimer(Some(hwnd), 1);
    let _ = ShowWindow(hwnd, SW_HIDE);
    if let Ok(mut state) = STATUS_POPUP.lock() {
        if state.hwnd == hwnd.0 as isize {
            state.visible = false;
        }
    }
}

fn tray_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ID,
        hIcon: icon,
        ..Default::default()
    }
}

fn create_embedded_icon(size: i32) -> Result<HICON, AppError> {
    let resource = ico_image_nearest_to(ICON_FILE, size as u32)
        .ok_or_else(|| AppError("内嵌图标格式无效".into()))?;
    unsafe {
        CreateIconFromResourceEx(
            resource,
            true,
            0x0003_0000,
            size,
            size,
            IMAGE_FLAGS(LR_DEFAULTCOLOR.0),
        )
    }
    .map_err(AppError::from)
}

fn ico_image_nearest_to(bytes: &[u8], desired: u32) -> Option<&[u8]> {
    if bytes.len() < 6 || bytes[0..4] != [0, 0, 1, 0] {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..count)
        .filter_map(|index| {
            let entry = 6 + index * 16;
            let data = bytes.get(entry..entry + 16)?;
            let width = if data[0] == 0 { 256 } else { data[0] as u32 };
            let length = u32::from_le_bytes(data[8..12].try_into().ok()?) as usize;
            let offset = u32::from_le_bytes(data[12..16].try_into().ok()?) as usize;
            let resource = bytes.get(offset..offset.checked_add(length)?)?;
            Some((width.abs_diff(desired), resource))
        })
        .min_by_key(|(difference, _)| *difference)
        .map(|(_, resource)| resource)
}

fn copy_wide<const N: usize>(destination: &mut [u16; N], text: &str) {
    destination.fill(0);
    for (slot, value) in destination
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(text.encode_utf16())
    {
        *slot = value;
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn show_fatal_error(message: &str) {
    let wide_message = wide(message);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(wide_message.as_ptr()),
            APP_NAME,
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod status_popup_tests {
    use super::*;

    #[test]
    fn width_stays_within_compact_limits() {
        assert_eq!(status_text_width("已复制"), STATUS_MIN_WIDTH);
        assert_eq!(status_text_width(&"错误".repeat(40)), STATUS_MAX_WIDTH);
    }

    #[test]
    fn dynamic_width_keeps_the_anchor_edge_fixed() {
        let right = PopupAnchor {
            side: PopupSide::Right,
            edge_x: 100,
            y: 50,
            max_width: 300,
            work_left: 0,
            work_right: 800,
        };
        assert_eq!(right.position(112), (100, 50));
        assert_eq!(right.position(220), (100, 50));

        let left = PopupAnchor {
            side: PopupSide::Left,
            edge_x: 500,
            y: 50,
            max_width: 300,
            work_left: 0,
            work_right: 800,
        };
        assert_eq!(left.position(112).0 + 112, 500);
        assert_eq!(left.position(220).0 + 220, 500);
    }

    #[test]
    fn triple_tap_triggers_only_on_the_third_quick_press() {
        assert_eq!(advance_gesture_tap(0, 0, 100, 3), (1, 0, false));
        assert_eq!(advance_gesture_tap(1, 100, 250, 3), (2, 0, false));
        assert_eq!(advance_gesture_tap(2, 250, 400, 3), (0, 0, true));
    }

    #[test]
    fn expired_taps_are_replayed_before_starting_a_new_sequence() {
        assert_eq!(
            advance_gesture_tap(2, 100, 100 + GESTURE_INTERVAL_MS + 1, 3),
            (1, 2, false)
        );
    }

    #[test]
    fn embedded_ico_contains_small_and_large_icon_images() {
        assert!(ico_image_nearest_to(ICON_FILE, 16).is_some());
        assert!(ico_image_nearest_to(ICON_FILE, 256).is_some());
        assert!(ico_image_nearest_to(b"not-an-icon", 16).is_none());
    }
}
