#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate xdg;

use std::io;
use std::io::Read;
use std::process::Command;
use std::process::Stdio;
use std::path::{Path, PathBuf};
use std::fs;
use std::fs::File;
use std::collections::HashMap;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::prelude::*;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Json,
    Xdg(xdg::BaseDirectoriesError),
}
impl From<io::Error> for Error {
    fn from(x: io::Error) -> Self {
        Error::Io(x)
    }
}
//impl From<serde_json::Error> for Error {
//    fn from(x: serde_json::Error) -> Self {
//        Error::Json(x)
//    }
//}
impl From<xdg::BaseDirectoriesError> for Error {
    fn from(x: xdg::BaseDirectoriesError) -> Self {
        Error::Xdg(x)
    }
}

pub type DResult<T> = Result<T, Error>;

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum Orientation {
    Normal,
    Left,
    Inverted,
    Right,
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Geometry {
    pub width: usize,
    pub height: usize,
    pub x_offset: usize,
    pub y_offset: usize,
    pub orientation: Orientation,
    pub is_primary: bool,
}

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Output
{
    pub edid: String,
    pub connection_name : String,
    pub geometry: Option<Geometry>
}

pub type RawXrandr = String;
pub type Profiles = HashMap<String, Profile>;
pub type ConnectedOutputs = HashMap<String, Output>;
pub type OutputsRawXrandr = HashMap<String, RawXrandr>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile
{
    pub outputs: OutputsRawXrandr,
    pub other_outputs: RawXrandr,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    pub autoprofiles: Vec<ConnectedOutputs>,
    pub profiles: Profiles,
}

pub type OutputNames = Vec<String>;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MonitorConfig
{
    pub display_name : String,
    pub mode : String,
    pub position : String,
    pub orientation : Orientation,
    pub primary : bool
}

pub struct ConfigAndXrandr
{
    pub connected_outputs: ConnectedOutputs,
    pub output_names: OutputNames,
}

/*
impl Output {
    pub fn raw_edid_to_bytes(&self) -> Vec<u8> {
        let mut gather_bytes = Vec::new();

        for hex_byte in self.edid
            .as_bytes()
            .chunks(2)
            .map(|b| std::str::from_utf8(b).unwrap())
        {
            let byte = u8::from_str_radix(hex_byte, 16).unwrap();

            //println!("{}: {}", hex_byte, byte);
            gather_bytes.push(byte);
        }
        //println!("BYTES: {:?}", gather_bytes);
        gather_bytes
    }

    /*
    pub fn parse_edid(&self) -> EDID {
        edid::parse(&self.raw_edid_to_bytes()).unwrap().1
    }
    */
}
*/

pub fn query_xrandr() -> io::Result<String>
{
    //let mut s = String::new();
    //child.stdout.take().unwrap().read_to_string(&mut s)?;

    let monitor_info_cli = String::from_utf8(
        Command::new("xrandr")
            .args(&["--prop"])
            .output()
            .unwrap()
            .stdout,
    ).unwrap();

    return Ok(monitor_info_cli);
}

pub fn invoke_xrandr(args: &[String]) -> io::Result<()> {
    let mut child = Command::new("xrandr")
        .args(args)
        .spawn()?;

    let ecode = child.wait()?;
    assert!(ecode.success());

    return Ok(());
}

