use clap::Result;
use clap::{ArgEnum, Parser};
use notify_rust::{Hint, Notification};
use std::error;
use xcb::randr;
use xcb::x;

mod custom_errors;

// TODO: what happens when the min_backlight value from xcb is not 0? -> is this even possible?
// TODO: function documentation
// TODO: test on more systems

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
enum Mode {
    Absolute,
    Relative,
    Step,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Get current backlight value.
    #[clap(short, long)]
    get: bool,

    /// A string to format/ prettify the output of the 'get' option.
    /// The following values can be included: %val - current value, %min - minimal value, %max - maximal value.
    /// '%' needs to be escaped with '%%'.
    /// Example: "%v-%m"
    #[clap(short, long)]
    pretty_format: Option<String>,

    /// Returns max backlight value. Value depends on mode.
    #[clap(long)]
    max: bool,

    /// Returns min backlight value. Value depends on mode.
    #[clap(long)]
    min: bool,

    /// Set backlight to value. The set value depends on the mode. Can be either an absolute value, percentage value or a specific step value.
    #[clap(short, long)]
    set: Option<u32>,

    /// Mode to handle backlight values
    #[clap(arg_enum, default_value_t = Mode::Absolute)]
    mode: Mode,

    /// Number of steps to be used for steps mode. This is required for 'step' mode.
    #[clap(long, required_if_eq("mode", "step"))]
    steps: Option<u32>,

    /// Increases the backlight value. The value depends from the set mode.
    #[clap(short, long)]
    inc: Option<u32>,

    /// Decreases the backlight value. The value depends from the set mode.
    #[clap(short, long)]
    dec: Option<u32>,

    /// If this flag is set, notifications are emitted every time the backlight value is to be changed.
    #[clap(short, long)]
    notifications: bool,

    /// Specifies the title string of the notification. If not set, the title of the notification is the apps's name.
    #[clap(short, long)]
    title: Option<String>,
}

const APPNAME: &str = env!("CARGO_PKG_NAME");

fn main() -> Result<(), String> {
    // catch all errors and print their content
    match handle_backlight() {
        Ok(_) => {
            return Ok(());
        }
        Err(e) => {
            return Err(format!("{}", e));
        }
    }
}

fn handle_backlight() -> Result<(), Box<dyn error::Error>> {
    let args = Args::parse();

    let (conn, output) = init_bus_connection()?;

    let backlight_atom = query_backlight_atom(&conn)?;

    // we assume that the min backlight is always 0
    let (_, max_backlight) = query_min_max_backlight_values(&conn, output, backlight_atom)?;

    match args.mode {
        // ABSOLUTE MODE
        Mode::Absolute => {
            handle_backlight_requests(
                &conn,
                output,
                backlight_atom,
                max_backlight,
                0,
                max_backlight,
                &args,
                &identity,
                &identity,
            )?;
            return Ok(());
        }

        // RELATIVE MODE
        Mode::Relative => {
            handle_backlight_requests(
                &conn,
                output,
                backlight_atom,
                100,
                0,
                max_backlight,
                &args,
                &absolute_to_steps,
                &steps_to_absolute,
            )?;
            return Ok(());
        }

        // STEP MODE
        Mode::Step => {
            // we can unwrap here, since the steps parameter is required and therefore always passed here when we need it
            let valid_steps_range = 0..=max_backlight;
            let steps = args.steps.unwrap();
            if !valid_steps_range.contains(&steps) {
                return Err(Box::new(custom_errors::StepParameterOutOfRangeError {
                    max: max_backlight,
                    step_val: steps,
                }));
            } else {
                handle_backlight_requests(
                    &conn,
                    output,
                    backlight_atom,
                    steps,
                    0,
                    max_backlight,
                    &args,
                    &absolute_to_steps,
                    &steps_to_absolute,
                )?;
                return Ok(());
            }
        }
    }
}

