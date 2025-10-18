# Wayland AutoClicker

A simple and lightweight auto-clicker for Wayland-based Linux environments, written in Rust.  
This project is a Rust adaptation of an original C program by [phonetic112](https://github.com/phonetic112/wl-clicker) under the MIT License.

---

### Usage
Usage: wayland_autoclicker [OPTIONS] [CLICKS_PER_SECOND]

Arguments:
  [CLICKS_PER_SECOND]  Cps [default: 20]

Options:

  -b, --button <BUTTON>      Click options (0 for left, 1 for right, 2 for middle) [default: 0]

  -t, --toggle               Toggle the autoclicker on keypress

  --startkey <STARTKEY>  Start hotkey (F1-F12) [default: F2]

  --stopkey <STOPKEY>    Stop hotkey (F1-F12) [default: F3]

  -h, --help                 Print help

Example:
```bash
  wayland_autoclicker -t --startkey "F4" --stopkey "F6"
```
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