pub fn parse_xrandr(s: &str) -> (ConnectedOutputs, OutputNames)
{
    let mut connected_outputs = HashMap::new();
    let mut output_names = Vec::new();

    let mut lines = s.lines();
    let mut line;

    macro_rules! unwrap_or_break {
        ($e:expr) => (
            if let Some(e) = $e {
                e
            } else {
                break;
            }
        )
    }

    macro_rules! next_line {
        ($line:expr, $lines:expr) => (
            if let Some(line) = $lines.next() {
                $line = line;
            } else {
                break;
            }
        )
    }

    // Parse away "Screen N" headers
    loop {
        line = lines.next().expect("Expected Displays");
        if !line.starts_with("Screen") {
            break;
        }
    }

    // Parse Displays
    loop {
        if line.starts_with(char::is_whitespace) {
            next_line!(line, lines);
            continue;
        }

        let mut splited = line.split_whitespace();
        let output_name = unwrap_or_break!(splited.next());
        let state = unwrap_or_break!(splited.next());

        if state.ends_with("connected") && !output_name.starts_with("VIRTUAL") {
            output_names.push(output_name.to_string());
        }

        if state != "connected" || output_name.starts_with("VIRTUAL") {
            next_line!(line, lines);
            continue;
        }

        let mut next = unwrap_or_break!(splited.next());
        let is_primary = next == "primary";
        if is_primary {
            next = splited.next().unwrap();
        }

        let is_part_of_desktop = next != "(normal";

        let mut geometry = None;
        if is_part_of_desktop {
            // parse current screen config and orientation
            let resolution = next;

            let mut iter = resolution.split(&['x', '+'][..]);
            let width = iter.next().unwrap().parse().unwrap();
            let height = iter.next().unwrap().parse().unwrap();
            let x_offset = iter.next().unwrap().parse().unwrap();
            let y_offset = iter.next().unwrap().parse().unwrap();

            let _unknown_hex_id = splited.next().unwrap();

            let orientation = if _unknown_hex_id.contains(")") {   splited.next().unwrap() } else {_unknown_hex_id};//.replace("(", "").replace(")", "");
            let orientation = match &orientation[..] {
                "normal" => Orientation::Normal,
                "left" => Orientation::Left,
                "inverted" => Orientation::Inverted,
                "right" => Orientation::Right,
                _ => Orientation::Normal,
            };

            geometry = Some(Geometry {
                orientation,
                width,
                height,
                x_offset,
                y_offset,
                is_primary,
            });
        }

        loop {
            next_line!(line, lines);

            assert!(line.starts_with(char::is_whitespace),
                    "no EDID Data found for connected device {}!",
                    output_name);

            if line.trim() == "EDID:" {
                let mut gather = String::new();
                for _ in 0..16 {
                    gather.push_str(lines.next().unwrap().trim());
                }

                assert_eq!(gather.len(), 256 * 2);

                let data = hex::decode(&gather).expect("Decoding failed");

                let (mon_nfo, llll) =  edid::parse(data.as_ref()).unwrap();


                let name = llll.descriptors.iter().find_map(|x| match x
                {
                    edid::Descriptor::ProductName(v) => Some(v),
                    _ => None
                }).unwrap();

                let serial = llll.descriptors.iter().find_map(|x| match x
                {
                    edid::Descriptor::SerialNumber(v) => Some(v),
                    _ => None
                });

                println!("{:?}", serial);

                /*
                let mut monitor_name = gather.split_off(190);
                let mon_name_term = monitor_name.find("0a").unwrap();
                monitor_name.split_off(mon_name_term);
                let decoded_mon_name = hex::decode(monitor_name).expect("Decoding failed");
                let mon_name = match String::from_utf8(decoded_mon_name)
                {
                  Ok(x) => x,
                    Err(e) => "lol".to_string()
                };

                 */

                let out = Output
                {
                    edid: gather,
                    connection_name : output_name.to_string(),
                    geometry,
                };

                //println!("HEX: {}", out.edid);
                //println!("PARSED: {:?}", out.parse_edid());

                connected_outputs.insert(name.to_string(), out);

                break;
            }
        }

    }

    output_names.sort();
    (connected_outputs, output_names)
}


pub fn load_xrandr_layout() -> DResult<ConfigAndXrandr>
{
    //let config_file = {
    //    use std::thread;
    //    let path = path.to_owned();
    //    thread::spawn(move || parse_json(&load_file(&path)?))
    //};

    //let config_file = config_file.join().unwrap()?;

    let (connected_outputs, output_names) = parse_xrandr(&query_xrandr()?);

    Ok(ConfigAndXrandr
    {
        connected_outputs,
        output_names,
    })
}


