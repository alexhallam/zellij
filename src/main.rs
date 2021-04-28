#[cfg(test)]
mod tests;

mod cli;
mod client;
mod common;
mod server;

use client::{boundaries, layout, panes, start_client, tab};
use common::{command_is_executing, errors, os_input_output, pty_bus, screen, utils, wasm_vm};
use directories_next::ProjectDirs;
use server::{start_server, ServerInstruction};

use structopt::StructOpt;

use crate::cli::CliArgs;
use crate::command_is_executing::CommandIsExecuting;
use crate::os_input_output::{get_client_os_input, get_server_os_input, ClientOsApi, ServerOsApi};
use crate::utils::{
    consts::{ZELLIJ_TMP_DIR, ZELLIJ_TMP_LOG_DIR},
    logging::*,
};

pub fn main() {
    // First run installation of default plugins & layouts
    let project_dirs = ProjectDirs::from("org", "Zellij Contributors", "Zellij").unwrap();
    let data_dir = project_dirs.data_dir();
    let mut assets = asset_map! {
        "assets/layouts/default.yaml" => "layouts/default.yaml",
        "assets/layouts/strider.yaml" => "layouts/strider.yaml",
    };
    assets.extend(asset_map! {
        "assets/plugins/status-bar.wasm" => "plugins/status-bar.wasm",
        "assets/plugins/tab-bar.wasm" => "plugins/tab-bar.wasm",
        "assets/plugins/strider.wasm" => "plugins/strider.wasm",
    });

    for (path, bytes) in assets {
        let path = data_dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        if !path.exists() {
            std::fs::write(path, bytes).expect("Failed to install default assets!");
        }
    }

    let opts = CliArgs::from_args();
    if let Some(split_dir) = opts.split {
        match split_dir {
            'h' => {
                let os_input = get_client_os_input();
                os_input.connect_to_server();
                os_input.send_to_server(ServerInstruction::SplitHorizontally);
            }
            'v' => {
                let os_input = get_client_os_input();
                os_input.connect_to_server();
                os_input.send_to_server(ServerInstruction::SplitVertically);
            }
            _ => {}
        };
    } else if opts.move_focus {
        let os_input = get_client_os_input();
        os_input.connect_to_server();
        os_input.send_to_server(ServerInstruction::MoveFocus);
    } else if let Some(file_to_open) = opts.open_file {
        let os_input = get_client_os_input();
        os_input.connect_to_server();
        os_input.send_to_server(ServerInstruction::OpenFile(file_to_open));
    } else {
        atomic_create_dir(ZELLIJ_TMP_DIR).unwrap();
        atomic_create_dir(ZELLIJ_TMP_LOG_DIR).unwrap();
        let server_os_input = get_server_os_input();
        let os_input = get_client_os_input();
        start(Box::new(os_input), opts, Box::new(server_os_input));
    }
}

/// Start Zellij with the specified [`ClientOsApi`], [`ServerOsApi`] and command-line arguments.
// FIXME this should definitely be modularized and split into different functions.
pub fn start(
    client_os_input: Box<dyn ClientOsApi>,
    opts: CliArgs,
    server_os_input: Box<dyn ServerOsApi>,
) {
    let ipc_thread = start_server(server_os_input, opts.clone());
    start_client(client_os_input, opts);
    drop(ipc_thread.join());
}
