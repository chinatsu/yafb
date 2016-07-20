// TODO: Clean up the HWND bullshit, somehow
#![feature(custom_derive)]
extern crate kernel32;
extern crate user32;
extern crate winapi;
extern crate time;
#[macro_use]
extern crate lazy_static;
extern crate rustc_serialize;
extern crate toml_config;

mod implying;
use implying::*;
mod wide;
use wide::ToWide;

use std::path::Path;
use toml_config::ConfigFactory;

use std::io;
use std::io::prelude::*;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::sync::Arc;
use std::sync::Mutex;
use std::iter::Iterator;

// Global list of clients, wrapped in a bunch of magic
lazy_static! {
    static ref CLIENTS: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
    static ref CONFIG: Config = ConfigFactory::load(Path::new("yafb.toml"));
}

// (r (x, y)), where r is the sum of a client's height and width,
// x and y being the relative position of a battery change confirmation button.
static OFFSETS: [(i32, (i32, i32)); 14] = [(1400, (350, 360)), // 800x600
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
                                           (3120, (910, 595)) /* 1920x1200 */];

fn main() {
    let version: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!("yafb v{}", version.unwrap_or(" unknown"));
    let mode = mode_select();
    if mode == 0 {
        // "Healbot"-mode, presses an F-key at a specified interval.
        let procs = enum_clients();
        let client = prompt_user(procs);
        let setup = setup(false);
        let timeout = std::time::Duration::new(setup.timeout, 0);
        let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
        println!("Running spammer on {}", client.name);
        loop {
            if unsafe { user32::IsWindow(hwnd) == 0 } {
                raise_error(format!("Client: {} no longer exists!", client.name));
            }
            push_button(hwnd, setup.keysel, 500);
            std::thread::sleep(timeout);
        }
    }
    if mode == 1 {
        // Collector-mode, starts/stops collecting and changes battery.
        let procs = enum_clients();
        let mut cl = Vec::new(); // Make a fresh Vec to only include the logged on collectors.
        for client in &procs {
            if client.collect {
                cl.push(client.to_owned());
            }
        }
        println!("[Note] There is a modifier setting in yafb.toml,\n
        increasing this value will slow down battery changes if the default (1) is too fast\n");
        if !cl.is_empty() {
            println!("Found {} connected collectors:", cl.len());
            for c in &cl {
                println!("  {}", c.name);
            }
            let setup = setup(true);
            let timeout = std::time::Duration::new(setup.timeout, 0);
            let modifier: u64 = CONFIG.collector.modifier;
            loop {
                for client in &cl {
                    if unsafe { user32::IsWindow(client.hwnd as winapi::windef::HWND) } == 0 {
                        raise_error(format!("Client: {} no longer exists!", client.name));
                    }
                    change_battery(client, setup.keysel);
                    std::thread::sleep(std::time::Duration::new(modifier, 0));
                }
                std::thread::sleep(timeout);
            }
        } else {
            raise_error(format!("No configured collectors logged on!"));
        }

    }
    if mode == 2 {
        // "Kitebot"-mode, not really functional, as WASD is not accepted by the client if it's
        // not in focus. The idea is to have the user start autorunning, and this mode would keep
        // pressing D to turn the character at set intervals to have the player run in circles.
        let procs = enum_clients();
        let client = prompt_user(procs);
        let keytime = CONFIG.kitebot.keytime;
        let timeout = CONFIG.kitebot.timeout;
        let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
        println!("Running kitebot on {}", client.name);
        loop {
            if unsafe { user32::IsWindow(hwnd) } == 0 {
                let errmsg = format!("Client: {} no longer exists!", client.name);
                raise_error(errmsg);
            }
            push_button(hwnd, 0x44, keytime);
            std::thread::sleep(std::time::Duration::from_millis(timeout));
        }
    }
    if mode == 3 {
        // "Test"-mode. Currently hardcoded, don't bother using.
        println!("[Warning!] This mode is highly unstable, and will cause weird behavior");
        let procs = enum_clients();
        let client = prompt_user(procs);
        let offset = u32::from_str_radix(&CONFIG.dungeons.offset, 16).ok().unwrap();
        let off2 = u32::from_str_radix("164", 16).ok().unwrap();
        let address = get_base_addr(client.pid);
        let ref sel: Dungeon;
        let mut count = 0;
        println!("Available dungeons:");
        for k in &CONFIG.dungeon {
            println!("[{}] {}", count, k.name);
            count += 1;
        }
        loop {
            let input = user_input("Select dungeon > ".to_string());
            let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
            let input_int = match input_opt {
                Some(input_int) => input_int,
                None => raise_error(format!("{} is numberwang!", input.trim_right())),
            };
            if input_int <= CONFIG.dungeon.len() {
                sel = &CONFIG.dungeon[input_int];
                break;
            } else {
                let errmsg = format!("{} is too high!", input_int);
                raise_error(errmsg);
            }
        }
        println!("[Warning!] You must be currently in the dungeon to teleport without crashing");
        count = 0;
        for loc in &sel.coordinates {
            let pointer = read_memory(client.pid, address + offset) + off2;
            count += 1;
            let x: f32 = loc[0];
            let y: f32 = loc[1];
            let z: f32 = loc[2];
            println!("Location {} in {}, ready to teleport", count, sel.name);
            user_input("Press enter to teleport > ".to_string());
            change_pos(client.pid, pointer, [x, y, z]);
        }
    }
    if mode == 4 {
        let procs = enum_clients();
        let client = prompt_user(procs);
        println!("{}{}{}", "Running notification mode.\n",
                "  (Might put this as an option to run during other modes)\n",
                "---------------------------------------------------------");
        let pid = client.pid;
        let hwnd = client.hwnd;
        notification(pid, hwnd);
    }
}


fn notification(pid: u32, hwnd: u32) {
    let mut new_message = String::new();
    let offset = u32::from_str_radix(&CONFIG.systembuffer.offset, 16).ok().unwrap();
    let off0 = u32::from_str_radix("BC", 16).ok().unwrap();
    let off1 = u32::from_str_radix("728", 16).ok().unwrap();
    let address = get_base_addr(pid);
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let pointer = read_memory(pid, read_memory(pid, read_memory(pid, address + offset) + off0) + off1);
        let mut r = 0;
        let mut buffer = String::new();
        loop {
            let (memory, stop) = read_buffer(pid, pointer + r);
            buffer.push_str(&memory);
            if stop {
                break;
            }
            r += 2048;
        }
        if buffer.lines().last().is_none() {
            raise_error(format!("Unable to read memory, client closed or not logged in?"));
        }
        let message = buffer.lines().last().unwrap();

        if message.chars().next().unwrap() == '[' &&
            (&message[..8] != "[Notice]") &&
            (message != new_message) {
                    unsafe { user32::FlashWindow(hwnd as winapi::windef::HWND, 0) };
                    println!("[{}] {}", time::strftime("%Y-%m-%d %H:%M:%S", &time::now()).unwrap(), message);
                    new_message = message.to_string();
        }
    }
}

