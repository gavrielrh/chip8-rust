# chip8-rust
chip8-rust is an implementation of the [CHIP-8 virtual machine](https://en.wikipedia.org/wiki/CHIP-8).

## Installation
As this was intended as a learning project, there is no official crate/lib available, so you'll need to compile from source.

```bash
git clone git@github.com:gavrielrh/chip8-rust.git
cd chip8-rust
cargo build
```

## Usage

```bash
./chip8-rust <path-to-chip8-program>
```

The keypad for *chip8-rust* maps the following keys:

[CHIP-8 Keypad]
|   |   |   |   |
|---|---|---|---|
| 1 | 2 | 3 | C |
| 4 | 5 | 6 | D |
| 7 | 8 | 9 | E |
| A | 0 | B | F |
 

[Keyboard]
|   |   |   |   |
|---|---|---|---|
| 1 | 2 | 3 | 4 |
| Q | W | E | R |
| A | S | D | F |
| Z | X | C | V |
