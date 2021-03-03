use std::{error::Error, fs, path::Path, process::exit};
use pnet::datalink;
use reqwest::{self, Client};
use serde_json::json;
use serde::{Deserialize, Serialize};
use clap::{Arg, App};
use tokio::time::{sleep, Duration};
use chrono::prelude::*;

#[derive(Serialize, Deserialize)]
struct Link {
    status_code: i32,
    msg: String,
}

#[tokio::main]
async fn main() {
    let matches = App::new("Find NAS Client")
        .version("0.1.0")
        .author("LI Rui <lr_cn@outlook.com>")
        .about("This program will report all IP addresses of this machine per two minutes if running without any argument.\r\nFind more information on https://find-nas.lirui.tech.")
        .arg(Arg::new("link")
            .short('l')
            .long("link")
            .about("Get short link for current machine"))
        .arg(Arg::new("once")
            .short('o')
            .long("once")
            .about("Only report IP once."))    
        .get_matches();
    
    match matches.occurrences_of("link") {
        0 => (),
        _ => {
            let link = get_link().await;
            match link {
                Some(link) => {
                    println!("Link for this machine: {}", link);
                    exit(0);
                },
                None => {
                    println!("Failed to get link, please ensure this machine has reported IP.");
                    exit(-1);
                }
            }
        }
    }

    match matches.occurrences_of("once") {
        0 => (),
        _ => {
            let local = Local::now();
            let result = report_ip().await;
            match result {
                Ok(_) => {
                    println!("[*] Successfully reported IP. - {}", local.to_string());
                    exit(0);
                },
                Err(_) => {
                    println!("[!] Failed to report IP. - {}", local.to_string());
                    exit(-1);
                }
            }
        }
    }

    loop {
        let local = Local::now();
        let result = report_ip().await;
        match result {
            Ok(_) => {
                println!("[*] Successfully reported IP. - {}", local.to_string());
            },
            Err(_) => {
                println!("[!] Failed to report IP. - {}", local.to_string());
            }
        }
        sleep(Duration::from_secs(120)).await;
    }
}

async fn get_link() -> Option<String> {
    let hwid = match get_hwid() {
        Ok(s) => s,
        Err(_) => return None
    };
    let remote = format!("https://find-nas.lirui.tech/v1/link/{}", hwid);
    let response = Client::new()
        .get(&remote)
        .send()
        .await;
    let response = match response {
        Ok(r) => r,
        Err(_) => return None
    };
    if response.status().is_success() {
        let response = response.text().await.unwrap();
        let link: Link = match serde_json::from_str(&response) {
            Ok(r) => r,
            Err(_) => return None
        };
        return Some(link.msg);
    }
    None
}

async fn report_ip() -> Result<bool, Box<dyn Error>> {
    let remote = "https://find-nas.lirui.tech/v1/ip";
    let mut hwid = match get_hwid() {
        Ok(s) => s,
        Err(e) => return Err(e.to_string().into())
    };
    hwid = hwid.trim().into();
    let ips = get_ip();
    let body = json!({
        "hwid": hwid,
        "ip": ips
    });
    let response = Client::new()
        .post(remote)
        .json(&body)
        .send().await?;
    if response.status().is_success() {
        return Ok(true);
    }
    Err("Failed to report IP.".into())
} 

#[cfg(target_os = "linux")]
fn get_hwid() -> Result<String, Box<dyn Error>> {
    if Path::new("/var/lib/dbus/machine-id").exists() {
        let hwid = fs::read_to_string("/var/lib/dbus/machine-id").unwrap();
        return Ok(hwid);
    }
    if Path::new("/etc/machine-id").exists() {
        let hwid = fs::read_to_string("/etc/machine-id").unwrap();
        return Ok(hwid);
    }
    Err("Failed to get HWID.".into())
}

fn get_ip() -> Vec<String> {
    let mut ips: Vec<String> = Vec::new();
    for iface in datalink::interfaces() {
        for ip in iface.ips {
            ips.push(ip.to_string());
        }
    }
    return ips;
}