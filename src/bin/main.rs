use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use nanors::nano;
use nanors::wallet;
use std::fs::OpenOptions;
use std::io::{prelude::*, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let w = wallet::Wallet::new("gmon");
    //let node = nano::ClientRpc::new("https://mynano.ninja/api/node").expect("error initalizing node client");
    //node.connect().await?;
    
    let main_menu = &["wallet"];
    let wallet_menu = &["new", "load", "show", "back"];

    println!("\n    nanors v0.1.0\n    -------------\n");
    loop {
        let selection = menu_select(main_menu, "sub-menu:");
        match selection {
            "wallet" => run_wallet_menu(wallet_menu),
            "exit" => break,
            _ => print_err(&format!("{} unrecognized", selection)),
        }
    }
    Ok(())
}

fn print_err(msg : &str) {
    eprintln!("{}", console::style(msg).red());
}

fn print_show(msg : &str) {
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

fn run_wallet_menu(menu: &[&str]) {
    loop {
        let selection = menu_select(menu, "wallet options:");
        match selection {
            "new" => wallet_new(),
            "load" => wallet_load(),
            "show" => wallets_show(),
            "back" => break,
            _ => print_err(&format!("unrecognized command {}", selection)),
        }
    }
}

fn wallet_new() {
    let (name, password) = wallet_prompt(true);
    match wallet::Wallet::new(&name, &password) {
        Ok(w) => run_account_menu(w),
        Err(e) => print_err(&format!("\n{}\n", e)),
    };
}

fn wallet_load() {
    let (name, password) = wallet_prompt(false);
    match wallet::Wallet::load(&name, &password) {
        Ok(w) => run_account_menu(w),
        Err(e) => print_err(&format!("\n{}\n", e)),
    };
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


fn run_account_menu(wallet: wallet::Wallet) {
    let account_menu = &["new", "show", "back"];
    loop {
        println!("\n[nano:{}]:\n", wallet.name);
        let selection = menu_select(account_menu, "account options:");
        match selection {
            "new" => break,
            "show" => accounts_show(&wallet.accounts),
            "back" => break,
            _ => print_err(&format!("\n{} unrecognized\n", selection)),
        }
    }
}

fn accounts_show(accounts : &Vec<wallet::Account>) {
    println!();
    accounts.iter().for_each(|a| print_show(&format!("  {} : {}", a.index, a.addr)));
    println!();
}