fn init_bus_connection() -> Result<(xcb::Connection, xcb::randr::Output), Box<dyn error::Error>> {
    let (conn, screen_num) = xcb::Connection::connect(None)?;
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let root_window = screen.root();
    let curr_screen_res =
        conn.wait_for_reply(conn.send_request(&randr::GetScreenResourcesCurrent {
            window: root_window,
        }))?;

    if curr_screen_res.outputs().len() > 0 {
        let curr_output = curr_screen_res.outputs()[0];
        return Ok((conn, curr_output));
    } else {
        return Err(Box::new(custom_errors::NoValidScreenResourceError));
    }
}

fn query_backlight_atom(conn: &xcb::Connection) -> Result<xcb::x::Atom, Box<dyn error::Error>> {
    // check for 'Backlight' or 'BACKLIGHT' property
    // we also cannot recover from this error
    let atom_result = conn.wait_for_reply(conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: b"BACKLIGHT",
    }));

    // we want to recover from this error since the atom maybe written in another way

    match atom_result {
        Ok(atom) => {
            return Ok(atom.atom());
        }
        Err(e) => {
            eprintln!("{:?}", e);
            let atom_result = conn.wait_for_reply(conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"Backlight",
            }))?;
            return Ok(atom_result.atom());
        }
    }
}

fn query_min_max_backlight_values(
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
) -> Result<(u32, u32), Box<dyn error::Error>> {
    let valid_val = conn.wait_for_reply(conn.send_request(&randr::QueryOutputProperty {
        output,
        property: backlight_atom,
    }))?;

    // check validity of returned values
    // response type == 1 seems to be the proper response the query output property request
    if valid_val.response_type() == 1 && valid_val.range() && valid_val.valid_values().len() == 2 {
        let min_backlight_value = valid_val.valid_values()[0];
        let max_backlight_value = valid_val.valid_values()[1];
        return Ok((min_backlight_value as u32, max_backlight_value as u32));
    } else {
        return Err(Box::new(custom_errors::NoValidBacklightRangeValuesError));
    }
}

fn query_current_backlight_value(
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
) -> Result<u32, Box<dyn error::Error>> {
    let output_property = conn.wait_for_reply(conn.send_request(&randr::GetOutputProperty {
        output,
        property: backlight_atom,
        r#type: x::ATOM_INTEGER,
        long_offset: 0,
        long_length: 4,
        delete: false,
        pending: false,
    }))?;

    if output_property.response_type() == 1 && output_property.data::<u32>().len() == 1 {
        return Ok(output_property.data::<u32>()[0]);
    } else {
        return Err(Box::new(custom_errors::NoValidCurrenBacklightValueError));
    }
}

