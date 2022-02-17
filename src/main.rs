use clap::Parser;
use xcb::randr;
use xcb::x;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Get current backlight value.
    #[clap(short, long)]
    get: bool,

    /// Returns absolute max backlight value.
    #[clap(long)]
    max: bool,

    /// Returns absolute min backlight value.
    #[clap(long)]
    min: bool,

    /// Set backlight to value.
    #[clap(short, long)]
    set: Option<u32>,

}

fn main() {
    let args = Args::parse();

    let (conn, output) = init_bus_connection();

    let backlight_atom = query_backlight_atom(&conn);

    let (min_backlight, max_backlight) =
        query_min_max_backlight_values(&conn, output, backlight_atom);

    if args.get == true {
        let curr_backlight = query_current_backlight_value(&conn, output, backlight_atom);
        println!("{}", curr_backlight);
    } else if args.min == true {
        println!("{}", min_backlight);
    } else if args.max == true {
        println!("{}", max_backlight);
    } else {
        if let Some(val) = args.set {
            if val >= min_backlight && max_backlight >= val {
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
