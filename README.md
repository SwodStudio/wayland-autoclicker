# Wayland AutoClicker

A simple and lightweight auto-clicker for Wayland-based Linux environments, written in Rust.  
This project is a Rust adaptation of an original C program by [phonetic112](https://github.com/phonetic112/wl-clicker) under the MIT License.

---

### Usage
Usage: wayland_autoclicker [CLICKS_PER_SECOND] [OPTIONS]

Arguments:
  [CLICKS_PER_SECOND]  Clicks per second [default: 20]

Options:
  -b, --button <BUTTON>  Which mouse button to click (0 for left, 1 for right, 2 for middle) [default: 0]
  -t, --toggle           Toggle the autoclicker on keypress
  -h, --help             Print help
  -V, --version          Print version

---

### Build

Clone the repository:

```bash
git clone https://github.com/SwodStudio/wayland_autoclicker.git
cd wayland_autoclicker
cargo build --release
```

Make sure your user is in the `input` group
```bash
sudo usermod -aG input [user] #If you are not in
```
