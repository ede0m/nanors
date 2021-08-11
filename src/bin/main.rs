use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use nanors::account;
use nanors::manager;
use nanors::rpc;
use nanors::wallet;
use nanors::ws;
use regex::Regex;
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};

const PUBLIC_NANO_HTTP: &str = "https://mynano.ninja/api/node";
const PUBLIC_NANO_WS: &str = "wss://ws.mynano.ninja/";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {   
    let main_menu = &["wallet"];
    let wallet_menu = &["new", "load", "show", "back"];
    println!("\n    nanors v0.1.0\n    -------------\n");
    let rpc = rpc::ClientRpc::new(PUBLIC_NANO_HTTP);
    let ws = ws::ClientWS::new(PUBLIC_NANO_WS).await;
    if rpc.is_err() || ws.is_err() {
        print_err("could not initalize manager dependencies");
    } else {
        let mut m = match manager::Manager::new(Box::new(rpc.unwrap()), Box::new(ws.unwrap())) {
            Ok(m) => m,
            Err(e) => {
                print_err("could not initalize manager");
                return Ok(());
            }
        };
        loop {
            let selection = menu_select(main_menu, "sub-menu:");
            match selection {
                "wallet" => run_wallet_menu(wallet_menu, &mut m).await,
                "exit" => break,
                _ => print_err(&format!("{} unrecognized", selection)),
            }
        }
    }
    Ok(())
}

fn print_err(msg: &str) {
    eprintln!("{}", console::style(msg).red());
}

fn print_show(msg: &str) {
    println!("{}", console::style(msg).yellow());
}

fn menu_select<'a>(menu: &'a [&str], prompt: &str) -> &'a str {
    let idx_selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&menu[..])
        .interact()
        .unwrap();
    menu[idx_selected]
}

async fn run_wallet_menu(menu: &[&str], manager : &'static mut manager::Manager) {
    loop {
        let mut w : Option<Result<wallet::Wallet, Box<dyn std::error::Error>>> = None;
        let selection = menu_select(menu, "wallet options:");
        match selection {
            "new" => {
                w = Some(wallet_init(false).await);
            },
            "load" => {
                w = Some(wallet_init(true).await);
            },
            "show" => wallets_show(),
            "back" => break,
            _ => print_err(&format!("unrecognized command {}", selection)),
        }
        
        if w.is_some() {
            if let Err(e) = w.as_ref().unwrap() {
                print_err(&format!("\n{}\n", e));
                continue;
            }
            let wal = w.unwrap().unwrap();
            if let Err(e) = manager.set_wallet(wal).await {
                print_err(&format!("\n{}\n", e));
                continue;
            } 
            run_account_menu(&mut manager).await;
        }
    }
    
}

async fn wallet_init(load: bool) -> Result<wallet::Wallet, Box<dyn std::error::Error>> {
    let w: Result<wallet::Wallet, Box<dyn std::error::Error>>;
    if load {
        let (name, password) = wallet_prompt(false);
        w = wallet::Wallet::load(&name, &password);
    } else {
        let (name, password) = wallet_prompt(true);
        w = wallet::Wallet::new(&name, &password);
    }
    w
}

fn wallet_prompt(confirm_pass: bool) -> (String, String) {
    let name = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("wallet name:")
        .validate_with(|input: &String| -> Result<(), &str> {
            // todo: check against existing names. should be unique?
            if !input.trim().is_empty() {
                Ok(())
            } else {
                Err("name cannot be empty")
            }
        })
        .interact()
        .unwrap();
    let password;
    if confirm_pass {
        password = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("password")
            .with_confirmation("repeat password", "error: the passwords don't match.")
            .interact()
            .unwrap();
    } else {
        password = Password::with_theme(&ColorfulTheme::default())
            .with_prompt("password")
            .interact()
            .unwrap();
    }
    (name, password)
}

fn wallets_show() {
    if let Ok(file) = OpenOptions::new().read(true).open(wallet::WALLET_FILE_PATH) {
        let reader = BufReader::new(file);
        println!();
        for line in reader.lines() {
            let line = line.unwrap();
            if let Some(name) = line.split("|").next() {
                print_show(&format!("  {}", name));
            }
        }
        println!();
    } else {
        print_err("\nwallet file not found\n");
    }
}

async fn run_account_menu(manager: &mut manager::Manager) {
    let account_menu = &["create", "send", "show", "back"];
    let curr_wallet_name = String::from(manager.curr_wallet_name().unwrap());
    loop {
        println!("\n[nano:{:?}]:\n", curr_wallet_name);
        let selection = menu_select(account_menu, "account options:");
        match selection {
            "create" => {
                println!();
                manager
                    .account_add(&account_prompt(&curr_wallet_name))
                    .unwrap_or_else(|e| print_err(&format!("\n{}\n", e)));
                print_show(&format!(
                    "\ncreated new account for wallet {}",
                    curr_wallet_name
                ));
            }
            "send" => {
                let (from, to, amount) = send_prompt(manager.get_accounts());
                match manager.send(amount, &from, &to).await {
                    Ok(h) => print_show(&format!("\n  success. block hash: {}", h)),
                    Err(e) => print_err(&format!("\n{}\n", e)),
                };
            }
            "show" => {
                println!();
                manager.get_accounts().iter().for_each(|a| {
                    print_show(&format!("  {} : {} : {}", a.index, a.addr, a.balance))
                });
                println!();
            }
            "back" => break,
            _ => print_err(&format!("\n{} unrecognized\n", selection)),
        }
    }
}

fn send_prompt(valid_accounts: Vec<account::AccountInfo>) -> (String, String, u128) {
    let from = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("from account:")
        .validate_with(|input: &String| -> Result<(), &str> {
            if valid_accounts.iter().find(|a| a.addr == *input).is_some() {
                Ok(())
            } else {
                Err("account not in this wallet")
            }
        })
        .interact()
        .unwrap();
    let from_info = valid_accounts.iter().find(|a| a.addr == from).unwrap();
    let re = Regex::new(r"^(nano|xrb)_[13]{1}[13456789abcdefghijkmnopqrstuwxyz]{59}$").unwrap();
    let to = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("to account:")
        .validate_with(|input: &String| -> Result<(), &str> {
            // todo: validate with checksum
            if re.is_match(input) {
                Ok(())
            } else {
                Err("not a valid nano address")
            }
        })
        .interact()
        .unwrap();
    let amount: u128 = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("amount to send:")
        .validate_with(|input: &String| -> Result<(), &str> {
            let amount = match input.parse::<u128>() {
                Ok(a) => a,
                Err(_) => return Err("cannot parse this amount"),
            };
            if amount > from_info.balance {
                return Err("you do not have this much");
            }
            Ok(())
        })
        .interact()
        .unwrap()
        .parse()
        .unwrap();
    (from, to, amount)
}

fn account_prompt(wal_name: &str) -> String {
    let password = Password::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("password for {}", wal_name))
        .interact()
        .unwrap();
    password
}