pub fn save_layout(path : &str)
{
    let mut file = match File::create(path)
    {
        Ok(x) => x,
        Err(e) => { eprintln!("Error opening file: {}", e); return;}
    };

    let curr_layout = match load_xrandr_layout()
    {
        Ok(x) => x,
        Err(e) => {eprintln!("Error reading xrandr config: {:?}", e); return;}
    };

    //let curr_configs : Vec<MonitorConfig>
    let curr_configs : Vec<MonitorConfig> = curr_layout.connected_outputs
    .into_iter()
    .map(|x|
    {
        if x.1.geometry.is_none(){ return None }

        let geo = x.1.geometry.unwrap();

        let mode = if geo.orientation == Orientation::Normal || geo.orientation == Orientation::Inverted
        {
            format!("{}x{}", geo.width, geo.height)
        }
        else
        {
            format!("{}x{}", geo.height, geo.width)
        };


        return Some(MonitorConfig
        {
            display_name : x.0,
            mode,
            orientation : geo.orientation,
            position : format!("{}x{}", geo.x_offset, geo.y_offset),
            primary : geo.is_primary
        })
    }).filter(|x| x.is_some())
      .map(|x| x.unwrap())
      .collect();

    let yaml_file = serde_yaml::to_string(&curr_configs).unwrap();
    file.write_all(yaml_file.as_bytes());
    file.flush();
}

pub fn load_layout(path : &str)
{
    let mut file = match File::open(path)
    {
        Ok(x) => x,
        Err(e) => { eprintln!("Error opening file: {}", e); return;}
    };

    let mut configs : Vec<MonitorConfig> = serde_yaml::from_reader(file).unwrap();

    let curr_layout = match load_xrandr_layout()
    {
        Ok(x) => x,
        Err(e) => {eprintln!("Error reading xrandr config: {:?}", e); return;}
    };

    let ports_to_enable : Vec<String> = curr_layout.connected_outputs.iter().filter(|x| configs.iter().any(|y| y.display_name == *(*x).0)).map(|x| x.1.connection_name.clone()).collect();
    let monitor_to_enable : Vec<(String, Output)> = curr_layout.connected_outputs.into_iter().filter(|x| configs.iter().any(|y| y.display_name == *(*x).0)).map(|x| ( x.0, x.1)).collect();

    println!("{:?}", curr_layout.output_names);
    let ports_to_disable : Vec<String> = curr_layout.output_names.into_iter().filter(|x| !ports_to_enable.iter().any(|y| y == x)).collect();
    println!("{:?}", ports_to_enable);
    println!("{:?}", ports_to_disable);
    println!("{:?}", monitor_to_enable);

    let mut disable_args : Vec<String> = Vec::new();

    for po in ports_to_disable
    {
        disable_args.push("--output".to_string());
        disable_args.push(po);
        disable_args.push("--off".to_string());
    }

    disable_args.iter().for_each(|x| print!("{} ", x));
    println!();

    let xrandr_output = String::from_utf8(
        Command::new("xrandr")
            .args(disable_args)
            .output()
            .unwrap()
            .stdout,
    ).unwrap();

    let mut enable_args : Vec<String> = Vec::new();

    for po in monitor_to_enable
    {
        enable_args.push("--output".to_string());
        enable_args.push(po.1.connection_name.clone());

        enable_args.push("--mode".to_string());
        let config_idx = configs.iter().position(|x| x.display_name == po.0).unwrap();
        let config = configs.remove(config_idx);
        enable_args.push(config.mode);

        enable_args.push("--pos".to_string());
        enable_args.push(config.position);

        enable_args.push("--rotate".to_string());

        let orientation_str = match config.orientation {
            Orientation::Normal => "normal",
            Orientation::Inverted => "inverted",
            Orientation::Left => "left",
            Orientation::Right => "right",
        };

        enable_args.push(orientation_str.to_string());
    }

    enable_args.iter().for_each(|x| print!("{} ", x));
    println!();

    let xrandr_output = String::from_utf8(
        Command::new("xrandr")
            .args(enable_args)
            .output()
            .unwrap()
            .stdout,
    ).unwrap();

    println!("{}", xrandr_output);
}

