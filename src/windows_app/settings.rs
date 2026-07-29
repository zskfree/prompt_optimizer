// Hallmark · native workbench · genre: modern-minimal · theme: Cobalt · user-preserved
// Knob deltas: header=expanded rhythm · fields=optical vertical centering · feedback=inline state
// Pre-emit critique: Philosophy 5 · Hierarchy 5 · Execution 5 · Specificity 5 · Restraint 5 · Variety 3

use prompt_optimizer::config::{ApiProfile, Config};
use std::ffi::c_void;
use windows::core::{w, Error as WindowsError, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawFocusRect, DrawTextW,
    EndPaint, FillRect, InvalidateRect, RoundRect, SelectObject, SetBkColor, SetBkMode,
    SetTextColor, DEFAULT_CHARSET, DRAW_TEXT_FORMAT, DT_LEFT, DT_SINGLELINE, DT_VCENTER,
    FF_DONTCARE, FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FONT_QUALITY, HFONT, PS_SOLID,
    TRANSPARENT,
};
use windows::Win32::UI::Controls::{
    TaskDialog, BST_CHECKED, BST_UNCHECKED, DRAWITEMSTRUCT, EM_SETCUEBANNER, EM_SETPASSWORDCHAR,
    ODS_DISABLED, ODS_FOCUS, ODS_HOTLIGHT, ODS_SELECTED, TDCBF_CLOSE_BUTTON, TD_ERROR_ICON,
};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetFocus, SetFocus, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, IsChild, LoadCursorW, PostMessageW, RegisterClassW,
    SendMessageW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow,
    BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_OWNERDRAW, CBN_SELCHANGE,
    CBS_DROPDOWNLIST, CB_ADDSTRING, CB_RESETCONTENT, CB_SETCURSEL, CREATESTRUCTW, CW_USEDEFAULT,
    EN_CHANGE, EN_KILLFOCUS, EN_SETFOCUS, ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE,
    ES_READONLY, ES_WANTRETURN, GWLP_USERDATA, HICON, HMENU, ICON_BIG, ICON_SMALL, IDC_ARROW,
    MINMAXINFO, SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_CTLCOLORBTN, WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC,
    WM_DESTROY, WM_DPICHANGED, WM_DRAWITEM, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN,
    WM_NCCREATE, WM_PAINT, WM_SETFONT, WM_SETICON, WM_SIZE, WNDCLASSW, WS_CAPTION, WS_CHILD,
    WS_CLIPCHILDREN, WS_EX_APPWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
    WS_TABSTOP, WS_THICKFRAME, WS_VISIBLE, WS_VSCROLL,
};

pub const WM_APPLY_CONFIG: u32 = WM_APP + 20;
pub const WM_SETTINGS_CLOSED: u32 = WM_APP + 21;
pub const WM_TEST_API: u32 = WM_APP + 22;

const CLASS_NAME: PCWSTR = w!("PromptOptimizer.SettingsWindow");
const TITLE: PCWSTR = w!("PromptOptimizer 设置");

const ID_NAV_SERVICE: u16 = 2101;
const ID_NAV_PROMPT: u16 = 2102;
const ID_NAV_BEHAVIOR: u16 = 2103;
const ID_API_KEY: u16 = 2201;
const ID_SHOW_KEY: u16 = 2202;
const ID_BASE_URL: u16 = 2203;
const ID_MODEL: u16 = 2204;
const ID_TEMPERATURE: u16 = 2205;
const ID_MAX_TOKENS: u16 = 2206;
const ID_SYSTEM_PROMPT: u16 = 2207;
const ID_HOTKEY: u16 = 2209;
const ID_PLAY_SOUND: u16 = 2210;
const ID_AUTO_START: u16 = 2211;
const ID_API_PROFILE: u16 = 2212;
const ID_PROFILE_NAME: u16 = 2213;
const ID_RESET: u16 = 2301;
const ID_SAVE: u16 = 2302;
const ID_PROFILE_NEW: u16 = 2303;
const ID_PROFILE_DELETE: u16 = 2304;
const ID_TEST_API: u16 = 2305;
const ID_ERROR_DETAILS: u16 = 2306;

const PAGE_SERVICE: usize = 0;
const PAGE_PROMPT: usize = 1;
const PAGE_BEHAVIOR: usize = 2;

// Cobalt native tokens. All GDI colours and spacing route through this compact set.
const COLOR_PAPER: COLORREF = rgb(246, 248, 252);
const COLOR_SURFACE: COLORREF = rgb(252, 253, 255);
const COLOR_SIDEBAR: COLORREF = rgb(237, 241, 248);
const COLOR_INK: COLORREF = rgb(27, 32, 44);
const COLOR_MUTED: COLORREF = rgb(86, 96, 116);
const COLOR_RULE: COLORREF = rgb(203, 211, 226);
const COLOR_ACCENT: COLORREF = rgb(49, 86, 207);
const COLOR_ACCENT_SOFT: COLORREF = rgb(222, 230, 252);
const COLOR_ERROR: COLORREF = rgb(171, 53, 68);
const COLOR_SUCCESS: COLORREF = rgb(35, 119, 88);

const fn rgb(red: u8, green: u8, blue: u8) -> COLORREF {
    COLORREF(red as u32 | ((green as u32) << 8) | ((blue as u32) << 16))
}

pub struct ApplyRequest {
    pub config: Config,
    pub error: Option<String>,
}

pub struct ApiTestRequest {
    pub config: Config,
    pub error: Option<String>,
}

struct CreateParams<'a> {
    owner: HWND,
    config: &'a Config,
}

#[derive(Default)]
struct Controls {
    nav_service: HWND,
    nav_prompt: HWND,
    nav_behavior: HWND,
    api_profile: HWND,
    profile_name: HWND,
    profile_new: HWND,
    profile_delete: HWND,
    test_api: HWND,
    api_key: HWND,
    show_key: HWND,
    base_url: HWND,
    model: HWND,
    temperature: HWND,
    max_tokens: HWND,
    system_prompt: HWND,
    hotkey: HWND,
    play_sound: HWND,
    auto_start: HWND,
    error_details: HWND,
    reset: HWND,
    save: HWND,
}

struct SettingsState {
    owner: HWND,
    current: Config,
    page: usize,
    dirty: bool,
    suppress_events: bool,
    testing_api: bool,
    draft_profiles: Vec<ApiProfile>,
    draft_active_profile: String,
    editing_profile_original: Option<String>,
    status: String,
    status_error: bool,
    controls: Controls,
    font_body: HFONT,
    font_label: HFONT,
    font_title: HFONT,
    brush_paper: windows::Win32::Graphics::Gdi::HBRUSH,
    brush_surface: windows::Win32::Graphics::Gdi::HBRUSH,
}

pub fn register(instance: HINSTANCE) -> Result<(), WindowsError> {
    let class = WNDCLASSW {
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
        hInstance: instance,
        lpszClassName: CLASS_NAME,
        lpfnWndProc: Some(window_proc),
        ..Default::default()
    };
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(WindowsError::from_thread());
    }
    Ok(())
}

