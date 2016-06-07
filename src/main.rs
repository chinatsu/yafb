// TODO: Clean up the HWND bullshit, somehow

extern crate kernel32;
extern crate user32;
extern crate winapi;
extern crate ini;
#[macro_use]
extern crate lazy_static;

use ini::Ini;
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
    hwnd: u32, // I don't seem to be able to use winapi::windef::HWND as type directly here.
    name: String,
    collect: bool,
    offx: i32, // X offset for collecting purposes
    offy: i32, // Y offset for collecting purposes
}

#[derive(Debug)]
struct Setup {
    timeout: u64,
    keysel: u64,
}


// Global list of clients, wrapped in a bunch of magic
lazy_static! {
    static ref CLIENTS: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
}

// Reading this once here feels nicer than calling load_from_file whenever I want to use it
lazy_static! {
    static ref CONF: Ini = Ini::load_from_file("config.ini").unwrap();
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
                println!("Client: {} no longer exists!", &client.name);
                std::thread::park();
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
        if cl.len() != 0 {
            println!("Found {} connected collectors:", cl.len());
            for c in &cl {
                println!("  {}", c.name);
            }
            let setup = setup(true);
            let timeout = std::time::Duration::new(setup.timeout, 0);
            loop {
                for client in &cl {
                    if unsafe { user32::IsWindow(client.hwnd as winapi::windef::HWND) } == 0 {
                        println!("Client: {} no longer exists!", &client.name);
                        std::thread::park();
                    }
                    change_battery(&client, setup.keysel);
                    std::thread::sleep(std::time::Duration::new(1, 0));
                }
                std::thread::sleep(timeout);
            }
        } else {
            println!("No configured collectors logged on!");
            std::thread::park();
        }

    }
    if mode == 2 {
        // "Kitebot"-mode, not really functional, as WASD is not accepted by the client if it's
        // not in focus. The idea is to have the user start autorunning, and this mode would keep
        // pressing D to turn the character at set intervals to have the player run in circles.
        let procs = enum_clients();
        let client = prompt_user(procs);
        let timeout = CONF["kitebot"]["timeout"].trim_right().parse::<u64>().ok().unwrap();
        let keytime = CONF["kitebot"]["keytime"].trim_right().parse::<u64>().ok().unwrap();
        let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
        println!("Running kitebot on {}", client.name);
        loop {
            if unsafe { user32::IsWindow(hwnd) } == 0 {
                println!("Client: {} no longer exists!", &client.name);
                std::thread::park();
            }
            push_button(hwnd, 0x44, keytime);
            std::thread::sleep(std::time::Duration::from_millis(timeout));
        }
    }
    if mode == 3 {
        // "Test"-mode. Currently hardcoded, don't bother using.
        let procs = enum_clients();
        let client = prompt_user(procs);
        let hwnd: winapi::windef::HWND = client.hwnd as winapi::windef::HWND;
        let address = get_base_addr(client.pid);
        let offset = u32::from_str_radix("00867CD8", 16).ok().unwrap();
        let x = u32::from_str_radix("164", 16).ok().unwrap();
        let position: [f32; 3] = [845.28503, 57.0, 1275.8212];
        let mut pointer = read_memory(client.pid, address + offset);
        change_pos(client.pid, pointer + x, position);
        loop {
            pointer = read_memory(client.pid, address + offset);
            let xpos: f32 = unsafe { std::mem::transmute(read_memory(client.pid, pointer + x)) };
            let zpos: f32 =
                unsafe { std::mem::transmute(read_memory(client.pid, pointer + x + 8)) };
            let ypos: f32 =
                unsafe { std::mem::transmute(read_memory(client.pid, pointer + x + 4)) };
            write!(&mut io::stdout(),
                   "x: {:?}\ty: {:?}\tz: {:?}\t\t\t\t\t\r",
                   xpos,
                   ypos,
                   zpos);
            io::stdout().flush();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

}

fn user_input() -> String {
    // I use this enough to warrant its own function.
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut input = String::new();
    stdin.read_line(&mut input);
    input
}

fn mode_select() -> u8 {
    // Prints available modes, and prompts the user to select a desired ID (0, 1, or 2).
    let mut mode = 0u8;

    println!("Available modes:");
    println!("[0] Healbot");
    println!("[1] Collector");
    println!("[2] Kitebot (broken as fuck)");
    println!("[3] Test mode");
    loop {
        write!(&mut io::stdout(), "Select mode > ");
        io::stdout().flush();
        let mut input = user_input();
        let input_opt: Option<usize> = input.trim_right().parse::<usize>().ok();
        let input_int = match input_opt {
            Some(input_int) => input_int,
            None => panic!("{} is numberwang!", input.trim_right()),
        };
        if input_int <= 3 {
            mode = input_int as u8;
            break;
        } else {
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
        unsafe { user32::ShowWindow(hwnd, 1i32) }; // Show the window if the client is minimized.
        // Safety wait for the window restore animation to finish.
        std::thread::sleep(std::time::Duration::new(1, 0));
    }
    let rect = get_window_pos(hwnd);
    click_mouse(hwnd, rect.left + 83, rect.top + 187); // click "Stop"
    push_button(hwnd, key, 500); // press battery bound F-key
    // The server I'm playing has an additional window
    // in an attempt to thwart existing automation tools, so let's click that first.
    click_mouse(hwnd, rect.left + client.offx + 30, rect.top + client.offy);
    std::thread::sleep(std::time::Duration::from_millis(200));
    // press OK to replace battery
    click_mouse(hwnd, rect.left + client.offx, rect.top + client.offy);
    std::thread::sleep(std::time::Duration::from_millis(500));
    click_mouse(hwnd, rect.left + 83, rect.top + 187); // Click "Start".
    if iconic != 0 {
        // Minimize the client if it was minimized at the start.
        unsafe { user32::CloseWindow(hwnd) };
    };
}


fn prompt_user(clients: Vec<Client>) -> Client {
    // Prompts the user to select a specific client.
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
                break;
            } else {
                panic!("{} is too high!", input_int)
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
            break;
        }
    } else {
        timeout = CONF["collector"]["timeout"].trim_right().parse::<u64>().ok().unwrap()
    }
    loop {
        write!(&mut io::stdout(), "Select F-key (1 equals F1, etc.) > ");
        io::stdout().flush();
        let mut input = user_input();
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
            None => panic!("{} is numberwang!", input.trim_right()),
            _ => panic!("{} is numberwang!", input.trim_right()),
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
    unsafe extern "system" fn callback(hwnd: winapi::windef::HWND, lparam: i64) -> i32 {
        let exe: String = CONF["base"]["executable"].clone();
        let lock = CLIENTS.clone();
        let mut vec = lock.lock().unwrap(); // Access the Vec in here...
        // The function gets angry if it doesn't return a value, so let's use some i32.
        let mut somei32 = 0i32;
        let mut pid = 0u32;
        if user32::IsWindowVisible(hwnd) != 0 && (user32::IsWindowEnabled(hwnd) != 0) {
            // I wish I didn't need to get the PID on every window, but it's necessary.
            user32::GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
            let r = get_base_name(pid); // Get the executable name.
            if &r == exe.as_str() {
                // And make sure it matches our target exe name.
                let conf = Ini::load_from_file("characters.ini").unwrap(); // Get account info.
                let mut iconized = 0;
                let mut is_collect = false;
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
                let account = get_name(pid);
                // Account name will show if the account hasn't been configured in characters.ini.
                let mut name = format!("{} (account name)", account);
                for (sec, prop) in conf.iter() {
                    let acc = sec.clone().unwrap();
                    if acc == account {
                        let p = prop.clone();
                        name = p["name"].clone(); // Set the character name from characters.ini.
                        // Would be ok to accept "true", "yes", 1 and so on if possible..
                        if p["collect"] == "true" {
                            is_collect = true;
                        }
                    }
                }
                let client = Client {
                    collect: is_collect,
                    pid: pid.to_owned(),
                    hwnd: hwnd.to_owned() as u32,
                    name: name.to_owned(),
                    offx: offset.0.to_owned(),
                    offy: offset.1.to_owned(),
                };
                vec.push(client)
            }
        }
        somei32 = 1;
        somei32 // Not sure what this helps for, but it's needed...
    }
    unsafe { user32::EnumWindows(Some(callback), 0i64) };
    let lock = CLIENTS.clone();
    let mut vec = lock.lock().unwrap();
    let clients = vec.clone();
    clients // Return the Vec out here!
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

fn get_name(pid: u32) -> String {
    // Reads a memory location and returns the account name.
    let address = get_base_addr(pid);
    let offset = u32::from_str_radix(CONF["base"]["offset"].trim_right(), 16).ok().unwrap();
    let pointer = read_memory(pid, address + offset); // get a known pointer to the account name
    let text = get_text(pid, pointer);
    text
}

fn read_memory(pid: u32, address: u32) -> u32 {
    // Kind of "generic" memory read function, though it only reads a u32 at a specified location.
    const BUF_LEN: u64 = 1;
    let mut buffer = [0u32; 3];
    let mut bytes_read = 0u64;
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
    let mut bytes_read = 0u64;
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

fn get_text(pid: u32, address: u32) -> String {
    // Similar to `read_memory`, though this reads a section in memory where the account name lies.
    const BUF_LEN: u64 = 16;
    let mut buffer = [0u8; BUF_LEN as usize];
    let mut bytes_read = 0u64;
    let h_process = unsafe { kernel32::OpenProcess(0x1F0FFF, 0, pid) };
    unsafe {
        kernel32::ReadProcessMemory(h_process,
                                    address as winapi::minwindef::LPCVOID,
                                    buffer.as_mut_ptr() as winapi::minwindef::LPVOID,
                                    1u64 * BUF_LEN,
                                    bytes_read as *mut u64);
    };
    unsafe { kernel32::CloseHandle(h_process) };
    let name = std::str::from_utf8(&buffer[..]).unwrap().to_owned();
    name.split_terminator('\0').next().unwrap().to_string() // This is really not pretty.
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
