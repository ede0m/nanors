use nanors::nano;
use nanors::wallet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    
    let w = wallet::Wallet::new("gmon");

    //let node = nano::ClientRpc::new("https://mynano.ninja/api/node").expect("error initalizing node client");
    //node.connect().await?;
    
    Ok(())
}