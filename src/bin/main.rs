use dialoguer::{theme::ColorfulTheme, Input, Password, Select};
use nanors::nano;
use nanors::wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let w = wallet::Wallet::new("gmon");
    //let node = nano::ClientRpc::new("https://mynano.ninja/api/node").expect("error initalizing node client");
    //node.connect().await?;

    let main_menu = &["wallet"];

    let wallet_menu = &["new", "load", "show", "back"];

    println!("\n    nanors v0.1.0\n    -------------\n");
    loop {
        let selection = menu_select(main_menu, "select an sub-menu:");
        match selection {
            "wallet" => run_wallet_menu(wallet_menu),
            _ => println!("{} unrecognized", selection),
        }
    }
    Ok(())
}

fn run_wallet_menu(menu: &[&str]) {
    loop {
        let selection = menu_select(menu, "select a wallet option:");
        match selection {
            "new" => wallet_new(),
            "load" => wallet_load(),
            "show" => continue,
            "back" => break,
            _ => println!("{} unrecognized", selection),
        }
    }
}

fn menu_select<'a>(menu: &'a [&str], prompt: &str) -> &'a str {
    let idx_selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&menu[..])
        .interact()
        .unwrap();
    menu[idx_selected]
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
    let mut password = String::new();
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

fn wallet_new() {
    let (name, password) = wallet_prompt(true);
    match wallet::Wallet::new(&name, &password) {
        Ok(w) => println!("success"),
        Err(e) => eprintln!("{}", e),
    };
}

fn wallet_load() {
    let (name, password) = wallet_prompt(false);
    match wallet::Wallet::load(&name, &password) {
        Ok(w) => println!("success"),
        Err(e) => eprintln!("{}", e),
    };
}
