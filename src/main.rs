extern crate kernel32;
extern crate user32;
extern crate winapi;
extern crate ini;
use ini::Ini;


#[macro_use]
extern crate lazy_static;

use std::io;
use std::io::prelude::*;
use std::os::windows::ffi::OsStringExt;
use std::ffi::OsString;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug)]
#[derive(Clone)]
struct Client {
    pid: u32,
    hwnd: u32,
    name: String,
    x: i32,
    y: i32,
    collect: bool
}

#[derive(Debug)]
struct Setup {
    timeout: u64,
    keysel: u64
}

#[derive(Debug)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32
}

lazy_static! {
    static ref THING: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
}

static OFFSETS: [(i32, (i32, i32)); 14] = [
    (1400, (350, 360)), // 800x600
    (1792, (465, 445)), // 1024x768
    (2000, (590, 420)), // 1280x720
    (2048, (590, 445)), // 1280x768
    (2080, (590, 460)), // 1280x800
    (2304, (590, 570)), // 1280x1024
    (2128, (630, 445)), // 1360x768
    (2450, (650, 585)), // 1400x1050
    (2340, (670, 510)), // 1440x900
    (2500, (750, 510)), // 1600x900
    (2800, (750, 595)), // 1600x1200
    (2730, (790, 585)), // 1680x1050
    (3000, (910, 595)), // 1920x1080
    (3120, (910, 595)) // 1920x1200
    ];

fn main() {
    println!("yafb v0.0.9");
    let mode = mode_select();
    if mode == 0 {
        let procs = enum_clients();
        let client = prompt_user(procs);
        let setup = setup(false);
        let timeout = std::time::Duration::new(setup.timeout, 0);
        let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
        println!("Running spammer on {}", client.name);
        loop {
            if unsafe { user32::IsWindow(hwnd) != 0 } {
                push_button(hwnd, setup.keysel, 500);
                std::thread::sleep(timeout);
            }
            else {
                println!("Client: {} no longer exists!", &client.name);
                std::thread::park();
            }
        }
    }
    if mode == 1 {
        let mut is_collector = false;
        let procs = enum_clients();
        let mut cl = Vec::new();
        for client in &procs {
            if client.collect {
                cl.push(&client.name);
                is_collector = true;
            }
        }
        if is_collector {
            println!("Found {} connected collectors: {:?}", cl.len(), cl);
            let setup = setup(true);
            let timeout = std::time::Duration::new(setup.timeout, 0);
            println!("Running");
            loop {
                for client in &procs {
                    if client.collect {
                        if unsafe { user32::IsWindow(client.hwnd as winapi::windef::HWND) } == 0 {
                            println!("Client: {} no longer exists!", &client.name);
                            std::thread::park();
                        }
                        change_battery(&client, setup.keysel);
                        std::thread::sleep(std::time::Duration::new(1, 0));
                    }
                }
            std::thread::sleep(timeout);
            }
        }
        else {
            println!("No configured collectors logged on!");
            std::thread::park();
        }
    }
    if mode == 2 {
        let procs = enum_clients();
        let client = prompt_user(procs);
        let config = load_config();
        let timeout = config["kitebot"]["timeout"].trim_right().parse::<u64>().ok().unwrap();
        let keytime = config["kitebot"]["keytime"].trim_right().parse::<u64>().ok().unwrap();
        let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
        println!("Running kitebot on {}", client.name);
        loop {
            if unsafe { user32::IsWindow(hwnd) } == 0  {
                println!("Client: {} no longer exists!", &client.name);
                std::thread::park();
            }
            push_button(hwnd, 0x44, keytime);
            std::thread::sleep(std::time::Duration::from_millis(timeout));
        }
    }

}

fn load_config() -> Ini {
    let conf = Ini::load_from_file("config.ini").unwrap();
    conf
}

fn user_input() -> String {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = String::new();
    stdin.read_line(&mut input);
    input
}

fn mode_select() -> u8 {
    let mut mode = 0u8;

    println!("Available modes:");
    println!("[0] Healbot");
    println!("[1] Collector");
    println!("[2] Kitebot (broken as fuck)");
    loop {
        write!(&mut io::stdout(), "Select mode > ");
        io::stdout().flush();
        let mut input = user_input();
        let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
        let input_int = match input_opt {
            Some(input_int) => input_int,
            None => panic!("{} is numberwang!", input.trim_right()),
        };
        if input_int <= 2 {
            mode = input_int as u8;
            break;
        }
        else {
            println!("{} is too high!", input_int);
            std::thread::park();
        }
    }
    mode
}

