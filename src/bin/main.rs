use nanors::nano;
use nanors::wallet;
use dialoguer::{theme::ColorfulTheme, Select};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    //let w = wallet::Wallet::new("gmon");
    //let node = nano::ClientRpc::new("https://mynano.ninja/api/node").expect("error initalizing node client");
    //node.connect().await?;

    let main_menu = &[
        "wallet"
    ];

    let wallet_menu = &[
        "new",
        "load",
        "show",
        "back",
    ];


    loop {
        let selection = dialoguer_select(main_menu, "select an sub-menu:");
        match selection {
            "wallet" => run_wallet_menu(wallet_menu),
            _ => println!("{} unrecognized", selection)
        }
    }
    Ok(())
}


fn dialoguer_select<'a>(menu : &'a[&str], prompt : &str) -> &'a str {
    let idx_selected = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&menu[..])
        .interact()
        .unwrap();
    menu[idx_selected]
}

fn run_wallet_menu(menu : &[&str]) {

    loop {
        let selection = dialoguer_select(menu, "select a wallet option:");
        match selection {
            "new" => continue,
            "load" => continue,
            "show" => continue,
            "back" => break,
            _ => println!("{} unrecognized", selection)
        }
    }
}