use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use nanors::manager;
use nanors::wallet;
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let main_menu = &["wallet"];
    let wallet_menu = &["new", "load", "show", "back"];

    println!("\n    nanors v0.1.0\n    -------------\n");
    loop {
        let selection = menu_select(main_menu, "sub-menu:");
        match selection {
            "wallet" => run_wallet_menu(wallet_menu).await,
            "exit" => break,
            _ => print_err(&format!("{} unrecognized", selection)),
        }
    }
    Ok(())
}

fn print_err(msg: &str) {
    eprintln!("{}", console::style(msg).red());
}

fn print_show(msg: &str) {
    eprintln!("{}", console::style(msg).yellow());
}

fn menu_select<'a>(menu: &'a [&str], prompt: &str) -> &'a str {
    let idx_selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&menu[..])
        .interact()
        .unwrap();
    menu[idx_selected]
}

async fn run_wallet_menu(menu: &[&str]) {
    loop {
        let selection = menu_select(menu, "wallet options:");
        match selection {
            "new" => wallet_init(false).await,
            "load" => wallet_init(true).await,
            "show" => wallets_show(),
            "back" => break,
            _ => print_err(&format!("unrecognized command {}", selection)),
        }
    }
}

async fn wallet_init(load: bool) {
    let w: Result<wallet::Wallet, Box<dyn std::error::Error>>;
    if load {
        let (name, password) = wallet_prompt(false);
        w = wallet::Wallet::load(&name, &password);
    } else {
        let (name, password) = wallet_prompt(true);
        w = wallet::Wallet::new(&name, &password);
    }
    match w {
        Ok(w) => {
            let manager = manager::Manager::new(w)
                .await
                .expect("manager creation failed");
            run_account_menu(manager)
        }
        Err(e) => print_err(&format!("\n{}\n", e)),
    }
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
        .interact_text()
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

fn run_account_menu(manager: manager::Manager) {
    let account_menu = &["new", "show", "back"];
    loop {
        println!("\n[nano:{}]:\n", manager.curr_wallet_name());
        let selection = menu_select(account_menu, "account options:");
        match selection {
            "new" => break,
            "show" => {
                println!();
                manager.accounts_show().iter().for_each(|s| print_show(&s));
                println!();
            }
            "back" => break,
            _ => print_err(&format!("\n{} unrecognized\n", selection)),
        }
    }
}