fn change_battery(client: &Client, key: u64) {
    let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
    let mut iconic = 0u8;
    if unsafe { user32::IsIconic(hwnd) } != 0 {
        iconic = 1;
        unsafe { user32::ShowWindow(hwnd, 1i32) };
    }
    std::thread::sleep(std::time::Duration::new(1, 0));
    let rect = get_window_pos(hwnd);
    let mut offset: (i32, i32) = (0, 0);
    for off in &OFFSETS {
        if (client.x + client.y) as i32 == off.0 {
            offset = off.1;
            break;
        }
    }
    click_mouse(hwnd, rect.left + 83, rect.top + 187);
    push_button(hwnd, key, 500);
    click_mouse(hwnd, rect.left + offset.0 + 30, rect.top + offset.1);
    std::thread::sleep(std::time::Duration::from_millis(200));
    click_mouse(hwnd, rect.left + offset.0, rect.top + offset.1);
    std::thread::sleep(std::time::Duration::from_millis(500));
    click_mouse(hwnd, rect.left + 83, rect.top + 187);
    if iconic != 0 {
        unsafe { user32::CloseWindow(hwnd) };
    };
}


fn prompt_user(clients: Vec<Client>) -> Client {
    let mut client;
    let mut count = 0u32;
    if clients.len() > 1 {
        println!("Currently open clients:");
        for x in &clients {
            println!("[{}] {}", count, x.name);
            count += 1;
        }
        loop {
            write!(&mut io::stdout(), "Select client ID > ");
            io::stdout().flush();

            let mut input = user_input();
            let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
            let input_int = match input_opt {
                Some(input_int) => input_int,
                None => panic!("{} is numberwang!", input.trim_right()),
            };
            if input_int <= clients.len() {
                client = &clients[input_int];
                break
            }
            else {
                panic!("{} is too high!", input_int)
            }
        }
    }
    else {
        client = &clients[0];
    }
    client.to_owned()
}

fn setup(collector: bool) -> Setup {
    let mut timeout;
    let mut fkey;
    if collector == false {
        loop {
            write!(&mut io::stdout(), "Select heal timeout (in seconds) > ");
            io::stdout().flush();

            let mut input = user_input();
            let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
            let input_int = match input_opt {
                Some(input_int) => input_int,
                None => panic!("{} is numberwang!", input.trim_right()),
            };
            timeout = input_int as u64;
            break
        }
    }
    else {
        let config = load_config();
        timeout = config["collector"]["timeout"].trim_right().parse::<u64>().ok().unwrap()
    }
    loop {
        write!(&mut io::stdout(), "Select F-key (1 equals F1, etc.) > ");
        io::stdout().flush();

        let mut input = user_input();
        let input_code = match input.trim_right().parse::<u8>().ok() {
            Some(1u8) => 0x70,
            Some(2u8) => 0x71,
            Some(3u8) => 0x72,
            Some(4u8) => 0x73,
            Some(5u8) => 0x74,
            Some(6u8) => 0x75,
            Some(7u8) => 0x76,
            Some(8u8) => 0x77,
            Some(9u8) => 0x78,
            None => panic!("{} is numberwang!", input.trim_right()),
            _ => panic!("{} is numberwang!", input.trim_right()),
        };
        fkey = input_code;
        break
    }
    Setup{timeout: timeout.to_owned(), keysel: fkey.to_owned()}
}

fn enum_clients() -> Vec<Client> {
    let lparam = 0i64;
    unsafe extern "system" fn callback(hwnd: winapi::windef::HWND, lparam: i64) -> i32 {
        let config = load_config();
        let exe: String = config["base"]["executable"].clone();
        let lock = THING.clone();
        let mut vec = lock.lock().unwrap();

        let mut somei32 = 0i32;
        let mut pid = 0u32;
        let mut rect = winapi::windef::RECT {
            left: 0i32,
            top: 0i32,
            right: 0i32,
            bottom: 0i32
        };
        if user32::IsWindowVisible(hwnd) != 0 && (user32::IsWindowEnabled(hwnd) != 0) {
            user32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
            let r = get_base_name(pid);
            if &r == exe.as_str() {
                let conf = Ini::load_from_file("characters.ini").unwrap();
                let mut iconized = 0;
                let mut is_collect = false;
                if user32::IsIconic(hwnd) != 0 {
                    unsafe { user32::ShowWindow(hwnd, 1i32)};
                    iconized = 1;
                }
                user32::GetClientRect(hwnd, &mut rect as *mut winapi::windef::RECT);
                if iconized == 1 {
                    user32::CloseWindow(hwnd);
                }
                let text = get_name(pid);
                let mut name = format!("{} (account name)", text.to_string());
                for (sec, prop) in conf.iter() {
                    let acc = sec.clone().unwrap();
                    if acc == text {
                        let p = prop.clone();
                        name = p["name"].clone();
                        if p["collect"] == "true" {
                            is_collect = true;
                        }
                    }
                }
                let wew: u32 = hwnd as u32;
                let mut client = Client {
                    collect: is_collect,
                    pid: pid.to_owned(),
                    hwnd: wew.to_owned(),
                    name: name.to_owned(),
                    x: rect.right.to_owned(),
                    y: rect.bottom.to_owned()
                };
                vec.push(client)
            }
        }
        somei32 = 1;
        somei32
    }
    unsafe { user32::EnumWindows(Some(callback), lparam as i64) };
    let lock = THING.clone();
    let mut vec = lock.lock().unwrap();
    let clients = vec.clone();
    clients
}