fn player_pos(pid: u32) {
    let address = get_base_addr(pid);
    let offset = u32::from_str_radix(&CONFIG.dungeons.offset, 16).ok().unwrap();
    let x = u32::from_str_radix("164", 16).ok().unwrap();
    loop {
        let pointer = read_memory(pid, address + offset) + x;
        let xpos: f32 = unsafe { std::mem::transmute(read_memory(pid, pointer)) };
        let zpos: f32 =
            unsafe { std::mem::transmute(read_memory(pid, pointer + 8)) };
        let ypos: f32 =
            unsafe { std::mem::transmute(read_memory(pid, pointer + 4)) };
        print!("x: {:?}\ty: {:?}\tz: {:?}\t\t\t\t\t\r",
               xpos,
               ypos,
               zpos);
        let _ = io::stdout().flush();
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

fn user_input(prompt: String) -> String {
    // I use this enough to warrant its own function.
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    let _ = io::stdin().read_line(&mut input) ;
    input
}

fn mode_select() -> u8 {
    // Prints available modes, and prompts the user to select a desired ID (0, 1, or 2).
    println!("Available modes:");
    println!("[0] Healbot");
    println!("[1] Collector");
    println!("[2] Kitebot (broken as fuck)");
    println!("[3] Dungeon teleporter (mostly untested)");
    println!("[4] Notification mode");
    loop {
        let input = user_input("Select mode > ".to_string());
        let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
        let input_int = match input_opt {
            Some(input_int) => input_int,
            None => raise_error(format!("{} is numberwang!", input.trim_right())),
        };
        if input_int <= 4 {
            return input_int as u8;
        } else {
            raise_error(format!("{} is too high!", input_int));
        }
    }
}

fn change_battery(client: &Client, key: u64) {
    let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
    let modifier: u64 = CONFIG.collector.modifier;
    let mut iconic = 0u8;
    if unsafe { user32::IsIconic(hwnd) } != 0 {
        iconic = 1;
        unsafe { user32::ShowWindow(hwnd, 1i32) }; // Show the window if the client is minimized.
        // Safety wait for the window restore animation to finish.
        std::thread::sleep(std::time::Duration::new(modifier, 0));
    }
    let rect = get_window_pos(hwnd);
    click_mouse(hwnd, rect.left + 83, rect.top + 187); // click "Stop"
    push_button(hwnd, key, 500); // press battery bound F-key
    // The server I'm playing has an additional window
    // in an attempt to thwart existing automation tools, so let's click that first.
    click_mouse(hwnd, rect.left + client.offx + 30, rect.top + client.offy);
    std::thread::sleep(std::time::Duration::from_millis(200 * modifier));
    // press OK to replace battery
    click_mouse(hwnd, rect.left + client.offx, rect.top + client.offy);
    std::thread::sleep(std::time::Duration::from_millis(500 * modifier));
    click_mouse(hwnd, rect.left + 83, rect.top + 187); // Click "Start".
    if iconic != 0 {
        // Minimize the client if it was minimized at the start.
        unsafe { user32::CloseWindow(hwnd) };
    };
}


fn prompt_user(clients: Vec<Client>) -> Client {
    // Prompts the user to select a specific client.
    let client;
    let mut count = 0u32;
    if clients.len() > 1 {
        println!("Currently open clients:");
        for x in &clients {
            println!("[{}] {}", count, x.name);
            count += 1;
        }
        loop {
            let input = user_input("Select client ID > ".to_string());
            let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
            let input_int = match input_opt {
                Some(input_int) => input_int,
                None => raise_error(format!("{} is numberwang!", input.trim_right())),
            };
            if input_int <= clients.len() {
                client = &clients[input_int];
                break;
            } else {
                raise_error(format!("{} is too high!", input_int));
            }
        }
    } else {
        // If there's only one client open, select it without prompting the user.
        client = &clients[0];
    }
    client.to_owned()
}

fn setup(collector: bool) -> Setup {
    // Prompts the user to specify timeout and target F-key, if `collector` is true,
    // the timeout will be read from the config file.
    let timeout;
    let fkey;
    if !collector {
        loop {
            let input = user_input("Select heal timeout (in seconds) > ".to_string());
            let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
            let input_int = match input_opt {
                Some(input_int) => input_int,
                None => raise_error(format!("{} is numberwang!", input.trim_right())),
            };
            timeout = input_int as u64;
            break;
        }
    } else {
        timeout = CONFIG.collector.timeout;
    }
    loop {
        let input = user_input("Select F-key (1 equals F1, etc.) > ".to_string());
        let input_code = match input.trim_right().parse::<u8>().ok() {
            // Not a very nice solution, but I'm unsure if I could make it nicer.
            Some(1u8) => 0x70,
            Some(2u8) => 0x71,
            Some(3u8) => 0x72,
            Some(4u8) => 0x73,
            Some(5u8) => 0x74,
            Some(6u8) => 0x75,
            Some(7u8) => 0x76,
            Some(8u8) => 0x77,
            Some(9u8) => 0x78,
            None => raise_error(format!("{} is numberwang!", input.trim_right())) as u64,
            _ => raise_error(format!("{} is numberwang!", input.trim_right())) as u64,
        };
        fkey = input_code;
        break;
    }
    Setup {
        timeout: timeout.to_owned(),
        keysel: fkey.to_owned(),
    }
}

fn enum_clients() -> Vec<Client> {
    // Populates `CLIENTS` with metadata about each open client.
    unsafe extern "system" fn callback(hwnd: winapi::windef::HWND, _: i64) -> i32 {
        let exe = CONFIG.base.executable.as_str();
        let lock = CLIENTS.clone();
        let mut vec = lock.lock().unwrap(); // Access the Vec in here...
        let mut pid = 0u32;
        if user32::IsWindowVisible(hwnd) != 0 && (user32::IsWindowEnabled(hwnd) != 0) {
            // I wish I didn't need to get the PID on every window, but it's necessary.
            user32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
            let r = get_base_name(pid); // Get the executable name.
            if &r == exe {
                // And make sure it matches our target exe name.
                let mut iconized = 0;
                let mut rect = winapi::windef::RECT {
                    left: 0i32,
                    top: 0i32,
                    right: 0i32,
                    bottom: 0i32,
                };
                if user32::IsIconic(hwnd) != 0 {
                    // The window can't be minimized in order to get its coordinates, so let's
                    // show it.
                    user32::ShowWindow(hwnd, 1i32);
                    iconized = 1;
                }

                user32::GetClientRect(hwnd, &mut rect as *mut winapi::windef::RECT);
                if iconized == 1 {
                    user32::CloseWindow(hwnd); // Minimize it again, if it was minimized before.
                }
                let mut offset: (i32, i32) = (0, 0);
                for off in &OFFSETS {
                    if (rect.right + rect.bottom) as i32 == off.0 {
                        offset = off.1; // Get our offsets from `OFFSETS`.
                        break;
                    }
                }
                let account = read_account_name(pid).unwrap();
                let mut name = format!("{} (account name)", account);
                let mut collect = false;
                // Account name will show if the account hasn't been configured in characters.ini.
                let accounts: Accounts = ConfigFactory::load(Path::new("characters.toml"));
                for x in accounts.account {
                    if x.account == account {
                        collect = x.collect;
                        name = x.name;
                        break;
                    }
                }
                let client = Client {
                    collect: collect,
                    pid: pid.to_owned(),
                    hwnd: hwnd.to_owned() as u32,
                    name: name.to_owned(),
                    offx: offset.0.to_owned(),
                    offy: offset.1.to_owned(),
                };
                vec.push(client)
            }
        }
        1
    }
    unsafe { user32::EnumWindows(Some(callback), 0i64) };
    let lock = CLIENTS.clone();
    let vec = lock.lock().unwrap();
    vec.clone()
}

fn get_base_name(pid: u32) -> OsString {
    // Get an executable name from process ID.
    const BUF_LEN: usize = 64;
    let h_process = unsafe { kernel32::OpenProcess(0x0400 | 0x0010, 0, pid.to_owned()) };
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
    p // Return the OsString read from K32GetModuleBaseNameW.
}

fn read_account_name(pid: u32) -> std::result::Result<std::string::String, std::string::FromUtf8Error>  {
    // Reads a memory location and returns the account name.
    let address = get_base_addr(pid);
    let offset = u32::from_str_radix(&CONFIG.base.offset, 16).ok().unwrap();
    let pointer = read_memory(pid, address + offset); // get a known pointer to the account name
    const BUF_LEN: u64 = 16;
    let mut buffer = [0u8; BUF_LEN as usize];
    let bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::ReadProcessMemory(h_process,
                                    pointer as winapi::minwindef::LPCVOID,
                                    buffer.as_mut_ptr() as winapi::minwindef::LPVOID,
                                    BUF_LEN,
                                    bytes_read as *mut u64);
    };
    unsafe { kernel32::CloseHandle(h_process) };
    let string = buffer.iter().cloned().filter(|x| *x != 0).collect::<Vec<u8>>();
    String::from_utf8(string.clone())
}

fn read_memory(pid: u32, address: u32) -> u32 {
    // Kind of "generic" memory read function, though it only reads a u32 at a specified location.
    const BUF_LEN: u64 = 1;
    let mut buffer = [0u32; 3];
    let bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::ReadProcessMemory(h_process,
                                    address as winapi::minwindef::LPCVOID,
                                    buffer.as_mut_ptr() as winapi::minwindef::LPVOID,
                                    4u64 * BUF_LEN,
                                    bytes_read as *mut u64);
    };
    unsafe { kernel32::CloseHandle(h_process) };
    buffer[0]
}