pub unsafe fn show(
    owner: HWND,
    instance: HINSTANCE,
    icon: HICON,
    config: &Config,
) -> Result<HWND, WindowsError> {
    let dpi = GetDpiForWindow(owner).max(96);
    let style = WS_OVERLAPPED
        | WS_CAPTION
        | WS_SYSMENU
        | WS_THICKFRAME
        | WS_MINIMIZEBOX
        | WS_MAXIMIZEBOX
        | WS_CLIPCHILDREN;
    let ex_style = WS_EX_APPWINDOW;
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: scale(860, dpi),
        bottom: scale(730, dpi),
    };
    let _ = AdjustWindowRectExForDpi(&mut rect, style, false, ex_style, dpi);
    let params = CreateParams { owner, config };
    let hwnd = CreateWindowExW(
        ex_style,
        CLASS_NAME,
        TITLE,
        style,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        rect.right - rect.left,
        rect.bottom - rect.top,
        Some(owner),
        None,
        Some(instance),
        Some((&params as *const CreateParams<'_>).cast::<c_void>()),
    )?;
    let _ = SendMessageW(
        hwnd,
        WM_SETICON,
        Some(WPARAM(ICON_SMALL as usize)),
        Some(LPARAM(icon.0 as isize)),
    );
    let _ = SendMessageW(
        hwnd,
        WM_SETICON,
        Some(WPARAM(ICON_BIG as usize)),
        Some(LPARAM(icon.0 as isize)),
    );
    let preference = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        std::ptr::from_ref(&preference).cast::<c_void>(),
        size_of_val(&preference) as u32,
    );
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
    Ok(hwnd)
}

pub unsafe fn focus(hwnd: HWND) {
    let _ = ShowWindow(hwnd, SW_SHOW);
    let _ = SetForegroundWindow(hwnd);
}

pub unsafe fn refresh(hwnd: HWND, config: &Config) {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState;
    if pointer.is_null() {
        return;
    }
    let state = &mut *pointer;
    state.current = config.clone();
    state.draft_profiles = config.api_profiles.clone();
    state.draft_active_profile = config.active_profile.clone();
    state.editing_profile_original = Some(config.active_profile.clone());
    populate_controls(state);
    set_clean_state(state, "已加载当前配置");
    let _ = InvalidateRect(Some(hwnd), None, false);
}

pub unsafe fn complete_api_test(hwnd: HWND, result: Result<(), String>) {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState;
    if pointer.is_null() {
        return;
    }
    let state = &mut *pointer;
    state.testing_api = false;
    let _ = EnableWindow(state.controls.test_api, true);
    match result {
        Ok(()) => set_status(state, "API 连接正常", false),
        Err(error) => set_status(state, &error, true),
    }
    let _ = InvalidateRect(Some(state.controls.test_api), None, false);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam.0 as *const CREATESTRUCTW;
        if !create.is_null() {
            let params = (*create).lpCreateParams as *const CreateParams<'_>;
            if !params.is_null() {
                let state = Box::new(SettingsState {
                    owner: (*params).owner,
                    current: (*params).config.clone(),
                    page: PAGE_SERVICE,
                    dirty: false,
                    suppress_events: false,
                    testing_api: false,
                    draft_profiles: (*params).config.api_profiles.clone(),
                    draft_active_profile: (*params).config.active_profile.clone(),
                    editing_profile_original: Some((*params).config.active_profile.clone()),
                    status: "所有修改统一点击“保存并应用”".into(),
                    status_error: false,
                    controls: Controls::default(),
                    font_body: HFONT::default(),
                    font_label: HFONT::default(),
                    font_title: HFONT::default(),
                    brush_paper: CreateSolidBrush(COLOR_PAPER),
                    brush_surface: CreateSolidBrush(COLOR_SURFACE),
                });
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            }
        }
    }

    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsState;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    }
    let state = &mut *pointer;

    match message {
        WM_CREATE => {
            if let Err(error) = create_controls(hwnd, state) {
                state.status = format!("设置界面创建失败：{error}");
                state.status_error = true;
                return LRESULT(-1);
            }
            LRESULT(0)
        }
        WM_SIZE => {
            layout_controls(hwnd, state);
            let _ = InvalidateRect(Some(hwnd), None, false);
            LRESULT(0)
        }
        WM_DPICHANGED => {
            let suggested = lparam.0 as *const RECT;
            if !suggested.is_null() {
                let rect = *suggested;
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    rect.left,
                    rect.top,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            recreate_fonts(hwnd, state);
            layout_controls(hwnd, state);
            LRESULT(0)
        }
        WM_GETMINMAXINFO => {
            let info = lparam.0 as *mut MINMAXINFO;
            if !info.is_null() {
                let dpi = GetDpiForWindow(hwnd).max(96);
                (*info).ptMinTrackSize.x = scale(720, dpi);
                (*info).ptMinTrackSize.y = scale(760, dpi);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            handle_command(hwnd, state, wparam);
            LRESULT(0)
        }
        WM_DRAWITEM => {
            draw_button(state, lparam);
            LRESULT(1)
        }
        WM_CTLCOLOREDIT => {
            let dc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut c_void);
            let _ = SetTextColor(dc, COLOR_INK);
            let _ = SetBkColor(dc, COLOR_SURFACE);
            LRESULT(state.brush_surface.0 as isize)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let dc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as *mut c_void);
            let _ = SetTextColor(dc, COLOR_INK);
            let _ = SetBkMode(dc, TRANSPARENT);
            LRESULT(state.brush_paper.0 as isize)
        }
        WM_KEYDOWN if wparam.0 as u32 == VK_ESCAPE.0 as u32 => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_PAINT => {
            paint(hwnd, state);
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            let _ = PostMessageW(
                Some(state.owner),
                WM_SETTINGS_CLOSED,
                WPARAM(hwnd.0 as usize),
                LPARAM(0),
            );
            release_resources(state);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(pointer));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

unsafe fn create_controls(hwnd: HWND, state: &mut SettingsState) -> Result<(), WindowsError> {
    recreate_fonts(hwnd, state);
    let instance = HINSTANCE(windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?.0);

    state.controls.nav_service = create_button(hwnd, instance, ID_NAV_SERVICE, "模型与服务")?;
    state.controls.nav_prompt = create_button(hwnd, instance, ID_NAV_PROMPT, "优化规则")?;
    state.controls.nav_behavior = create_button(hwnd, instance, ID_NAV_BEHAVIOR, "应用行为")?;
    state.controls.api_profile = create_combo(hwnd, instance, ID_API_PROFILE)?;
    state.controls.profile_name = create_edit(hwnd, instance, ID_PROFILE_NAME, false, false)?;
    state.controls.profile_new = create_button(hwnd, instance, ID_PROFILE_NEW, "新建")?;
    state.controls.profile_delete = create_button(hwnd, instance, ID_PROFILE_DELETE, "删除")?;
    state.controls.test_api = create_button(hwnd, instance, ID_TEST_API, "测试连接")?;
    state.controls.api_key = create_edit(hwnd, instance, ID_API_KEY, false, false)?;
    state.controls.show_key = create_checkbox(hwnd, instance, ID_SHOW_KEY, "显示 API Key")?;
    state.controls.base_url = create_edit(hwnd, instance, ID_BASE_URL, false, false)?;
    state.controls.model = create_edit(hwnd, instance, ID_MODEL, false, false)?;
    state.controls.temperature = create_edit(hwnd, instance, ID_TEMPERATURE, false, false)?;
    state.controls.max_tokens = create_edit(hwnd, instance, ID_MAX_TOKENS, false, false)?;
    state.controls.system_prompt = create_edit(hwnd, instance, ID_SYSTEM_PROMPT, true, false)?;
    state.controls.hotkey = create_edit(hwnd, instance, ID_HOTKEY, false, false)?;
    state.controls.play_sound = create_checkbox(hwnd, instance, ID_PLAY_SOUND, "完成后播放提示音")?;
    state.controls.auto_start =
        create_checkbox(hwnd, instance, ID_AUTO_START, "登录 Windows 后自动启动")?;
    state.controls.error_details = create_button(hwnd, instance, ID_ERROR_DETAILS, "查看详情")?;
    state.controls.reset = create_button(hwnd, instance, ID_RESET, "重置更改")?;
    state.controls.save = create_button(hwnd, instance, ID_SAVE, "保存并应用")?;

    let cue_base = wide("https://api.siliconflow.cn/v1");
    let cue_model = wide("deepseek-ai/DeepSeek-V4-Flash");
    let cue_hotkey = wide("Ctrl+TripleA");
    let cue_profile_name = wide("例如：硅基流动");
    let _ = SendMessageW(
        state.controls.profile_name,
        EM_SETCUEBANNER,
        Some(WPARAM(1)),
        Some(LPARAM(cue_profile_name.as_ptr() as isize)),
    );
    let _ = SendMessageW(
        state.controls.base_url,
        EM_SETCUEBANNER,
        Some(WPARAM(1)),
        Some(LPARAM(cue_base.as_ptr() as isize)),
    );
    let _ = SendMessageW(
        state.controls.model,
        EM_SETCUEBANNER,
        Some(WPARAM(1)),
        Some(LPARAM(cue_model.as_ptr() as isize)),
    );
    let _ = SendMessageW(
        state.controls.hotkey,
        EM_SETCUEBANNER,
        Some(WPARAM(1)),
        Some(LPARAM(cue_hotkey.as_ptr() as isize)),
    );

    populate_controls(state);
    recreate_fonts(hwnd, state);
    set_clean_state(state, "所有修改统一点击“保存并应用”");
    update_page_visibility(state);
    layout_controls(hwnd, state);
    Ok(())
}

unsafe fn create_combo(parent: HWND, instance: HINSTANCE, id: u16) -> Result<HWND, WindowsError> {
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("COMBOBOX"),
        None,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
        0,
        0,
        0,
        0,
        Some(parent),
        Some(HMENU(id as usize as *mut c_void)),
        Some(instance),
        None,
    )
}