fn get_base_name(pid: u32) -> OsString {
    const BUF_LEN: usize = 64;
    let h_process = unsafe { kernel32::OpenProcess(0x0400 | 0x0010, 0, pid.to_owned())};
    let mut p: OsString = OsString::new();
    if h_process as usize != 0 {
        let mut modname = [0u16; BUF_LEN];
        let len = unsafe {
            kernel32::K32GetModuleBaseNameW(h_process,
                                            std::ptr::null_mut(),
                                            modname.as_mut_ptr(),
                                            4u32 * BUF_LEN as u32)
        };
        p = OsStringExt::from_wide(&modname[..len as usize]);
        unsafe { kernel32::CloseHandle(h_process) };
    };
    p
}

fn get_name(pid: u32) -> String {
    let config = load_config();
    let address = get_base_addr(pid);
    let offset = u32::from_str_radix(config["base"]["offset"].trim_right(), 16).ok().unwrap();
    let pointer = get_pointer(pid, address + offset);
    let text = get_text(pid, pointer as u32);
    let test = text.split_terminator('\0').next().unwrap();
    test.to_string()
}

fn get_pointer(pid: u32, address: u32) -> u32 {
    const BUF_LEN: u64 = 1;
    let mut buffer = [0u32];
    let mut bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::ReadProcessMemory(
            h_process,
            address as winapi::minwindef::LPCVOID,
            buffer.as_mut_ptr() as winapi::minwindef::LPVOID,
            4u64 * BUF_LEN,
            bytes_read as *mut u64
        );
    };
    unsafe { kernel32::CloseHandle(h_process) };
    buffer[0]
}

fn get_text(pid: u32, address: u32) -> String {
    const BUF_LEN: u64 = 16;
    let mut buffer = [0u8; BUF_LEN as usize];
    let mut bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::ReadProcessMemory(
            h_process,
            address as winapi::minwindef::LPCVOID,
            buffer.as_mut_ptr() as winapi::minwindef::LPVOID,
            1u64 * BUF_LEN,
            bytes_read as *mut u64
        );
    };
    unsafe { kernel32::CloseHandle(h_process) };
    let name = std::str::from_utf8(&buffer[..]).unwrap().to_owned();
    name
}

fn get_base_addr(pid: u32) -> u32 {
    let snap = unsafe { kernel32::CreateToolhelp32Snapshot(0x00000008, pid) };
    let mut mod32 = winapi::tlhelp32::MODULEENTRY32W {
        dwSize: 0u32,
        th32ModuleID:  0u32,
        th32ProcessID:  0u32,
        GlblcntUsage:  0u32,
        ProccntUsage:  0u32,
        modBaseAddr: 0 as *mut u8,
        modBaseSize:  0u32,
        hModule: 0 as winapi::minwindef::HMODULE,
        szModule: [0u16; 256],
        szExePath: [0u16; 260],
    };
    mod32.dwSize = std::mem::size_of::<winapi::tlhelp32::MODULEENTRY32W>() as u32;
    unsafe { kernel32::Module32NextW(snap, &mut mod32 as *mut winapi::tlhelp32::MODULEENTRY32W) };
    unsafe { kernel32::CloseHandle(snap) };
    mod32.modBaseAddr as u32

}

fn push_button(hwnd: winapi::windef::HWND, key: u64, millis: u64) {
    unsafe { user32::PostMessageW(hwnd, 0x0100, key, 0i64); };
    std::thread::sleep(std::time::Duration::from_millis(millis));
    unsafe { user32::PostMessageW(hwnd, 0x0101, key, 0i64); };
}

fn click_mouse(hwnd: winapi::windef::HWND, tx: i32, ty: i32) {
    unsafe {
        user32::SetCursorPos(tx, ty);
        user32::PostMessageW(hwnd, 0x0201, 0u64, 0i64);
        user32::PostMessageW(hwnd, 0x0202, 0u64, 0i64);
    }
}

fn make_lparam(low: u16, high: u16) -> i64 {
    let lparam: i64 = (low as i64 & 0xFFFF) | ((high as i64 & 0xFFFF) << 16) as i64;
    lparam
}

fn get_window_pos(hwnd: winapi::windef::HWND) -> Rect {
    let mut rect = winapi::windef::RECT {
        left: 0i32,
        top: 0i32,
        right: 0i32,
        bottom: 0i32
    };
    unsafe {
        user32::GetWindowRect(hwnd, &mut rect as *mut winapi::windef::RECT);

    };
    Rect{left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom}
}