fn change_pos(pid: u32, address: u32, mut value: [f32; 3]) {
    // Experimental function to teleport a player elsewhere. Functional on shorter teleports.
    const BUF_LEN: u64 = 3;
    let bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::WriteProcessMemory(h_process,
                                     address as winapi::minwindef::LPVOID,
                                     value.as_mut_ptr() as winapi::minwindef::LPVOID,
                                     4u64 * BUF_LEN,
                                     bytes_read as *mut u64);
    };
    unsafe { kernel32::CloseHandle(h_process) };
}

fn read_buffer(pid: u32, address: u32) -> (String, bool) {
    // Similar to `read_memory`, though this reads a section in memory where the account name lies.
    const BUF_LEN: u64 = 2048;
    let mut buffer = [0u8; BUF_LEN as usize];
    let mut stop = false;
    let bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::ReadProcessMemory(h_process,
                                    address as winapi::minwindef::LPCVOID,
                                    buffer.as_mut_ptr() as winapi::minwindef::LPVOID,
                                    BUF_LEN,
                                    bytes_read as *mut u64);
    };
    unsafe { kernel32::CloseHandle(h_process) };
    let string = buffer.iter().cloned().take_while(|x| *x != 0).collect::<Vec<u8>>();
    if string.len() < 2048 {
        stop = true;
    }
    (String::from_utf8(string.clone()).unwrap(), stop)
}