unsafe fn create_button(
    parent: HWND,
    instance: HINSTANCE,
    id: u16,
    text: &str,
) -> Result<HWND, WindowsError> {
    let label = wide(text);
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        PCWSTR(label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
        0,
        0,
        0,
        0,
        Some(parent),
        Some(HMENU(id as usize as *mut c_void)),
        Some(instance),
        None,
    )
}

unsafe fn create_checkbox(
    parent: HWND,
    instance: HINSTANCE,
    id: u16,
    text: &str,
) -> Result<HWND, WindowsError> {
    let label = wide(text);
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        PCWSTR(label.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        0,
        0,
        0,
        0,
        Some(parent),
        Some(HMENU(id as usize as *mut c_void)),
        Some(instance),
        None,
    )
}

unsafe fn create_edit(
    parent: HWND,
    instance: HINSTANCE,
    id: u16,
    multiline: bool,
    readonly: bool,
) -> Result<HWND, WindowsError> {
    let mut style = WS_CHILD | WS_VISIBLE | WS_TABSTOP;
    if multiline {
        style |= WINDOW_STYLE((ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN) as u32) | WS_VSCROLL;
    } else {
        style |= WINDOW_STYLE(ES_AUTOHSCROLL as u32);
    }
    if readonly {
        style |= WINDOW_STYLE(ES_READONLY as u32);
    }
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("EDIT"),
        None,
        style,
        0,
        0,
        0,
        0,
        Some(parent),
        Some(HMENU(id as usize as *mut c_void)),
        Some(instance),
        None,
    )
}