fn handle_backlight_requests(
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
    max_val: u32,
    min_val: u32,
    max_backlight: u32,
    args: &Args,
    // these function convert between the values from the absolute mode and the values from the step/relative mode
    to_step: &dyn Fn(u32, u32, u32) -> u32,
    from_step: &dyn Fn(u32, u32, u32) -> u32,
) -> Result<(), Box<dyn error::Error>> {
    let notification_title = if let Some(title) = &args.title {
        &title
    } else {
        APPNAME
    };

    let valid_backlight_range = min_val..=max_val;

    // HANDLE GET COMMAND
    if args.get == true {
        let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom)?;
        let val_step = to_step(max_backlight, max_val, curr_backlight);
        if let Some(pretty_output) = &args.pretty_format {
            let pretty_out = format_output(min_val, max_val, val_step, pretty_output.to_string());
            println!("{}", pretty_out);
            return Ok(());
        } else {
            println!("{}", val_step);
            return Ok(());
        }

    // HANDLE MIN COMMAND
    } else if args.min == true {
        println!("{}", min_val);
        return Ok(());

    // HANDLE MAX COMMAND
    } else if args.max == true {
        println!("{}", max_val);
        return Ok(());

    // HANDLE INC COMMAND
    } else if let Some(inc_val) = args.inc {
        // calculate new to be increased backlight val
        let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom)?;
        let val_step = to_step(max_backlight, max_val, curr_backlight);
        let new_backlight_val = if ((val_step as i32) + (inc_val as i32)) > max_val as i32 {
            max_val
        } else {
            val_step + inc_val
        };

        // set increased backlight val
        if valid_backlight_range.contains(&new_backlight_val) {
            let val = from_step(max_backlight, max_val, new_backlight_val);
            request_backlight_value_change(val, &conn, output, backlight_atom)?;
            send_notification(max_backlight, val, notification_title)?;
            return Ok(());
        } else {
            return Err(Box::new(custom_errors::ValueOutOfRangeError {
                min: min_val,
                max: max_val,
                val: new_backlight_val,
            }));
        }

    // HANDLE DEC COMMAND
    } else if let Some(dec_val) = args.dec {
        // calculate new to be decreased backlight val
        let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom)?;
        let val_step = to_step(max_backlight, max_val, curr_backlight);
        let new_backlight_val = if ((val_step as i32) - (dec_val as i32)) < min_val as i32 {
            min_val
        } else {
            val_step - dec_val
        };

        // set decreased backlight val
        if valid_backlight_range.contains(&new_backlight_val) {
            let val = from_step(max_backlight, max_val, new_backlight_val);
            request_backlight_value_change(val, &conn, output, backlight_atom)?;
            send_notification(max_backlight, val, notification_title)?;
            return Ok(());
        } else {
            return Err(Box::new(custom_errors::ValueOutOfRangeError {
                min: min_val,
                max: max_val,
                val: new_backlight_val,
            }));
        }

    // HANDLE SET COMMAND
    } else if let Some(val_step) = args.set {
        if valid_backlight_range.contains(&val_step) {
            let val = from_step(max_backlight, max_val, val_step);
            request_backlight_value_change(val, &conn, output, backlight_atom)?;
            send_notification(max_backlight, val, notification_title)?;
            return Ok(());
        } else {
            return Err(Box::new(custom_errors::ValueOutOfRangeError {
                min: min_val,
                max: max_val,
                val: val_step,
            }));
        }

    // HANDLE CASE OF NO COMMANDS
    } else {
        // if no arguments/ options are provided, just print out the current absolute value
        let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom)?;
        let val_step = to_step(max_backlight, max_val, curr_backlight);
        if let Some(pretty_output) = &args.pretty_format {
            let pretty_out = format_output(min_val, max_val, val_step, pretty_output.to_string());
            println!("{}", pretty_out);
            return Ok(());
        } else {
            println!("{}", val_step);
            return Ok(());
        }
    }
}

fn request_backlight_value_change(
    val: u32,
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
) -> Result<(), Box<dyn error::Error>> {
    conn.check_request(conn.send_request_checked(&randr::ChangeOutputProperty {
        output,
        property: backlight_atom,
        mode: x::PropMode::Replace,
        data: &[val],
        r#type: x::ATOM_INTEGER,
    }))?;
    return Ok(());
}

fn format_output(min: u32, max: u32, val: u32, format: String) -> String {
    let format = format.replace("%val", &val.to_string());
    let format = format.replace("%min", &min.to_string());
    let format = format.replace("%max", &max.to_string());
    let format = format.replace("%%", "%");
    return format;
}

fn absolute_to_steps(max: u32, step: u32, val: u32) -> u32 {
    let rslt = (val as f32 / max as f32) * step as f32;
    return rslt.round() as u32;
}

fn steps_to_absolute(max: u32, steps: u32, val: u32) -> u32 {
    let rslt = (max as f32 / steps as f32) * val as f32;
    return rslt.round() as u32;
}

/// An identity function used for the absolute mode
/// This function always returns val
fn identity(_max: u32, _steps: u32, val: u32) -> u32 {
    return val;
}

fn send_notification(max_abs: u32, abs_val: u32, title: &str) -> Result<(), Box<dyn error::Error>> {
    // we always need step 100 (relative mode), since this is the representation of percentage we need
    let rel_val = absolute_to_steps(max_abs, 100, abs_val);

    let icon_name = if rel_val > 50 {
        "brightness-high"
    } else {
        "brightness-low"
    };

    Notification::new()
        // set static ID to override previous notification
        .id(765432)
        .summary(title)
        .body(&format!("{}%", rel_val))
        .icon(icon_name)
        .appname(APPNAME)
        .hint(Hint::CustomInt(
            "value".to_string(),
            rel_val.try_into().unwrap(),
        ))
        .hint(Hint::Category("device".to_string()))
        .show()?;
    return Ok(());
}
