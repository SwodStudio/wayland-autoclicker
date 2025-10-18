// 

use clap::Parser;
use std::{
    fs::File,
    io::{self},
    os::unix::io::AsRawFd,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
    process,
};
use wayland_client::{
    protocol::{wl_pointer, wl_registry},
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1, zwlr_virtual_pointer_v1,
};

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const START_KEY: u16 = 60; // F2
const TERM_KEY: u16 = 61; // F3

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Cps
    #[arg(default_value_t = 10)]
    clicks_per_second: u32,

    /// Click options (0 for left, 1 for right, 2 for middle)
    #[arg(short, long, default_value_t = 0)]
    button: u32,

    /// Toggle the autoclicker on keypress
    #[arg(short, long)]
    toggle: bool,
}

struct State {
    pointer_manager: Option<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1>,
    virtual_pointer: Option<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1>,
    kbd_fds: Vec<File>,
    key_pressed: bool,
    click_interval: Duration,
    button_type: u32,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if interface == "zwlr_virtual_pointer_manager_v1" {
                state.pointer_manager = Some(registry.bind(name, version, qh, ()));
            }
        }
    }
}

impl Dispatch<zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
        _: zwlr_virtual_pointer_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
        _: zwlr_virtual_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

fn get_keyboard_devices() -> io::Result<Vec<String>> {
    let mut devices = Vec::new();
    let content = std::fs::read_to_string("/proc/bus/input/devices")?;
    let mut current_device = None;
    let mut is_keyboard = false;

    for line in content.lines() {
        if line.starts_with("I:") {
            if let Some(dev) = current_device.take() {
                if is_keyboard {
                    devices.push(dev);
                }
            }
            is_keyboard = false;
            current_device = None;
        }

        if line.contains("Handlers=") {
            if line.contains("kbd") && line.contains("sysrq") {
                is_keyboard = true;
            }
        }

        if line.starts_with("H:") {
            if let Some(event_part) = line.split_whitespace().find(|s| s.starts_with("event")) {
                current_device = Some(format!("/dev/input/{}", event_part));
            }
        }
    }
    if let Some(dev) = current_device.take() {
        if is_keyboard {
            devices.push(dev);
        }
    }

    Ok(devices)
}

fn get_keyboard_input(fd: &File) -> i32 {
    let mut ev: libc::input_event = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::input_event>();
    let n = unsafe { libc::read(fd.as_raw_fd(), &mut ev as *mut _ as *mut libc::c_void, size) };

    if n == -1 {
        let err = io::Error::last_os_error();
        if err.kind() != io::ErrorKind::WouldBlock {
            perror("read");
        }
        return -1;
    }

    if n as usize == size && ev.type_ == 1 && ev.code == START_KEY {
        return ev.value;
    } else if ev.code == TERM_KEY {
        println!("Exiting...");
        process::exit(0);
    }

    -1
}

fn send_click(state: &State, conn: &Connection) {
    let pointer = state.virtual_pointer.as_ref().unwrap();
    let now = Instant::now();
    let ms = now.elapsed().as_millis() as u32;

    let button = match state.button_type {
        1 => BTN_RIGHT,
        2 => BTN_MIDDLE,
        _ => BTN_LEFT,
    };

    pointer.button(ms, button, wl_pointer::ButtonState::Pressed);
    pointer.frame();
    conn.flush().unwrap();

    pointer.button(ms, button, wl_pointer::ButtonState::Released);
    pointer.frame();
    conn.flush().unwrap();
}

fn perror(s: &str) {
    let err = io::Error::last_os_error();
    eprintln!("{}: {}", s, err);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let conn = Connection::connect_to_env()?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    let mut state = State {
        pointer_manager: None,
        virtual_pointer: None,
        kbd_fds: Vec::new(),
        key_pressed: false,
        click_interval: Duration::from_nanos((1e9 / args.clicks_per_second as f64) as u64),
        button_type: args.button,
    };

    event_queue.roundtrip(&mut state)?;

    let pointer_manager = state
        .pointer_manager
        .take()
        .ok_or("Compositor does not support wlr-virtual-pointer")?;
    state.virtual_pointer = Some(pointer_manager.create_virtual_pointer(None, &qh, ()));

    let kbd_devices = get_keyboard_devices()?;
    if kbd_devices.is_empty() {
        return Err("Failed to find any keyboard devices.".into());
    }

    for device_path in kbd_devices {
        println!("Found keyboard: {}", device_path);
        let file = File::open(&device_path)?;
        let fd = file.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL, 0) };
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        state.kbd_fds.push(file);
    }

    if unsafe { libc::prctl(libc::PR_SET_TIMERSLACK, 1) } == -1 {
        perror("prctl");
        return Err("prctl failed".into());
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    signal_hook::flag::register(signal_hook::consts::SIGINT, r)?;

    println!("Ready");
    let mut last_click_time = Instant::now();

    while running.load(Ordering::SeqCst) {
        for fd in &state.kbd_fds {
            let key_state = get_keyboard_input(fd);
            if key_state != -1 {
                if args.toggle && key_state == 1 {
                    state.key_pressed = !state.key_pressed;
                } else if !args.toggle {
                    state.key_pressed = key_state == 1;
                }
            }
        }

        if state.key_pressed {
            if last_click_time.elapsed() >= state.click_interval {
                send_click(&state, &conn);
                last_click_time = Instant::now();
            }
        }

        event_queue.dispatch_pending(&mut state)?;
        std::thread::sleep(Duration::from_nanos(1_000_000)); // 1ms
    }

    println!(" Exiting...");
    Ok(())
}