unsafe fn recreate_fonts(hwnd: HWND, state: &mut SettingsState) {
    for font in [state.font_body, state.font_label, state.font_title] {
        if !font.is_invalid() {
            let _ = DeleteObject(font.into());
        }
    }
    let dpi = GetDpiForWindow(hwnd).max(96);
    state.font_body = create_font(-scale(16, dpi), 400);
    state.font_label = create_font(-scale(13, dpi), 400);
    state.font_title = create_font(-scale(26, dpi), 650);
    for control in all_controls(&state.controls) {
        if !control.is_invalid() {
            let font = if control == state.controls.nav_service
                || control == state.controls.nav_prompt
                || control == state.controls.nav_behavior
                || control == state.controls.reset
                || control == state.controls.save
            {
                state.font_body
            } else {
                state.font_label
            };
            let _ = SendMessageW(
                control,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
}

unsafe fn create_font(height: i32, weight: i32) -> HFONT {
    CreateFontW(
        height,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        DEFAULT_CHARSET,
        FONT_OUTPUT_PRECISION::default(),
        FONT_CLIP_PRECISION::default(),
        FONT_QUALITY(5),
        FF_DONTCARE.0 as u32,
        w!("Segoe UI Variable Text"),
    )
}

unsafe fn populate_controls(state: &mut SettingsState) {
    state.suppress_events = true;
    populate_profile_list(state);
    let active = state
        .draft_profiles
        .iter()
        .find(|profile| {
            profile
                .name
                .trim()
                .eq_ignore_ascii_case(state.draft_active_profile.trim())
        })
        .cloned()
        .unwrap_or_default();
    populate_api_fields(state, &active);
    set_text(state.controls.system_prompt, &state.current.system_prompt);
    set_text(state.controls.hotkey, &state.current.hotkey);
    set_checked(state.controls.show_key, false);
    set_checked(state.controls.play_sound, state.current.play_sound);
    set_checked(state.controls.auto_start, state.current.auto_start);
    let _ = SendMessageW(
        state.controls.api_key,
        EM_SETPASSWORDCHAR,
        Some(WPARAM('●' as usize)),
        Some(LPARAM(0)),
    );
    let _ = InvalidateRect(Some(state.controls.api_key), None, true);
    state.suppress_events = false;
}

unsafe fn populate_profile_list(state: &mut SettingsState) {
    let _ = SendMessageW(
        state.controls.api_profile,
        CB_RESETCONTENT,
        Some(WPARAM(0)),
        Some(LPARAM(0)),
    );
    let mut selected = None;
    for (index, profile) in state.draft_profiles.iter().enumerate() {
        let name = wide(profile.name.trim());
        let _ = SendMessageW(
            state.controls.api_profile,
            CB_ADDSTRING,
            Some(WPARAM(0)),
            Some(LPARAM(name.as_ptr() as isize)),
        );
        if state
            .draft_active_profile
            .eq_ignore_ascii_case(profile.name.trim())
        {
            selected = Some(index);
        }
    }
    if let Some(index) = selected {
        let _ = SendMessageW(
            state.controls.api_profile,
            CB_SETCURSEL,
            Some(WPARAM(index)),
            Some(LPARAM(0)),
        );
    } else {
        set_text(state.controls.api_profile, "");
    }
}

unsafe fn populate_api_fields(state: &mut SettingsState, profile: &ApiProfile) {
    let was_suppressed = state.suppress_events;
    state.suppress_events = true;
    set_text(state.controls.profile_name, profile.name.trim());
    set_text(state.controls.api_key, &profile.api_key);
    set_text(state.controls.base_url, &profile.base_url);
    set_text(state.controls.model, &profile.model);
    set_text(
        state.controls.temperature,
        &format_temperature(profile.temperature),
    );
    set_text(state.controls.max_tokens, &profile.max_tokens.to_string());
    state.suppress_events = was_suppressed;
}

unsafe fn handle_command(hwnd: HWND, state: &mut SettingsState, wparam: WPARAM) {
    let id = (wparam.0 & 0xFFFF) as u16;
    let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;
    if notification == BN_CLICKED {
        match id {
            ID_NAV_SERVICE => switch_page(hwnd, state, PAGE_SERVICE),
            ID_NAV_PROMPT => switch_page(hwnd, state, PAGE_PROMPT),
            ID_NAV_BEHAVIOR => switch_page(hwnd, state, PAGE_BEHAVIOR),
            ID_SHOW_KEY => toggle_key_visibility(state),
            ID_RESET => {
                state.draft_profiles = state.current.api_profiles.clone();
                state.draft_active_profile = state.current.active_profile.clone();
                state.editing_profile_original = Some(state.current.active_profile.clone());
                populate_controls(state);
                set_clean_state(state, "已恢复为当前运行配置");
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            ID_SAVE => save(hwnd, state),
            ID_PROFILE_NEW => new_profile(hwnd, state),
            ID_PROFILE_DELETE => delete_profile(hwnd, state),
            ID_TEST_API => test_api(hwnd, state),
            ID_ERROR_DETAILS => show_error_details(hwnd, state),
            ID_PLAY_SOUND | ID_AUTO_START => mark_dirty(hwnd, state),
            _ => {}
        }
    } else if id == ID_API_PROFILE && notification == CBN_SELCHANGE {
        switch_api_profile(hwnd, state);
    } else if should_mark_dirty(state.suppress_events, notification, id) {
        mark_dirty(hwnd, state);
    } else if notification == EN_SETFOCUS || notification == EN_KILLFOCUS {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

unsafe fn save(hwnd: HWND, state: &mut SettingsState) {
    let config = match collect_config(state) {
        Ok(config) => config,
        Err(error) => {
            show_error(hwnd, state, &error);
            return;
        }
    };
    if let Err(error) = config.validate() {
        show_error(hwnd, state, &error.to_string());
        return;
    }

    let mut request = ApplyRequest {
        config,
        error: None,
    };
    let result = SendMessageW(
        state.owner,
        WM_APPLY_CONFIG,
        Some(WPARAM(hwnd.0 as usize)),
        Some(LPARAM((&mut request as *mut ApplyRequest) as isize)),
    );
    if result.0 == 1 {
        state.current = request.config;
        state.draft_profiles = state.current.api_profiles.clone();
        state.draft_active_profile = state.current.active_profile.clone();
        state.editing_profile_original = Some(state.current.active_profile.clone());
        populate_profile_list_safely(state);
        set_clean_state(state, "已保存并应用");
    } else {
        show_error(
            hwnd,
            state,
            request.error.as_deref().unwrap_or("配置未能应用，请重试"),
        );
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn collect_config(state: &SettingsState) -> Result<Config, String> {
    let profile = collect_api_profile(state)?;
    let mut profiles = state.draft_profiles.clone();
    stage_profile(
        &mut profiles,
        state.editing_profile_original.as_deref(),
        profile.clone(),
    )?;
    Ok(Config {
        active_profile: profile.name,
        api_profiles: profiles,
        hotkey: get_text(state.controls.hotkey).trim().to_string(),
        system_prompt: get_text(state.controls.system_prompt).trim().to_string(),
        result_mode: "clipboard".into(),
        play_sound: is_checked(state.controls.play_sound),
        auto_start: is_checked(state.controls.auto_start),
    })
}

unsafe fn collect_api_profile(state: &SettingsState) -> Result<ApiProfile, String> {
    let temperature = match get_text(state.controls.temperature).trim().parse::<f64>() {
        Ok(value) => value,
        Err(_) => return Err("温度必须是 0.0–2.0 之间的数字".into()),
    };
    let max_tokens = match get_text(state.controls.max_tokens).trim().parse::<u32>() {
        Ok(value) => value,
        Err(_) => return Err("最大 Token 数必须是大于 0 的整数".into()),
    };
    let profile = ApiProfile {
        name: get_text(state.controls.profile_name).trim().to_string(),
        api_key: get_text(state.controls.api_key),
        base_url: get_text(state.controls.base_url).trim().to_string(),
        model: get_text(state.controls.model).trim().to_string(),
        temperature,
        max_tokens,
    };
    profile.validate().map_err(|error| error.to_string())?;
    Ok(profile)
}

unsafe fn new_profile(hwnd: HWND, state: &mut SettingsState) {
    if let Err(error) = stage_visible_profile(state) {
        show_error(hwnd, state, &error);
        return;
    }
    state.editing_profile_original = None;
    let mut profile = ApiProfile::default();
    profile.name.clear();
    populate_api_fields(state, &profile);
    let _ = SendMessageW(
        state.controls.api_profile,
        CB_SETCURSEL,
        Some(WPARAM(usize::MAX)),
        Some(LPARAM(0)),
    );
    let _ = SetFocus(Some(state.controls.profile_name));
    mark_dirty_with_status(hwnd, state, "正在新建 API 配置，填写后保存并应用");
}

unsafe fn delete_profile(hwnd: HWND, state: &mut SettingsState) {
    let Some(name) = state.editing_profile_original.clone() else {
        let active = state
            .draft_profiles
            .iter()
            .find(|profile| {
                profile
                    .name
                    .trim()
                    .eq_ignore_ascii_case(state.draft_active_profile.trim())
            })
            .cloned()
            .unwrap_or_default();
        state.editing_profile_original = Some(active.name.clone());
        populate_api_fields(state, &active);
        populate_profile_list_safely(state);
        mark_dirty_with_status(hwnd, state, "已取消新建配置");
        return;
    };
    if state.draft_profiles.len() == 1 {
        show_error(hwnd, state, "至少需要保留一个 API 配置");
        return;
    }
    if !remove_profile(&mut state.draft_profiles, &name) {
        show_error(hwnd, state, "未找到要删除的 API 配置");
        return;
    }
    let next = state.draft_profiles[0].clone();
    state.draft_active_profile = next.name.clone();
    state.editing_profile_original = Some(next.name.clone());
    populate_api_fields(state, &next);
    populate_profile_list_safely(state);
    mark_dirty_with_status(hwnd, state, "API 配置已移除；保存并应用，或重置撤销");
}

unsafe fn switch_api_profile(hwnd: HWND, state: &mut SettingsState) {
    if state.suppress_events {
        return;
    }
    let target_name = get_text(state.controls.api_profile);
    let previous_name = state.editing_profile_original.clone();
    if let Err(error) = stage_visible_profile(state) {
        if let Some(previous) = previous_name.as_deref() {
            select_profile(state, previous);
        }
        show_error(hwnd, state, &format!("当前配置尚未填写完整：{error}"));
        return;
    }
    let Some(profile) = state
        .draft_profiles
        .iter()
        .find(|profile| profile.name.trim().eq_ignore_ascii_case(target_name.trim()))
        .cloned()
    else {
        return;
    };
    state.draft_active_profile = profile.name.clone();
    state.editing_profile_original = Some(profile.name.clone());
    populate_api_fields(state, &profile);
    populate_profile_list_safely(state);
    mark_dirty_with_status(
        hwnd,
        state,
        &format!("当前配置：{}（保存并应用后生效）", profile.name),
    );
}

unsafe fn test_api(hwnd: HWND, state: &mut SettingsState) {
    if state.testing_api {
        return;
    }
    let config = match collect_config(state) {
        Ok(config) => config,
        Err(error) => {
            show_error(hwnd, state, &error);
            return;
        }
    };
    if config
        .active_api()
        .is_none_or(|profile| profile.api_key.trim().is_empty())
    {
        show_error(hwnd, state, "请先填写 API Key");
        return;
    }
    if let Err(error) = config.validate() {
        show_error(hwnd, state, &error.to_string());
        return;
    }
    let mut request = ApiTestRequest {
        config,
        error: None,
    };
    let result = SendMessageW(
        state.owner,
        WM_TEST_API,
        Some(WPARAM(hwnd.0 as usize)),
        Some(LPARAM((&mut request as *mut ApiTestRequest) as isize)),
    );
    if result.0 == 1 {
        state.testing_api = true;
        set_status(state, "正在测试 API 连接…", false);
        let _ = EnableWindow(state.controls.test_api, false);
    } else {
        show_error(
            hwnd,
            state,
            request.error.as_deref().unwrap_or("无法开始 API 连接测试"),
        );
    }
    let _ = InvalidateRect(Some(state.controls.test_api), None, false);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn populate_profile_list_safely(state: &mut SettingsState) {
    state.suppress_events = true;
    populate_profile_list(state);
    state.suppress_events = false;
}

unsafe fn show_error(hwnd: HWND, state: &mut SettingsState, message: &str) {
    set_status(state, message, true);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn show_error_details(hwnd: HWND, state: &SettingsState) {
    if !state.status_error {
        return;
    }
    let content = wide(&state.status);
    let _ = TaskDialog(
        Some(hwnd),
        None,
        w!("PromptOptimizer 设置"),
        w!("请求未成功"),
        PCWSTR(content.as_ptr()),
        TDCBF_CLOSE_BUTTON,
        TD_ERROR_ICON,
        None,
    );
}

unsafe fn set_status(state: &mut SettingsState, status: &str, is_error: bool) {
    state.status = status.into();
    state.status_error = is_error;
    if !state.controls.error_details.is_invalid() {
        let _ = ShowWindow(
            state.controls.error_details,
            if is_error { SW_SHOW } else { SW_HIDE },
        );
    }
}

unsafe fn mark_dirty(hwnd: HWND, state: &mut SettingsState) {
    mark_dirty_with_status(hwnd, state, "有尚未保存的更改");
}

unsafe fn mark_dirty_with_status(hwnd: HWND, state: &mut SettingsState, status: &str) {
    if state.suppress_events {
        return;
    }
    state.dirty = true;
    set_status(state, status, false);
    let _ = EnableWindow(state.controls.save, true);
    let _ = EnableWindow(state.controls.reset, true);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn set_clean_state(state: &mut SettingsState, status: &str) {
    state.dirty = false;
    set_status(state, status, false);
    let _ = EnableWindow(state.controls.save, false);
    let _ = EnableWindow(state.controls.reset, false);
}

unsafe fn switch_page(hwnd: HWND, state: &mut SettingsState, page: usize) {
    if state.page == page {
        return;
    }
    state.page = page;
    update_page_visibility(state);
    layout_controls(hwnd, state);
    let focus = match page {
        PAGE_SERVICE => state.controls.api_key,
        PAGE_PROMPT => state.controls.system_prompt,
        _ => state.controls.hotkey,
    };
    let _ = SetFocus(Some(focus));
    for nav in [
        state.controls.nav_service,
        state.controls.nav_prompt,
        state.controls.nav_behavior,
    ] {
        let _ = InvalidateRect(Some(nav), None, false);
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}

unsafe fn toggle_key_visibility(state: &mut SettingsState) {
    let password = if is_checked(state.controls.show_key) {
        0
    } else {
        '●' as usize
    };
    let _ = SendMessageW(
        state.controls.api_key,
        EM_SETPASSWORDCHAR,
        Some(WPARAM(password)),
        Some(LPARAM(0)),
    );
    let _ = InvalidateRect(Some(state.controls.api_key), None, true);
}

unsafe fn update_page_visibility(state: &SettingsState) {
    let service = state.page == PAGE_SERVICE;
    let prompt = state.page == PAGE_PROMPT;
    let behavior = state.page == PAGE_BEHAVIOR;
    for control in [
        state.controls.api_profile,
        state.controls.profile_name,
        state.controls.profile_new,
        state.controls.profile_delete,
        state.controls.test_api,
        state.controls.api_key,
        state.controls.show_key,
        state.controls.base_url,
        state.controls.model,
        state.controls.temperature,
        state.controls.max_tokens,
    ] {
        let _ = ShowWindow(control, if service { SW_SHOW } else { SW_HIDE });
    }
    let _ = ShowWindow(
        state.controls.system_prompt,
        if prompt { SW_SHOW } else { SW_HIDE },
    );
    for control in [
        state.controls.hotkey,
        state.controls.play_sound,
        state.controls.auto_start,
    ] {
        let _ = ShowWindow(control, if behavior { SW_SHOW } else { SW_HIDE });
    }
}

unsafe fn layout_controls(hwnd: HWND, state: &SettingsState) {
    let dpi = GetDpiForWindow(hwnd).max(96);
    let mut client = RECT::default();
    let _ = GetClientRect(hwnd, &mut client);
    let sidebar = scale(196, dpi);
    let content_left = sidebar + scale(38, dpi);
    let content_right = client.right - scale(38, dpi);
    let content_width = (content_right - content_left).max(scale(360, dpi));
    let header = settings_header_height(dpi);
    let footer = scale(72, dpi);
    let field_h = scale(44, dpi);

    move_control(
        state.controls.nav_service,
        scale(18, dpi),
        scale(112, dpi),
        scale(160, dpi),
        scale(44, dpi),
    );
    move_control(
        state.controls.nav_prompt,
        scale(18, dpi),
        scale(164, dpi),
        scale(160, dpi),
        scale(44, dpi),
    );
    move_control(
        state.controls.nav_behavior,
        scale(18, dpi),
        scale(216, dpi),
        scale(160, dpi),
        scale(44, dpi),
    );

    match state.page {
        PAGE_SERVICE => {
            let profile_y = header + scale(10, dpi);
            let profile_actions = scale(132, dpi);
            let profile_bounds = [
                content_left,
                profile_y,
                content_width - profile_actions,
                field_h,
            ];
            let [profile_x, profile_edit_y, profile_width, _] = field_content(profile_bounds, dpi);
            move_control(
                state.controls.api_profile,
                profile_x,
                profile_edit_y,
                profile_width,
                scale(220, dpi),
            );
            move_control(
                state.controls.profile_new,
                content_right - scale(120, dpi),
                profile_y,
                scale(52, dpi),
                field_h,
            );
            move_control(
                state.controls.profile_delete,
                content_right - scale(60, dpi),
                profile_y,
                scale(60, dpi),
                field_h,
            );
            move_field_control(
                state.controls.profile_name,
                [
                    content_left,
                    header + scale(82, dpi),
                    content_width,
                    field_h,
                ],
                dpi,
                false,
            );
            move_field_control(
                state.controls.api_key,
                [
                    content_left,
                    header + scale(154, dpi),
                    content_width,
                    field_h,
                ],
                dpi,
                false,
            );
            move_control(
                state.controls.show_key,
                content_left,
                header + scale(202, dpi),
                scale(150, dpi),
                scale(32, dpi),
            );
            move_control(
                state.controls.test_api,
                content_right - scale(104, dpi),
                header + scale(198, dpi),
                scale(104, dpi),
                field_h,
            );
            move_field_control(
                state.controls.base_url,
                [
                    content_left,
                    header + scale(266, dpi),
                    content_width,
                    field_h,
                ],
                dpi,
                false,
            );
            move_field_control(
                state.controls.model,
                [
                    content_left,
                    header + scale(338, dpi),
                    content_width,
                    field_h,
                ],
                dpi,
                false,
            );
            let half = (content_width - scale(16, dpi)) / 2;
            move_field_control(
                state.controls.temperature,
                [content_left, header + scale(410, dpi), half, field_h],
                dpi,
                false,
            );
            move_field_control(
                state.controls.max_tokens,
                [
                    content_left + half + scale(16, dpi),
                    header + scale(410, dpi),
                    half,
                    field_h,
                ],
                dpi,
                false,
            );
        }
        PAGE_PROMPT => {
            let prompt_h = (client.bottom - footer - header - scale(124, dpi)).max(scale(180, dpi));
            move_field_control(
                state.controls.system_prompt,
                [
                    content_left,
                    header + scale(22, dpi),
                    content_width,
                    prompt_h,
                ],
                dpi,
                true,
            );
        }
        _ => {
            move_field_control(
                state.controls.hotkey,
                [
                    content_left,
                    header + scale(22, dpi),
                    content_width,
                    field_h,
                ],
                dpi,
                false,
            );
            move_control(
                state.controls.play_sound,
                content_left,
                header + scale(112, dpi),
                content_width,
                scale(44, dpi),
            );
            move_control(
                state.controls.auto_start,
                content_left,
                header + scale(164, dpi),
                content_width,
                scale(44, dpi),
            );
        }
    }

    let buttons_y = client.bottom - scale(56, dpi);
    move_control(
        state.controls.error_details,
        client.right - scale(354, dpi),
        buttons_y,
        scale(84, dpi),
        scale(40, dpi),
    );
    move_control(
        state.controls.reset,
        client.right - scale(250, dpi),
        buttons_y,
        scale(108, dpi),
        scale(40, dpi),
    );
    move_control(
        state.controls.save,
        client.right - scale(132, dpi),
        buttons_y,
        scale(112, dpi),
        scale(40, dpi),
    );
}

unsafe fn paint(hwnd: HWND, state: &SettingsState) {
    let dpi = GetDpiForWindow(hwnd).max(96);
    let mut paint = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
    let dc = BeginPaint(hwnd, &mut paint);
    let mut client = RECT::default();
    let _ = GetClientRect(hwnd, &mut client);
    FillRect(dc, &client, state.brush_paper);

    let sidebar_width = scale(196, dpi);
    let sidebar_brush = CreateSolidBrush(COLOR_SIDEBAR);
    let sidebar_rect = RECT {
        left: 0,
        top: 0,
        right: sidebar_width,
        bottom: client.bottom,
    };
    FillRect(dc, &sidebar_rect, sidebar_brush);
    let _ = DeleteObject(sidebar_brush.into());

    let previous = SelectObject(dc, state.font_title.into());
    let _ = SetBkMode(dc, TRANSPARENT);
    let _ = SetTextColor(dc, COLOR_INK);
    let [title_top, title_bottom, subtitle_top, subtitle_bottom] = header_vertical_metrics(dpi);
    draw_text(
        dc,
        "设置",
        RECT {
            left: sidebar_width + scale(38, dpi),
            top: title_top,
            right: client.right - scale(32, dpi),
            bottom: title_bottom,
        },
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );
    let _ = SelectObject(dc, state.font_body.into());
    let subtitle = match state.page {
        PAGE_SERVICE => "模型与服务",
        PAGE_PROMPT => "优化规则",
        _ => "应用行为",
    };
    draw_text(
        dc,
        subtitle,
        RECT {
            left: sidebar_width + scale(38, dpi),
            top: subtitle_top,
            right: client.right - scale(32, dpi),
            bottom: subtitle_bottom,
        },
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );

    let _ = SelectObject(dc, state.font_label.into());
    let _ = SetTextColor(dc, COLOR_MUTED);
    draw_text(
        dc,
        "PromptOptimizer",
        RECT {
            left: scale(20, dpi),
            top: scale(24, dpi),
            right: sidebar_width - scale(16, dpi),
            bottom: scale(52, dpi),
        },
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );
    draw_text(
        dc,
        "本地配置",
        RECT {
            left: scale(20, dpi),
            top: scale(54, dpi),
            right: sidebar_width - scale(16, dpi),
            bottom: scale(76, dpi),
        },
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );

    paint_fields(dc, state, client, dpi);

    let footer_top = client.bottom - scale(72, dpi);
    let rule_brush = CreateSolidBrush(COLOR_RULE);
    FillRect(
        dc,
        &RECT {
            left: sidebar_width,
            top: footer_top,
            right: client.right,
            bottom: footer_top + 1,
        },
        rule_brush,
    );
    let _ = DeleteObject(rule_brush.into());
    let _ = SetTextColor(
        dc,
        if state.status_error {
            COLOR_ERROR
        } else if !state.dirty && state.status == "已保存并应用" {
            COLOR_SUCCESS
        } else {
            COLOR_MUTED
        },
    );
    let status = if state.status_error {
        concise_status(&state.status, 18)
    } else {
        state.status.clone()
    };
    draw_text(
        dc,
        &status,
        RECT {
            left: sidebar_width + scale(38, dpi),
            top: footer_top + scale(17, dpi),
            right: client.right - scale(if state.status_error { 374 } else { 272 }, dpi),
            bottom: client.bottom - scale(10, dpi),
        },
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );

    let _ = SelectObject(dc, previous);
    let _ = EndPaint(hwnd, &paint);
}

unsafe fn paint_fields(
    dc: windows::Win32::Graphics::Gdi::HDC,
    state: &SettingsState,
    client: RECT,
    dpi: u32,
) {
    let sidebar = scale(196, dpi);
    let left = sidebar + scale(38, dpi);
    let right = client.right - scale(38, dpi);
    let width = (right - left).max(scale(360, dpi));
    let header = settings_header_height(dpi);
    let field_h = scale(44, dpi);
    let _ = SetTextColor(dc, COLOR_INK);
    match state.page {
        PAGE_SERVICE => {
            let profile_width = width - scale(132, dpi);
            field(
                dc,
                state.controls.api_profile,
                "当前配置",
                [left, header + scale(10, dpi), profile_width, field_h],
                dpi,
            );
            field(
                dc,
                state.controls.profile_name,
                "配置名称",
                [left, header + scale(82, dpi), width, field_h],
                dpi,
            );
            field(
                dc,
                state.controls.api_key,
                "API Key",
                [left, header + scale(154, dpi), width, field_h],
                dpi,
            );
            field(
                dc,
                state.controls.base_url,
                "服务地址",
                [left, header + scale(266, dpi), width, field_h],
                dpi,
            );
            field(
                dc,
                state.controls.model,
                "模型",
                [left, header + scale(338, dpi), width, field_h],
                dpi,
            );
            let half = (width - scale(16, dpi)) / 2;
            field(
                dc,
                state.controls.temperature,
                "温度",
                [left, header + scale(410, dpi), half, field_h],
                dpi,
            );
            field(
                dc,
                state.controls.max_tokens,
                "最大 Token 数",
                [
                    left + half + scale(16, dpi),
                    header + scale(410, dpi),
                    half,
                    field_h,
                ],
                dpi,
            );
        }
        PAGE_PROMPT => {
            let footer = scale(72, dpi);
            let prompt_h = (client.bottom - footer - header - scale(124, dpi)).max(scale(180, dpi));
            field(
                dc,
                state.controls.system_prompt,
                "系统提示词",
                [left, header + scale(22, dpi), width, prompt_h],
                dpi,
            );
            let _ = SelectObject(dc, state.font_body.into());
            let _ = SetTextColor(dc, COLOR_INK);
            draw_text(
                dc,
                "优化结果将复制到剪贴板",
                RECT {
                    left,
                    top: header + prompt_h + scale(52, dpi),
                    right,
                    bottom: header + prompt_h + scale(78, dpi),
                },
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
            let _ = SelectObject(dc, state.font_label.into());
            let _ = SetTextColor(dc, COLOR_MUTED);
            draw_text(
                dc,
                "不会自动替换原文字，当前无需额外设置",
                RECT {
                    left,
                    top: header + prompt_h + scale(80, dpi),
                    right,
                    bottom: header + prompt_h + scale(102, dpi),
                },
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
        _ => {
            field(
                dc,
                state.controls.hotkey,
                "全局热键",
                [left, header + scale(22, dpi), width, field_h],
                dpi,
            );
            let _ = SetTextColor(dc, COLOR_MUTED);
            draw_text(
                dc,
                "示例：Ctrl+TripleA · Ctrl+DoubleA · Ctrl+F8 · Alt+Shift+P",
                RECT {
                    left,
                    top: header + scale(80, dpi),
                    right,
                    bottom: header + scale(102, dpi),
                },
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }
}

unsafe fn field(
    dc: windows::Win32::Graphics::Gdi::HDC,
    control: HWND,
    label: &str,
    bounds: [i32; 4],
    dpi: u32,
) {
    let [x, y, width, height] = bounds;
    let _ = SetTextColor(dc, COLOR_MUTED);
    draw_text(
        dc,
        label,
        RECT {
            left: x,
            top: y - scale(24, dpi),
            right: x + width,
            bottom: y - scale(2, dpi),
        },
        DT_LEFT | DT_SINGLELINE | DT_VCENTER,
    );
    let focus = GetFocus();
    let focused = focus == control || IsChild(control, focus).as_bool();
    let border = if focused { COLOR_ACCENT } else { COLOR_RULE };
    let brush = CreateSolidBrush(COLOR_SURFACE);
    let pen = CreatePen(PS_SOLID, 1, border);
    let old_brush = SelectObject(dc, brush.into());
    let old_pen = SelectObject(dc, pen.into());
    let _ = RoundRect(
        dc,
        x,
        y,
        x + width,
        y + height,
        scale(8, dpi),
        scale(8, dpi),
    );
    let _ = SelectObject(dc, old_pen);
    let _ = SelectObject(dc, old_brush);
    let _ = DeleteObject(pen.into());
    let _ = DeleteObject(brush.into());
}

unsafe fn draw_button(state: &SettingsState, lparam: LPARAM) {
    let item = lparam.0 as *const DRAWITEMSTRUCT;
    if item.is_null() {
        return;
    }
    let item = &*item;
    let id = item.CtlID as u16;
    let selected_nav = matches!(
        (id, state.page),
        (ID_NAV_SERVICE, PAGE_SERVICE)
            | (ID_NAV_PROMPT, PAGE_PROMPT)
            | (ID_NAV_BEHAVIOR, PAGE_BEHAVIOR)
    );
    let disabled = (item.itemState.0 & ODS_DISABLED.0) != 0;
    let pressed = (item.itemState.0 & ODS_SELECTED.0) != 0;
    let hot = (item.itemState.0 & ODS_HOTLIGHT.0) != 0;
    let primary = id == ID_SAVE;
    let content_button = matches!(
        id,
        ID_RESET | ID_PROFILE_NEW | ID_PROFILE_DELETE | ID_TEST_API | ID_ERROR_DETAILS
    );
    let background = if disabled {
        COLOR_RULE
    } else if primary {
        if pressed {
            rgb(38, 68, 172)
        } else {
            COLOR_ACCENT
        }
    } else if selected_nav {
        COLOR_ACCENT_SOFT
    } else if hot || pressed || content_button {
        COLOR_SURFACE
    } else {
        COLOR_SIDEBAR
    };
    let text_color = if disabled {
        COLOR_MUTED
    } else if primary {
        COLOR_SURFACE
    } else if selected_nav {
        COLOR_ACCENT
    } else {
        COLOR_INK
    };
    let brush = CreateSolidBrush(background);
    let pen = CreatePen(
        PS_SOLID,
        1,
        if primary || selected_nav {
            background
        } else {
            COLOR_RULE
        },
    );
    let old_brush = SelectObject(item.hDC, brush.into());
    let old_pen = SelectObject(item.hDC, pen.into());
    let _ = RoundRect(
        item.hDC,
        item.rcItem.left,
        item.rcItem.top,
        item.rcItem.right,
        item.rcItem.bottom,
        8,
        8,
    );
    let _ = SelectObject(item.hDC, old_pen);
    let _ = SelectObject(item.hDC, old_brush);
    let _ = DeleteObject(pen.into());
    let _ = DeleteObject(brush.into());

    let old_font = SelectObject(item.hDC, state.font_body.into());
    let _ = SetBkMode(item.hDC, TRANSPARENT);
    let _ = SetTextColor(item.hDC, text_color);
    let mut text = vec![0_u16; (GetWindowTextLengthW(item.hwndItem) + 1).max(1) as usize];
    let len = GetWindowTextW(item.hwndItem, &mut text).max(0) as usize;
    let mut rect = item.rcItem;
    let align = if matches!(id, ID_NAV_SERVICE | ID_NAV_PROMPT | ID_NAV_BEHAVIOR) {
        DT_LEFT | DT_SINGLELINE | DT_VCENTER
    } else {
        windows::Win32::Graphics::Gdi::DT_CENTER | DT_SINGLELINE | DT_VCENTER
    };
    if matches!(id, ID_NAV_SERVICE | ID_NAV_PROMPT | ID_NAV_BEHAVIOR) {
        rect.left += 14;
    }
    DrawTextW(item.hDC, &mut text[..len], &mut rect, align);
    if (item.itemState.0 & ODS_FOCUS.0) != 0 {
        let mut focus = item.rcItem;
        focus.left += 3;
        focus.top += 3;
        focus.right -= 3;
        focus.bottom -= 3;
        let _ = DrawFocusRect(item.hDC, &focus);
    }
    let _ = SelectObject(item.hDC, old_font);
}

unsafe fn draw_text(
    dc: windows::Win32::Graphics::Gdi::HDC,
    text: &str,
    mut rect: RECT,
    format: DRAW_TEXT_FORMAT,
) {
    let mut values = text.encode_utf16().collect::<Vec<_>>();
    DrawTextW(dc, &mut values, &mut rect, format);
}

unsafe fn release_resources(state: &mut SettingsState) {
    for font in [state.font_body, state.font_label, state.font_title] {
        if !font.is_invalid() {
            let _ = DeleteObject(font.into());
        }
    }
    for brush in [state.brush_paper, state.brush_surface] {
        if !brush.is_invalid() {
            let _ = DeleteObject(brush.into());
        }
    }
}

unsafe fn set_text(hwnd: HWND, value: &str) {
    let text = wide(value);
    let _ = SetWindowTextW(hwnd, PCWSTR(text.as_ptr()));
}

unsafe fn get_text(hwnd: HWND) -> String {
    let length = GetWindowTextLengthW(hwnd).max(0) as usize;
    let mut buffer = vec![0_u16; length + 1];
    let copied = GetWindowTextW(hwnd, &mut buffer).max(0) as usize;
    String::from_utf16_lossy(&buffer[..copied])
}

unsafe fn set_checked(hwnd: HWND, checked: bool) {
    let _ = SendMessageW(
        hwnd,
        BM_SETCHECK,
        Some(WPARAM(if checked {
            BST_CHECKED.0 as usize
        } else {
            BST_UNCHECKED.0 as usize
        })),
        Some(LPARAM(0)),
    );
}

unsafe fn is_checked(hwnd: HWND) -> bool {
    SendMessageW(hwnd, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0 as u32 == BST_CHECKED.0
}

unsafe fn move_control(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    let _ = SetWindowPos(
        hwnd,
        None,
        x,
        y,
        width.max(1),
        height.max(1),
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe fn move_field_control(hwnd: HWND, bounds: [i32; 4], dpi: u32, multiline: bool) {
    let [x, y, width, height] = if multiline {
        multiline_field_content(bounds, dpi)
    } else {
        field_content(bounds, dpi)
    };
    move_control(hwnd, x, y, width, height);
}

fn field_content([x, y, width, height]: [i32; 4], dpi: u32) -> [i32; 4] {
    inset_bounds([x, y, width, height], scale(12, dpi), scale(10, dpi))
}

fn multiline_field_content([x, y, width, height]: [i32; 4], dpi: u32) -> [i32; 4] {
    inset_bounds([x, y, width, height], scale(12, dpi), scale(10, dpi))
}

fn inset_bounds([x, y, width, height]: [i32; 4], horizontal: i32, vertical: i32) -> [i32; 4] {
    [
        x + horizontal,
        y + vertical,
        (width - horizontal * 2).max(1),
        (height - vertical * 2).max(1),
    ]
}

fn concise_status(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let result: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{result}…")
    } else {
        result
    }
}

fn settings_header_height(dpi: u32) -> i32 {
    scale(124, dpi)
}

fn header_vertical_metrics(dpi: u32) -> [i32; 4] {
    [
        scale(24, dpi),
        scale(60, dpi),
        scale(70, dpi),
        scale(96, dpi),
    ]
}

fn scale(value: i32, dpi: u32) -> i32 {
    ((value as i64 * dpi as i64 + 48) / 96) as i32
}

fn format_temperature(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn is_edit_id(id: u16) -> bool {
    matches!(
        id,
        ID_API_KEY
            | ID_PROFILE_NAME
            | ID_BASE_URL
            | ID_MODEL
            | ID_TEMPERATURE
            | ID_MAX_TOKENS
            | ID_SYSTEM_PROMPT
            | ID_HOTKEY
    )
}

fn should_mark_dirty(suppress_events: bool, notification: u32, id: u16) -> bool {
    !suppress_events && notification == EN_CHANGE && is_edit_id(id)
}

unsafe fn stage_visible_profile(state: &mut SettingsState) -> Result<(), String> {
    let profile = collect_api_profile(state)?;
    stage_profile(
        &mut state.draft_profiles,
        state.editing_profile_original.as_deref(),
        profile.clone(),
    )?;
    state.editing_profile_original = Some(profile.name);
    Ok(())
}

fn stage_profile(
    profiles: &mut Vec<ApiProfile>,
    original_name: Option<&str>,
    profile: ApiProfile,
) -> Result<(), String> {
    let original_index = original_name.and_then(|name| {
        profiles
            .iter()
            .position(|item| item.name.trim().eq_ignore_ascii_case(name.trim()))
    });
    let duplicate_index = profiles
        .iter()
        .position(|item| item.name.trim().eq_ignore_ascii_case(profile.name.trim()));
    if duplicate_index.is_some() && duplicate_index != original_index {
        return Err(format!("配置名称已存在：{}", profile.name.trim()));
    }
    if let Some(index) = original_index {
        profiles[index] = profile;
    } else {
        profiles.push(profile);
    }
    Ok(())
}

fn remove_profile(profiles: &mut Vec<ApiProfile>, name: &str) -> bool {
    let original_len = profiles.len();
    profiles.retain(|profile| !profile.name.trim().eq_ignore_ascii_case(name.trim()));
    profiles.len() != original_len
}

unsafe fn select_profile(state: &mut SettingsState, name: &str) {
    state.draft_active_profile = name.to_string();
    populate_profile_list_safely(state);
}

fn all_controls(controls: &Controls) -> [HWND; 21] {
    [
        controls.nav_service,
        controls.nav_prompt,
        controls.nav_behavior,
        controls.api_profile,
        controls.profile_name,
        controls.profile_new,
        controls.profile_delete,
        controls.test_api,
        controls.api_key,
        controls.show_key,
        controls.base_url,
        controls.model,
        controls.temperature,
        controls.max_tokens,
        controls.system_prompt,
        controls.hotkey,
        controls.play_sound,
        controls.auto_start,
        controls.error_details,
        controls.reset,
        controls.save,
    ]
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature_format_is_compact() {
        assert_eq!(format_temperature(0.3), "0.3");
        assert_eq!(format_temperature(1.0), "1");
        assert_eq!(format_temperature(1.25), "1.25");
    }

    #[test]
    fn native_scale_uses_96_dpi_as_the_baseline() {
        assert_eq!(scale(44, 96), 44);
        assert_eq!(scale(44, 144), 66);
        assert_eq!(scale(1, 120), 1);
    }

    #[test]
    fn single_line_field_content_is_inset_symmetrically() {
        assert_eq!(field_content([234, 174, 588, 44], 96), [246, 184, 564, 24]);
        assert_eq!(field_content([351, 261, 882, 66], 144), [369, 276, 846, 36]);
    }

    #[test]
    fn header_title_subtitle_and_first_label_never_overlap() {
        let [title_top, title_bottom, subtitle_top, subtitle_bottom] = header_vertical_metrics(96);
        let first_label_top = settings_header_height(96) + 10 - 24;

        assert!(title_top < title_bottom);
        assert!(title_bottom < subtitle_top);
        assert!(subtitle_top < subtitle_bottom);
        assert!(subtitle_bottom + 14 <= first_label_top);
    }

    #[test]
    fn long_error_status_has_an_explicit_compact_summary() {
        let full = "API 返回错误 403：服务端返回了一段很长的诊断内容";
        let summary = concise_status(full, 18);

        assert!(summary.ends_with('…'));
        assert_eq!(summary.chars().count(), 19);
        assert_eq!(concise_status("API 连接正常", 18), "API 连接正常");
    }

    #[test]
    fn multiline_field_content_never_touches_the_outer_border() {
        let outer = [234, 126, 588, 340];
        let inner = multiline_field_content(outer, 96);
        assert_eq!(inner, [246, 136, 564, 320]);
        assert_eq!(
            inner[0] - outer[0],
            outer[2] - (inner[0] - outer[0]) - inner[2]
        );
        assert_eq!(
            inner[1] - outer[1],
            outer[3] - (inner[1] - outer[1]) - inner[3]
        );
    }

    #[test]
    fn programmatic_population_never_marks_the_form_dirty() {
        assert!(!should_mark_dirty(true, EN_CHANGE, ID_MODEL));
        assert!(should_mark_dirty(false, EN_CHANGE, ID_MODEL));
        assert!(!should_mark_dirty(false, EN_SETFOCUS, ID_MODEL));
    }

    #[test]
    fn profile_staging_distinguishes_update_rename_and_new_profile() {
        let mut profiles = vec![ApiProfile {
            name: "工作".into(),
            ..ApiProfile::default()
        }];
        let mut updated = profiles[0].clone();
        updated.model = "updated-model".into();
        stage_profile(&mut profiles, Some("工作"), updated).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].model, "updated-model");

        let mut renamed = profiles[0].clone();
        renamed.name = "硅基流动".into();
        stage_profile(&mut profiles, Some("工作"), renamed).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "硅基流动");

        let second = ApiProfile {
            name: "备用".into(),
            ..ApiProfile::default()
        };
        stage_profile(&mut profiles, None, second).unwrap();
        assert_eq!(profiles.len(), 2);
    }

    #[test]
    fn profile_staging_rejects_duplicate_names_and_supports_removal() {
        let mut profiles = vec![
            ApiProfile {
                name: "工作".into(),
                ..ApiProfile::default()
            },
            ApiProfile {
                name: "备用".into(),
                ..ApiProfile::default()
            },
        ];
        let duplicate = ApiProfile {
            name: " 备用 ".into(),
            ..ApiProfile::default()
        };
        assert!(stage_profile(&mut profiles, Some("工作"), duplicate).is_err());
        assert!(remove_profile(&mut profiles, " 工作 "));
        assert_eq!(profiles.len(), 1);
    }
}
