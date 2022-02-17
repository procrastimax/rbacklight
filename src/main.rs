use clap::{ArgEnum, Parser};
use xcb::randr;
use xcb::x;

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
    #[clap(long)]
    steps: Option<u32>,
}

fn main() {
    let args = Args::parse();

    let (conn, output) = init_bus_connection();

    let backlight_atom = query_backlight_atom(&conn);

    let (min_backlight, max_backlight) =
        query_min_max_backlight_values(&conn, output, backlight_atom);

    match args.mode {
        // ABSOLUTE MODE
        Mode::Absolute => {
            let valid_backlight_range = min_backlight..=max_backlight;
            if args.get == true {
                let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom);
                println!("{}", curr_backlight);
            } else if args.min == true {
                println!("{}", min_backlight);
            } else if args.max == true {
                println!("{}", max_backlight);
            } else {
                if let Some(val) = args.set {
                    if valid_backlight_range.contains(&val) {
                        request_backlight_value_change(val, &conn, output, backlight_atom);
                    } else {
                        panic!(
                            "Absolute backlight value out of bounds! Min:{} Max:{} Value:{}",
                            min_backlight, max_backlight, val
                        );
                    }
                }
            }
        }

        // RELATIVE MODE
        Mode::Relative => {
            handle_non_absolute(&conn, output, backlight_atom, 100, 0, max_backlight, &args);
        }

        // STEP MODE
        Mode::Step => {
            if let Some(steps) = args.steps {
                if steps > max_backlight {
                    panic!(
                        "Steps parameter ({}) must not be higher than the max backlight value ({})",
                        steps, max_backlight
                    );
                } else if steps == 0 {
                    panic!("The steps parameter should be greater than 0!");
                } else {
                    handle_non_absolute(
                        &conn,
                        output,
                        backlight_atom,
                        steps,
                        0,
                        max_backlight,
                        &args,
                    );
                }
            } else {
                panic!("'--steps' is a required parameter for the step mode!");
            }
        }
    }
}

fn init_bus_connection() -> (xcb::Connection, xcb::randr::Output) {
    // all functions here should panic, we can not really recover from any error happening here
    let (conn, screen_num) = xcb::Connection::connect(None).unwrap();
    let setup = conn.get_setup();
    let screen = setup.roots().nth(screen_num as usize).unwrap();
    let root_window = screen.root();
    let curr_screen_res = conn
        .wait_for_reply(conn.send_request(&randr::GetScreenResourcesCurrent {
            window: root_window,
        }))
        .unwrap();

    if curr_screen_res.outputs().len() > 0 {
        let curr_output = curr_screen_res.outputs()[0];
        return (conn, curr_output);
    } else {
        panic!("Did not receive a valid screen resource");
    }
}

fn query_backlight_atom(conn: &xcb::Connection) -> xcb::x::Atom {
    // check for 'Backlight' or 'BACKLIGHT' property
    // we also cannot recover from this error
    let atom_result = conn.wait_for_reply(conn.send_request(&x::InternAtom {
        only_if_exists: true,
        name: b"BACKLIGHT",
    }));

    match atom_result {
        Ok(atom) => {
            return atom.atom();
        }
        Err(e) => {
            eprintln!("{:?}", e);
            let atom_result = conn.wait_for_reply(conn.send_request(&x::InternAtom {
                only_if_exists: true,
                name: b"Backlight",
            }));

            match atom_result {
                Ok(atom) => {
                    return atom.atom();
                }
                Err(e) => {
                    eprintln!("{e}");
                }
            }

            panic!("Could not find backlight property! You probably cannot change the backlight value!");
        }
    }
}

fn query_min_max_backlight_values(
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
) -> (u32, u32) {
    let valid_val = conn
        .wait_for_reply(conn.send_request(&randr::QueryOutputProperty {
            output,
            property: backlight_atom,
        }))
        .unwrap();

    // check validity of returned values
    // response type == 1 seems to be the proper response the query output property request
    if valid_val.response_type() == 1 && valid_val.range() && valid_val.valid_values().len() == 2 {
        let min_backlight_value = valid_val.valid_values()[0];
        let max_backlight_value = valid_val.valid_values()[1];
        return (min_backlight_value as u32, max_backlight_value as u32);
    } else {
        panic!("Did not receive valid backlight values!");
    }
}

fn query_current_backlight_value(
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
) -> u32 {
    let output_property = conn
        .wait_for_reply(conn.send_request(&randr::GetOutputProperty {
            output,
            property: backlight_atom,
            r#type: x::ATOM_INTEGER,
            long_offset: 0,
            long_length: 4,
            delete: false,
            pending: false,
        }))
        .unwrap();

    if output_property.response_type() == 1 && output_property.data::<u32>().len() == 1 {
        return output_property.data::<u32>()[0];
    } else {
        panic!("Could not request current backlight value");
    }
}

fn request_backlight_value_change(
    val: u32,
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
) {
    conn.check_request(conn.send_request_checked(&randr::ChangeOutputProperty {
        output,
        property: backlight_atom,
        mode: x::PropMode::Replace,
        data: &[val],
        r#type: x::ATOM_INTEGER,
    }))
    .unwrap();
}

fn handle_non_absolute(
    conn: &xcb::Connection,
    output: xcb::randr::Output,
    backlight_atom: xcb::x::Atom,
    max_val: u32,
    min_val: u32,
    max_backlight: u32,
    args: &Args,
) {
    let valid_backlight_range = min_val..=max_val;
    if args.get == true {
        let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom);
        let val_step = absolute_to_steps(max_backlight, max_val, curr_backlight);
        println!("{}", val_step);
    } else if args.min == true {
        println!("{}", min_val);
    } else if args.max == true {
        println!("{}", max_val);
    } else {
        if let Some(val_step) = args.set {
            if valid_backlight_range.contains(&val_step) {
                let val = steps_to_absolute(max_backlight, max_val, val_step);
                request_backlight_value_change(val, &conn, output, backlight_atom);
            } else {
                panic!(
                    "Backlight value out of bounds! Min:{} Max:{} Value:{}",
                    min_val, max_val, val_step
                );
            }
        }
    }
}

fn absolute_to_steps(max: u32, step: u32, val: u32) -> u32 {
    let rslt = (val as f32 / max as f32) * step as f32;
    return rslt.round() as u32;
}

fn steps_to_absolute(max: u32, steps: u32, val: u32) -> u32 {
    let rslt = (max as f32 / steps as f32) * val as f32;
    return rslt.round() as u32;
}
