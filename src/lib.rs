use std::env::VarError;
use std::fs;
use toml_edit::{Document, value};
use std::env;

mod command;
mod commands_registry;
mod help_page;
mod install;
mod read_cfg;
use crate::command::Command;
use crate::help_page::BONNIE_HELP_PAGE;
use crate::install::{
    download_package, get_dependencies_and_dev_dependencies, get_latest_version,
    get_related_dependencies, get_tarball_download_link_and_name,
};
use crate::read_cfg::{
    get_commands_registry_from_cfg, parse_cfg, parse_dependencies,
};

pub const DEFAULT_BONNIE_CFG_PATH: &str = "./bonnie.toml";
// Performs most program logic with manipulable arguments for easier testing
// This only calls component functions that propagate pre-formed errors, so we can safely use `?`
// This function does not run the final command because that would produce side effects outside the testing environment
pub fn get_command_from_cfg_and_args(
    cfg_string: String,
    prog_args: Vec<String>,
) -> Result<String, String> {
    let cfg = parse_cfg(cfg_string)?;
    let registry = get_commands_registry_from_cfg(&cfg);

    // Extract the command the user wants to run and the arguments they're providing to it
    // When getting the command the user wants to run, they may not have provided one, so we handle that
    let cmd = &prog_args.get(1); // The command the user wants to run
    let cmd = match cmd {
        Some(cmd) => cmd,
        None => return Err(String::from("You must provide a command to run.")),
    };
    let args = &prog_args[2..]; // Any arguments to that command the user has provided

    let command = registry.get(cmd)?;
    let command_with_args = command.insert_args(&args.to_vec())?;

    Ok(command_with_args)
}

pub async fn install_dependencie_from_toml(value: String) {
    let dep=parse_dependencies(value).unwrap();
    for (package, version) in dep.dependencies {
        let version = version.replace("~", "");
        let version = version.replace("^", "");
        println!("getting packages...");
        let mut dep = get_dependencies_and_dev_dependencies(&package, &version)
            .await
            .unwrap();
        for (k, v) in dep.clone() {
            let v = v.replace("~", "");
            let v = v.replace("^", "");
            let a = get_related_dependencies(&k, &v).await.unwrap();
            println!("{:?}", a);
            for (key, value) in a {
                dep.insert(key, value);
            }
        }
        for (k, v) in dep.clone() {
            println!("downloading dependency {} ...", k);
            let v = v.replace("~", "");
            let v = v.replace("^", "");
            let link = get_tarball_download_link_and_name(&k, &v).await;
            match link {
                Ok(link) => {
                    download_package(link).await.unwrap();
                }
                Err(err) => {
                    eprintln!("{}", err)
                }
            }
        }
        println!("downloading dependency {} ...", package);
        let link = get_tarball_download_link_and_name(&package, &version).await;
        match link {
            Ok(link) => {
                download_package(link).await.unwrap();
            }
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
   
}

pub async fn install_dependencie_from_arg(args: &[std::string::String]) {
    for dependency in args {
        let (package, version) = get_latest_version(dependency).await.unwrap();
        let version = version.replace("~", "");
        let version = version.replace("^", "");
        println!("getting packages...");
        let mut dep = get_dependencies_and_dev_dependencies(package, &version)
            .await
            .unwrap();
            println!("{:?}", dep);
        for (k, v) in dep.clone() {
            let v = v.replace("~", "");
            let v = v.replace("^", "");
            let a = get_related_dependencies(&k, &v).await.unwrap();
            println!("{:?}", a);
            for (key, value) in a {
                dep.insert(key, value);
            }
        }
        for (k, v) in dep.clone() {
            println!("downloading dependency {} ...", k);
            let v = v.replace("~", "");
            let v = v.replace("^", "");
            let link = get_tarball_download_link_and_name(&k, &v).await;
            match link {
                Ok(link) => {
                    download_package(link).await.unwrap();
                }
                Err(err) => {
                    eprintln!("{}", err)
                }
            }
        }
        println!("downloading dependency {} ...", package);
        let link = get_tarball_download_link_and_name(&package, &version).await;
        match link {
            Ok(link) => {
                download_package(link).await.unwrap();
                //write to bonie toml
                let cfg_path = get_cfg_path(env::var("BONNIE_CONF"));
                let contents = fs::read_to_string(&cfg_path)
                .expect("Something went wrong reading the file");
                let mut doc = contents.parse::<Document>().expect("invalid toml document");
                doc["dependencies"][&package]=value(version);
                doc["dependencies"].as_inline_table_mut().map(|t| t.fmt());
                fs::write(&cfg_path, doc.to_string()).unwrap();
            }
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
}

// Extracts the config from the TOML file at the given path
pub fn get_cfg(path: &str) -> Result<String, String> {
    let cfg_string = fs::read_to_string(path);
    match cfg_string {
		Ok(cfg_string) => Ok(cfg_string),
		Err(_) => Err(String::from("Error reading bonnie.toml, make sure the file is present in this directory and you have the permissions to read it."))
	}
}

// Gets the path to the config file based on given environment variables
// TODO if non-unicode variable given, print a warning to explain it (right now this behaviour needs to be documented)
pub fn get_cfg_path(env_var: Result<String, VarError>) -> String {
    let default_cfg_path = DEFAULT_BONNIE_CFG_PATH.to_string();

    env_var.unwrap_or(default_cfg_path)
}

// Runs a command (abstracted here to keep the call-only pattern in `main`)
pub fn run_cmd(cmd: String) -> Result<(), String> {
    Command::run(&cmd)?;

    Ok(())
}

// Functions for reserved commands
pub fn init() -> Result<(), String> {
    // Check if there's already a config file in this directory
    if fs::metadata("./bonnie.toml").is_ok() {
        Err(String::from("A Bonnie configuration file already exists in this directory. If you want to create a new one, please delete the old one first."))
    } else {
        // Create a new `bonnie.toml` file
        let output = fs::write(
            "./bonnie.toml",
            "[scripts]
start = \"echo \\\"No start script yet.\\\"\"
",
        );

        match output {
    		Ok(_) => Ok(()),
    		Err(_) => Err(String::from("Error creating new bonnie.toml, make sure you have the permissions to write to this directory."))
    	}
    }
}
pub fn help() {
    println!("{}", BONNIE_HELP_PAGE);
}
