# rbacklight

Like xbacklight, only better.

RIIR (Rewrite it in Rust) - because why not?

## What is this?

rbacklight is a rewrite of xbacklight in rust.
It works just like xbacklight by interacting with the xcb library to control the backlight value of laptop screens (unlikely to work for external-monitors).
In addition to xbacklight, rbacklight provides some more features like provide different modes to set the backlight values, format output and provide notifications for backlight changes. rbacklight is for Linux/X11 only.

## How to install
First make sure you have *rust* installed on your system.
Then clone the repo, build and install the binary with:
```sh
git clone https://github.com/procrastimax/rbacklight.git
cd rbacklight
cargo install --path .
```

## Usage
```
USAGE:
    rbacklight [OPTIONS] [MODE]

ARGS:
    <MODE>    Mode to handle backlight values.
               - Absolute Mode: does not change any values, receives and responds with raw
              backlight values (the range of these values can vary between systems)
               - Relative Mode: maps the absolute values on a range 0-100
               - Step Mode: maps the absolute values on a range 0 - steps, here the steps
              parameter can be set arbitrarily
               [default: absolute] [possible values: absolute, relative, step]

OPTIONS:
    -d, --dec <DEC>
            Decreases the backlight value. The value depends from the set mode

    -g, --get
            Get current backlight value

    -h, --help
            Print help information

    -i, --inc <INC>
            Increases the backlight value. The value depends from the set mode

        --max
            Returns max backlight value. Value depends on mode

        --min
            Returns min backlight value. Value depends on mode

    -n, --notifications
            If this flag is set, notifications are emitted every time the backlight value is to be
            changed

    -p, --pretty-format <PRETTY_FORMAT>
            A string to format/ prettify the output of the 'get' option. The following values can be
            included: %val - current value, %min - minimal value, %max - maximal value. '%' needs to
            be escaped with '%%'. Example: "%val-%max"

    -s, --set <SET>
            Set backlight to value. The set value depends on the mode. Can be either an absolute
            value, percentage value or a specific step value

        --steps <STEPS>
            Number of steps to be used for steps mode. This is required for 'step' mode

    -t, --title <TITLE>
            Specifies the title string of the notification. If not set, the title of the
            notification is the apps's name

    -V, --version
            Print version information

```

## Example
Get absolute backlight brightness:
```sh
rbacklight
```

Get relative backlight brightness:
```sh
rbacklight relative
```

Set backlight brightness to 0:
```sh
rbacklight --set 0
```

Get maximum backlight brightness:
```sh
rbacklight --max
```

Set backlight to 50%:
```sh
rbacklight relative -s 50
```

Set backlight to a specific value within an interval (ca. 60%):
```sh
rbacklight steps --steps 10 -s 6
```

Increase backlight in steps of 1/20:
```sh
rbacklight step --steps 20 --inc 1
```

Enable notification and set notification title to "Brightness"
```sh
rbacklight step --steps 20 --inc 1 --notifications --title "Brightness"
```

Get current backlight value in a 20 step range and format the output.
The following command results in this output: `0/19/20`
```sh
rbacklight step --steps 20 --get --pretty-format="%min/%val/%max"
```

## Notifications
Notifications are only fired, when the `-n` parameter is set and the backlight value has been changed.
Notifications are built with [rust-notify](https://docs.rs/crate/notify-rust/latest) in the following way:
```rust
Notification::new()
    // set static ID to override previous notification
    .id(765432)
    .summary(title)
    .body(&format!("{}%", rel_val))
    .icon(icon_name) // brightness-full > 50%, brightness-low <= 50%
    .appname(APPNAME)
    .hint(Hint::CustomInt(
        "value".to_string(),
        rel_val.try_into().unwrap(),
    ))
    .hint(Hint::Category("device".to_string()))
    .show()?;
```
The passed value to the notification is always a relative one (between 0 - 100).
The hint is given to enable the progress bar in the [dunst](https://dunst-project.org/) notification daemon.

**Note:** All notification have a hardcoded ID, this only works for XDG systems.