fn get_base_addr(pid: u32) -> u32 {
    // Get the base address for the first module loaded in the executable.
    let snap = unsafe { kernel32::CreateToolhelp32Snapshot(0x00000008, pid) };
    let mut mod32 = winapi::tlhelp32::MODULEENTRY32W {
        dwSize: 0u32,
        th32ModuleID: 0u32,
        th32ProcessID: 0u32,
        GlblcntUsage: 0u32,
        ProccntUsage: 0u32,
        modBaseAddr: 0 as *mut u8,
        modBaseSize: 0u32,
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
    // Send a button press to a specified hwnd.
    unsafe {
        user32::PostMessageW(hwnd, 0x0100, key, 0i64);
    };
    std::thread::sleep(std::time::Duration::from_millis(millis));
    unsafe {
        user32::PostMessageW(hwnd, 0x0101, key, 0i64);
    };
}

fn click_mouse(hwnd: winapi::windef::HWND, tx: i32, ty: i32) {
    // Set the cursor to a position on the screen and click that location.
    unsafe {
        user32::SetCursorPos(tx, ty);
        user32::PostMessageW(hwnd, 0x0201, 0u64, 0i64);
        user32::PostMessageW(hwnd, 0x0202, 0u64, 0i64);
    }
}

fn get_window_pos(hwnd: winapi::windef::HWND) -> winapi::windef::RECT {
    // Get the window position.
    let mut rect = winapi::windef::RECT {
        left: 0i32,
        top: 0i32,
        right: 0i32,
        bottom: 0i32,
    };
    unsafe {
        user32::GetWindowRect(hwnd, &mut rect as *mut winapi::windef::RECT);
    };
    rect
}

fn raise_error(errmsg: String) -> usize {
     let _  = unsafe { user32::MessageBoxW(0 as winapi::windef::HWND,
                                           errmsg.to_wide_null().as_ptr(),
                                           errmsg.to_wide_null().as_ptr(),
                                           0u32)};
     panic!(errmsg);
     0
}