/*
pub fn parse_json(s: &str) -> DResult<ConfigFile> {
    Ok(serde_json::from_str(s)?)
}

pub fn generate_json(p: &ConfigFile) -> DResult<String> {
    Ok(serde_json::to_string_pretty(p)?)
}

pub fn save_file(path: &Path, contents: &str) -> DResult<()> {
    let file = File::create(path)?;
    let mut buf_writer = BufWriter::new(file);
    buf_writer.write(contents.as_bytes())?;
    buf_writer.get_ref().sync_all()?;

    Ok(())
}

pub fn load_file(path: &Path) -> DResult<String> {
    let file = File::open(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut ret = String::new();
    buf_reader.read_to_string(&mut ret)?;
    Ok(ret)
}

pub fn xdg_config_file() -> DResult<PathBuf> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("quickrandr")?;
    Ok(xdg_dirs.place_config_file("config.json")?)
}



pub fn save_config(path: &Path, config_file: &ConfigFile) -> DResult<()> {
    save_file(path, &generate_json(config_file)?)?;

    Ok(())
}

pub fn cmd_create_empty(path: &Path, debug: bool) {
    if fs::metadata(path).is_err() {
        let empty_database = ConfigFile::default();

        let contents = generate_json(&empty_database).unwrap();

        if debug {
            println!("DEBUG: Write to path {:?}:\n{}", path.display(), contents);
        } else {
            save_file(path, &contents).unwrap();
        }
    }
}

pub fn fingerprint(connected_outputs: &ConnectedOutputs) -> Vec<(&str, &str)> {
    let mut fingerprint = connected_outputs
        .iter()
        .map(|(name, &Output { ref edid, .. })| (name.as_ref(), edid.as_ref()))
        .collect::<Vec<_>>();
    fingerprint.sort_by_key(|x| x.0);
    fingerprint
}

pub fn build_xrandr_args<F>(output_names: &[String], mut f: F) -> Vec<String>
    where F: FnMut(&str) -> Vec<String>
{
        let mut xrandr_command_queue = Vec::<String>::new();

        for output_name in output_names {
            xrandr_command_queue.push("--output".into());
            xrandr_command_queue.push(output_name.clone());
            xrandr_command_queue.extend(f(&output_name));
        }

        xrandr_command_queue
}

pub fn cmd_auto(path: &Path, default_profile: Option<&str>, debug: bool) {
    cmd_create_empty(path, debug);

    let ConfigAndXrandr {
        config_file,
        connected_outputs,
        output_names,
    } = load_config_and_query_xrandr(path).unwrap();

    let current_hardware_fingerprint = fingerprint(&connected_outputs);

    if let Some(target_config) = config_file.autoprofiles
        .iter().find(|x| fingerprint(x) == current_hardware_fingerprint)
    {
        // Found a fingerprint
        if debug {
            println!("FOUND target config: {:?}\n", target_config);
        }

        let xrandr_args = build_xrandr_args(&output_names, |output_name| {
            let mut xrandr_command_queue = Vec::<String>::new();

            if let Some(geometry) = target_config
                .get(output_name)
                .and_then(|x| x.geometry.as_ref())
            {
                xrandr_command_queue.push("--mode".into());
                match geometry.orientation {
                    Orientation::Normal | Orientation::Inverted => {
                        xrandr_command_queue.push(format!("{}x{}", geometry.width, geometry.height));
                    }
                    Orientation::Left | Orientation::Right => {
                        xrandr_command_queue.push(format!("{}x{}", geometry.height, geometry.width));
                    }
                }

                xrandr_command_queue.push("--rotate".into());
                let orientation_str = match geometry.orientation {
                    Orientation::Normal => "normal",
                    Orientation::Inverted => "inverted",
                    Orientation::Left => "left",
                    Orientation::Right => "right",
                };
                xrandr_command_queue.push(orientation_str.into());

                xrandr_command_queue.push("--pos".into());
                xrandr_command_queue.push(format!("{}x{}", geometry.x_offset, geometry.y_offset));

                if geometry.is_primary {
                    xrandr_command_queue.push("--primary".into());
                }
            } else {
                xrandr_command_queue.push("--off".into());
            }

            xrandr_command_queue
        });

        if debug {
            println!("xrandr args: {:?}", xrandr_args);
        } else {
            invoke_xrandr(&xrandr_args).unwrap();
        }
    } else if let Some(default_profile) = default_profile {
        // Start working with defaults
        if debug {
            println!("DEFAULTS {} out of {:?}\n", default_profile, config_file.profiles);
        }
        apply_profile(&output_names, &config_file.profiles, default_profile, debug);
    } else {
        eprintln!("Error: Unknown device config, and no default profile given!")
    }
}

pub fn apply_profile(output_names: &[String], profiles: &Profiles, name: &str, debug: bool) {
    if let Some(profile) = profiles.get(name) {
        let xrandr_args = build_xrandr_args(&output_names, |output_name| {
            if let Some(default) = profile.outputs.get(output_name) {
                default.split_whitespace().map(|x| x.to_string()).collect()
            } else {
                vec![profile.other_outputs.clone()]
            }
        });

        if debug {
            println!("xrandr args: {:?}", xrandr_args);
        } else {
            invoke_xrandr(&xrandr_args).unwrap();
        }
    } else {
        eprintln!("Error: Unknown profile {}!", name);
    }
}

pub fn cmd_save(path: &Path, debug: bool) {
    cmd_create_empty(path, debug);

    let ConfigAndXrandr {
        mut config_file,
        connected_outputs,
        ..
    } = load_config_and_query_xrandr(path).unwrap();

    let mut found = false;
    {
        let current_hardware_fingerprint = fingerprint(&connected_outputs);
        if let Some(target_config) = config_file.autoprofiles
            .iter_mut().find(|x| fingerprint(x) == current_hardware_fingerprint)
        {
            *target_config = connected_outputs.clone();
            found = true;
        }
    }

    if !found {
        config_file.autoprofiles.push(connected_outputs.clone());
    }

    if debug {
        println!("Writing new config file:\n{}", generate_json(&config_file).unwrap());
    } else {
        save_config(path, &config_file).unwrap();
    }
}

pub fn cmd_info(path: &Path, debug: bool) {
    cmd_create_empty(path, debug);

    let ConfigAndXrandr {
        config_file,
        connected_outputs,
        ..
    } = load_config_and_query_xrandr(path).unwrap();

    let print_entry = |x: &ConnectedOutputs| {
        let mut v: Vec<_> = x.iter().collect();
        v.sort_by_key(|x| x.0);

        for x in v {
            print!("   {}:", x.0);
            if let Some(ref x) =  x.1.geometry {

                print!(" {:?}", x.orientation);

                print!(" {}x{}+{}+{}", x.width, x.height, x.x_offset, x.y_offset);

                if x.is_primary {
                    print!(" primary");
                }

            } else {
                print!(" disabled");
            }
            println!();
        }
    };

    println!("Auto Profiles:");
    for x in &config_file.autoprofiles {
        print_entry(x);
        println!();
    }
    println!("Current:");
    print_entry(&connected_outputs);
    println!("Profiles:");
    for x in &config_file.profiles {
        println!("    {}: ", x.0);
        let mut v: Vec<_> = x.1.outputs.iter().collect();
        v.sort_by_key(|x| x.0);
        for x in v {
            println!("       {}: {}", x.0, x.1);
        }
        println!("       <other>: {}", x.1.other_outputs);
    }
}

pub fn cmd_profile(path: &Path, profile: &str, debug: bool) {
    cmd_create_empty(path, debug);

    let ConfigAndXrandr {
        config_file,
        output_names,
        ..
    } = load_config_and_query_xrandr(path).unwrap();

    apply_profile(&output_names, &config_file.profiles, profile, debug);
}
*/