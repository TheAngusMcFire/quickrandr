extern crate quickrandr;
extern crate clap;

use clap::{Arg, App, ArgGroup};

fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .group(ArgGroup::with_name("main-options")
            .args(&["save", "load", "auto"])
            .required(true)
        )
        .arg(Arg::with_name("auto")
            .short("a")
            .long("auto")
            .help("Automatically configures the displays according to the config file.")
        )
        /*
                    .arg(Arg::with_name("default-profile")
                        .short("d")
                        .long("default-profile")
                        .value_name("PROFILE")
                        .help("Selects a profile to apply in case --auto does not recognize the current system config.")
                        .takes_value(true)
                        .requires("auto")
                    )
                    .arg(Arg::with_name("profile")
                        .short("p")
                        .long("profile")
                        .value_name("PROFILE")
                        .help("Applies the given profile.")
                        .takes_value(true)
                    )
                    */

        .arg(Arg::with_name("save")
            .short("s")
            .long("save")
            .help("Generates a config file from the current config")
            .value_name("CONFIG_FILE")
            .takes_value(true)
        ).arg(Arg::with_name("load")
        .short("l")
        .long("load")
        .help("Loads a display configuration from the provided file")
        .value_name("CONFIG_FILE")
        .takes_value(true)
    ).get_matches();

    if matches.is_present("save")
    {
        let config_file =  matches.value_of("save").unwrap();
        quickrandr::save_layout(config_file);
        println!("{:?}", config_file);
    }

    if matches.is_present("load")
    {
        let config_file =  matches.value_of("load").unwrap();
        quickrandr::load_layout(config_file);
        println!("{:?}", config_file);
    }

    /*
    let debug = matches.is_present("debug");
    let config_path = if let Some(p) = matches.value_of_os("config") {
        p.into()
    } else {
        quickrandr::xdg_config_file().unwrap()
    };

    if matches.is_present("auto") {
        let default_profile = matches.value_of("default-profile");

        quickrandr::cmd_auto(&config_path, default_profile, debug);
        return;
    }
    if matches.is_present("create-empty") {
        quickrandr::cmd_create_empty(&config_path, debug);
        return;
    }
    if matches.is_present("info") {
        quickrandr::cmd_info(&config_path, debug);
        return;
    }
    if matches.is_present("save") {
        quickrandr::cmd_save(&config_path, debug);
        return;
    }
    if let Some(profile) = matches.value_of("profile") {
        quickrandr::cmd_profile(&config_path, profile, debug);
        return;
    }
     */
}
